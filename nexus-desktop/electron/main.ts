// Electron main process: owns the agent server child, the window (frameless,
// Codex-style custom titlebar), the tray, and the IPC surface for the preload.

import {
  app,
  BrowserWindow,
  globalShortcut,
  ipcMain,
  Menu,
  nativeImage,
  Notification,
  shell,
  Tray,
  dialog,
} from 'electron';
import fs from 'node:fs';
import path from 'node:path';
import {
  ensureServer,
  type ServerHandle,
  type ServerOptions,
} from './server';

let mainWindow: BrowserWindow | null = null;
let tray: Tray | null = null;
let server: ServerHandle | null = null;
let isQuitting = false;

/** Resolve the `nexus` binary: packaged resources → env override → debug build. */
function resolveBinaryPath(): string {
  if (process.env.NEXUS_DESKTOP_BIN) return process.env.NEXUS_DESKTOP_BIN;
  if (app.isPackaged) {
    return path.join(
      process.resourcesPath,
      'bin',
      process.platform === 'win32' ? 'nexus.exe' : 'nexus',
    );
  }
  const repoRoot = path.resolve(__dirname, '..', '..', '..');
  const candidate = path.join(repoRoot, 'nexus', 'target', 'debug', 'nexus.exe');
  if (fs.existsSync(candidate)) return candidate;
  return 'nexus';
}

/** Web UI dist dir: packaged resources → repo build output. */
function resolveWebDir(): string | undefined {
  if (app.isPackaged) return path.join(process.resourcesPath, 'web');
  const repoRoot = path.resolve(__dirname, '..', '..', '..');
  const dist = path.join(repoRoot, 'nexus-web', 'dist');
  return fs.existsSync(dist) ? dist : undefined;
}

function argValue(name: string): string | undefined {
  const prefix = `${name}=`;
  const hit = process.argv.find((a) => a.startsWith(prefix));
  return hit?.slice(prefix.length);
}

// ---- window state persistence ---------------------------------------------

interface WindowState {
  width: number;
  height: number;
  x?: number;
  y?: number;
  isMaximized?: boolean;
}

function stateFile(): string {
  return path.join(app.getPath('userData'), 'window-state.json');
}

function loadWindowState(): WindowState {
  try {
    const s = JSON.parse(fs.readFileSync(stateFile(), 'utf8')) as WindowState;
    if (typeof s.width === 'number' && typeof s.height === 'number') return s;
  } catch {
    /* first run */
  }
  return { width: 1280, height: 800 };
}

function saveWindowState(): void {
  const w = mainWindow;
  if (!w) return;
  try {
    fs.writeFileSync(
      stateFile(),
      JSON.stringify({ ...w.getBounds(), isMaximized: w.isMaximized() }),
    );
  } catch {
    /* best effort */
  }
}

// ---- window ---------------------------------------------------------------

function createWindow(): void {
  const state = loadWindowState();
  const url = server?.url;
  if (!url) {
    dialog.showErrorBox('Nexus', '无法启动 agent 服务，应用即将退出。');
    app.quit();
    return;
  }

  const devUrl = argValue('--dev-url');
  const loadTarget = devUrl
    ? `${devUrl.replace(/\/+$/, '')}/?key=${server!.key}`
    : url;

  const preload = process.argv.includes('--no-preload')
    ? undefined
    : path.join(__dirname, 'preload.js');
  mainWindow = new BrowserWindow({
    width: state.width,
    height: state.height,
    ...(state.x !== undefined && state.y !== undefined
      ? { x: state.x, y: state.y }
      : {}),
    minWidth: 880,
    minHeight: 560,
    frame: false, // custom titlebar drawn by the renderer (Codex-style)
    backgroundColor: '#09090b',
    show: false,
    webPreferences: {
      preload,
      contextIsolation: true,
      nodeIntegration: false,
      sandbox: true,
    },
  });

  mainWindow.once('ready-to-show', () => {
    if (state.isMaximized) mainWindow?.maximize();
    mainWindow?.show();
  });

  // Lock the window to the agent server origin; any other http(s) URL opens in
  // the system browser, file:/javascript: etc. are simply denied.
  mainWindow.webContents.setWindowOpenHandler(({ url: target }) => {
    if (/^https?:/.test(target)) void shell.openExternal(target);
    return { action: 'deny' };
  });
  mainWindow.webContents.on('will-navigate', (event, target) => {
    if (!target.startsWith(url.split('?')[0])) {
      event.preventDefault();
      if (/^https?:/.test(target)) void shell.openExternal(target);
    }
  });

  const sendMaximized = () => {
    mainWindow?.webContents.send(
      'window:maximized-changed',
      mainWindow?.isMaximized() ?? false,
    );
  };
  mainWindow.on('maximize', sendMaximized);
  mainWindow.on('unmaximize', sendMaximized);

  // Closing hides to tray unless the app is really quitting.
  mainWindow.on('close', (event) => {
    saveWindowState();
    if (!isQuitting) {
      event.preventDefault();
      mainWindow?.hide();
    }
  });
  mainWindow.on('closed', () => {
    mainWindow = null;
  });
  mainWindow.on('resize', () => saveWindowState());
  mainWindow.on('move', () => saveWindowState());

  void mainWindow.loadURL(loadTarget);
}

