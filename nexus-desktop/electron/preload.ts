// Preload: the only bridge between the page and Node. Exposes a tiny, typed
// surface for window controls, external links, native notifications and file
// paths. contextIsolation + sandbox are on; nothing else reaches Node.

import { contextBridge, ipcRenderer, webUtils } from 'electron';

export type DesktopPlatform = 'win32' | 'darwin' | 'linux' | string;

contextBridge.exposeInMainWorld('nexusDesktop', {
  isDesktop: true,
  platform: process.platform as DesktopPlatform,
  windowControls: {
    minimize: () => ipcRenderer.send('window:minimize'),
    toggleMaximize: () => ipcRenderer.send('window:toggle-maximize'),
    close: () => ipcRenderer.send('window:close'),
    isMaximized: () => ipcRenderer.invoke('window:is-maximized') as Promise<boolean>,
    onMaximizedChange: (cb: (maximized: boolean) => void) => {
      const handler = (_event: unknown, maximized: boolean) => cb(maximized);
      ipcRenderer.on('window:maximized-changed', handler);
      return () => {
        ipcRenderer.removeListener('window:maximized-changed', handler);
      };
    },
  },
  openExternal: (url: string) => ipcRenderer.invoke('shell:open-external', url),
  // File.path was removed in Electron 32; webUtils is the supported way to get
  // the real filesystem path of a dropped File in a sandboxed renderer.
  getPathForFile: (file: File) => webUtils.getPathForFile(file),
  notify: (title: string, body: string) =>
    ipcRenderer.invoke('notifications:show', { title, body }),
});
