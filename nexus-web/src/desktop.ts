// Desktop-shell integration.
//
// The Electron preload exposes `window.nexusDesktop` only inside the desktop
// app (see nexus-desktop/electron/preload.ts). In a plain browser it is absent
// and everything here degrades to no-ops, so the same bundle serves both.

import { useEffect, useState } from 'react';

export interface DesktopWindowControls {
  minimize(): void;
  toggleMaximize(): void;
  close(): void;
  isMaximized(): Promise<boolean>;
  onMaximizedChange(cb: (maximized: boolean) => void): () => void;
}

export interface NexusDesktop {
  isDesktop: boolean;
  platform: string;
  windowControls: DesktopWindowControls;
  openExternal(url: string): Promise<void>;
  /** Real filesystem path of a dropped File (Electron 32+ has no File.path). */
  getPathForFile(file: File): string;
  /** Native OS notification via the main process. */
  notify(title: string, body: string): void;
}

declare global {
  interface Window {
    nexusDesktop?: NexusDesktop;
  }
}

export interface DesktopState {
  desktop: NexusDesktop | undefined;
  maximized: boolean;
  isMac: boolean;
}

/** Tracks the desktop bridge and the live maximized state. */
export function useDesktop(): DesktopState {
  // The preload exposes the bridge before page scripts run, so it is stable
  // for the lifetime of the renderer; read it once.
  const [desktop] = useState<NexusDesktop | undefined>(window.nexusDesktop);
  const [maximized, setMaximized] = useState(false);

  useEffect(() => {
    if (!desktop) return;
    let alive = true;
    void desktop.windowControls.isMaximized().then((v) => {
      if (alive) setMaximized(v);
    });
    const off = desktop.windowControls.onMaximizedChange(setMaximized);
    return () => {
      alive = false;
      off();
    };
  }, [desktop]);

  return { desktop, maximized, isMac: desktop?.platform === 'darwin' };
}
