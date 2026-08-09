// Preload: the only bridge between the page and Node. Exposes a tiny, typed
// surface for window controls and external links. contextIsolation + sandbox
// are on; nothing else from the renderer reaches Node.

import { contextBridge, ipcRenderer } from 'electron';

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
});