function showWindow(): void {
  if (!mainWindow) {
    createWindow();
    return;
  }
  if (mainWindow.isMinimized()) mainWindow.restore();
  mainWindow.show();
  mainWindow.focus();
}

// ---- tray -----------------------------------------------------------------

function createTray(): void {
  const iconPath = path.join(__dirname, '..', '..', 'resources', 'tray.png');
  let icon = nativeImage.createFromPath(iconPath);
  if (icon.isEmpty()) icon = nativeImage.createEmpty();
  tray = new Tray(icon.resize({ width: 16, height: 16 }));
  tray.setToolTip('Nexus');
  tray.setContextMenu(
    Menu.buildFromTemplate([
      { label: '显示 / 隐藏', click: showWindow },
      {
        label: '在浏览器中打开',
        click: () => {
          if (server) void shell.openExternal(server.url);
        },
      },
      { type: 'separator' },
      {
        label: '退出',
        click: () => {
          isQuitting = true;
          app.quit();
        },
      },
    ]),
  );
  tray.on('click', showWindow);
}

// ---- app menu -------------------------------------------------------------

function createMenu(): void {
  const isMac = process.platform === 'darwin';
  const template: Electron.MenuItemConstructorOptions[] = [
    ...(isMac
      ? [{ role: 'appMenu' as const }]
      : [
          {
            label: '文件',
            submenu: [
              { label: '退出', click: () => { isQuitting = true; app.quit(); } },
            ] as Electron.MenuItemConstructorOptions[],
          },
        ]),
    {
      label: '编辑',
      submenu: [
        { role: 'undo', label: '撤销' },
        { role: 'redo', label: '重做' },
        { type: 'separator' },
        { role: 'cut', label: '剪切' },
        { role: 'copy', label: '复制' },
        { role: 'paste', label: '粘贴' },
        { role: 'selectAll', label: '全选' },
      ],
    },
    {
      label: '视图',
      submenu: [
        { role: 'reload', label: '重新加载' },
        { role: 'toggleDevTools', label: '开发者工具' },
        { type: 'separator' },
        { role: 'resetZoom', label: '重置缩放' },
        { role: 'zoomIn', label: '放大' },
        { role: 'zoomOut', label: '缩小' },
      ],
    },
    {
      label: '窗口',
      submenu: [
        { role: 'minimize', label: '最小化' },
        { role: 'close', label: '关闭' },
      ],
    },
  ];
  Menu.setApplicationMenu(Menu.buildFromTemplate(template));
}

// ---- IPC ------------------------------------------------------------------

function registerIpc(): void {
  ipcMain.on('window:minimize', () => mainWindow?.minimize());
  ipcMain.on('window:toggle-maximize', () => {
    const w = mainWindow;
    if (!w) return;
    if (w.isMaximized()) w.unmaximize();
    else w.maximize();
  });
  // Deliberately routes through close() so the close-to-tray behavior applies.
  ipcMain.on('window:close', () => mainWindow?.close());
  ipcMain.handle('window:is-maximized', () => mainWindow?.isMaximized() ?? false);
  ipcMain.handle('shell:open-external', (_event, url: unknown) => {
    if (typeof url === 'string' && /^https?:/.test(url)) {
      return shell.openExternal(url);
    }
    return Promise.resolve();
  });
  ipcMain.handle(
    'notifications:show',
    (_event, payload: unknown) => {
      const { title, body } = (payload ?? {}) as { title?: string; body?: string };
      if (!title) return;
      new Notification({ title, body: body ?? '' }).show();
    },
  );
}

// ---- smoke mode -----------------------------------------------------------

async function runSmoke(): Promise<void> {
  const wc = mainWindow?.webContents;
  if (!wc) {
    console.error('SMOKE: no webContents');
    app.exit(3);
    return;
  }
  const timer = setTimeout(() => {
    console.error('SMOKE: timeout waiting for page load');
    app.exit(4);
  }, 30_000);
  const shotPath = argValue('--shot');
  const consoleErrors: string[] = [];
  wc.on('console-message', (event, levelOrMsg, messageArg) => {
    const msg =
      typeof event === 'object' && event !== null && 'message' in event
        ? (event as { message: string }).message
        : messageArg ?? levelOrMsg;
    consoleErrors.push(String(msg));
  });
  wc.on('did-start-loading', () => {
    wc.executeJavaScript(
      `window.__errs=[];
       window.addEventListener('error', e => window.__errs.push(e.error && e.error.stack ? e.error.stack : String(e.message)));
       window.addEventListener('unhandledrejection', e => window.__errs.push('PROMISE: ' + String(e.reason && e.reason.stack ? e.reason.stack : e.reason)));`,
    ).catch(() => {});
  });
  wc.once('did-finish-load', async () => {
    try {
      // Let React hydrate and render before asserting/screenshotting.
      await new Promise((r) => setTimeout(r, 1500));
      const title = await wc.executeJavaScript('document.title');
      const url = wc.getURL();
      const dom = (await wc.executeJavaScript(`(() => {
        const header = document.querySelector('header');
        const q = (s) => !!document.querySelector(s);
        const nd = window.nexusDesktop;
        return {
          headerDrag: header ? header.className.includes('app-drag') : false,
          minBtn: q('[title="最小化"]'),
          maxBtn: q('[title="最大化"], [title="还原"]'),
          closeBtn: q('[title="关闭"]'),
          mailboxBtn: q('[title="信箱"]'),
          sidebar: q('aside'),
          desktopBridge:
            typeof nd === 'object' &&
            typeof nd?.notify === 'function' &&
            typeof nd?.getPathForFile === 'function' &&
            typeof nd?.openExternal === 'function',
          bodyText: (document.body.innerText || '').slice(0, 200),
          stacks: window.__errs || [],
        };
      })()`)) as Record<string, unknown>;
      const ok =
        url.startsWith(`http://127.0.0.1:${server?.port}/`) &&
        dom.headerDrag === true &&
        dom.minBtn === true &&
        dom.maxBtn === true &&
        dom.closeBtn === true &&
        dom.mailboxBtn === true &&
        dom.sidebar === true &&
        dom.desktopBridge === true;
      const info: Record<string, unknown> = {
        title,
        url,
        ok,
        port: server?.port,
        dom,
        consoleErrors,
      };
      if (shotPath) {
        const image = await wc.capturePage();
        fs.writeFileSync(shotPath, image.toPNG());
        info.screenshot = shotPath;
      }
      console.log(JSON.stringify(info));
      clearTimeout(timer);
      app.exit(ok ? 0 : 2);
    } catch (err) {
      console.error('SMOKE error:', err);
      clearTimeout(timer);
      app.exit(3);
    }
  });
}

// ---- lifecycle ------------------------------------------------------------

async function bootstrap(): Promise<void> {
  const opts: ServerOptions = {
    binaryPath: resolveBinaryPath(),
    webDir: resolveWebDir(),
    onLog: (line) => console.log('[agent]', line),
  };
  try {
    server = await ensureServer(opts);
  } catch (err) {
    console.error('[agent] failed to start server:', err);
    dialog.showErrorBox('Nexus', `无法启动 agent 服务：${String(err)}`);
    app.quit();
    return;
  }
  createMenu();
  registerIpc();
  createTray();
  createWindow();
  // Global toggle for the window regardless of focus. Registered best-effort:
  // the accelerator may already be taken by another app, which is fine.
  try {
    globalShortcut.register('CommandOrControl+Alt+N', () => {
      if (mainWindow?.isVisible()) mainWindow.hide();
      else showWindow();
    });
  } catch {
    /* shortcut unavailable — non-fatal */
  }
  if (process.argv.includes('--smoke')) void runSmoke();
}

const gotLock = app.requestSingleInstanceLock();
if (!gotLock) {
  app.quit();
} else {
  app.on('second-instance', showWindow);
  app.on('before-quit', () => {
    isQuitting = true;
  });
  app.on('will-quit', () => {
    globalShortcut.unregisterAll();
    server?.stop();
  });
  app.on('window-all-closed', () => {
    // Keep the app alive in the tray (except when actually quitting).
    if (isQuitting) app.quit();
  });
  void app.whenReady().then(bootstrap);
}
