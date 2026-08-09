// Custom window control buttons for the frameless desktop shell.
// Windows/Linux: min / max-restore / close on the right (Codex style).
// macOS: traffic lights on the left (structural support; not the primary UI).
// Renders nothing in a plain browser (no `window.nexusDesktop`).

import { useDesktop } from '../desktop';

function Icon({ d }: { d: string }) {
  return (
    <svg
      className="h-3.5 w-3.5"
      fill="none"
      viewBox="0 0 24 24"
      stroke="currentColor"
      strokeWidth={1.8}
    >
      <path strokeLinecap="round" strokeLinejoin="round" d={d} />
    </svg>
  );
}

const MINIMIZE_ICON = 'M5 12h14';
const MAXIMIZE_ICON = 'M5 4h14a1 1 0 011 1v14a1 1 0 01-1 1H5a1 1 0 01-1-1V5a1 1 0 011-1z';
const RESTORE_ICON =
  'M8 8V5a1 1 0 011-1h10a1 1 0 011 1v10a1 1 0 01-1 1h-3M4 8h10a1 1 0 011 1v10a1 1 0 01-1 1H4a1 1 0 01-1-1V9a1 1 0 011-1z';
const CLOSE_ICON = 'M6 6l12 12M18 6L6 18';

export default function WindowControls() {
  const { desktop, maximized, isMac } = useDesktop();
  if (!desktop) return null;

  const c = desktop.windowControls;

  // macOS traffic lights.
  if (isMac) {
    const light = (bg: string, label: string, onClick: () => void) => (
      <button
        title={label}
        onClick={onClick}
        className={`app-no-drag h-3 w-3 rounded-full ${bg} opacity-80 hover:opacity-100`}
      />
    );
    return (
      <div className="app-no-drag flex shrink-0 items-center gap-2 pl-1">
        {light('bg-red-400', '关闭', c.close)}
        {light('bg-amber-400', '最小化', c.minimize)}
        {light('bg-emerald-400', '最大化', c.toggleMaximize)}
      </div>
    );
  }

  // Windows / Linux: buttons flush against the right edge, full header height.
  const btn =
    'app-no-drag flex h-full w-[46px] items-center justify-center text-zinc-400 transition-colors';
  return (
    <div className="app-no-drag -my-2.5 -mr-4 flex shrink-0 self-stretch">
      <button title="最小化" onClick={c.minimize} className={`${btn} hover:bg-zinc-800 hover:text-zinc-100`}>
        <Icon d={MINIMIZE_ICON} />
      </button>
      <button
        title={maximized ? '还原' : '最大化'}
        onClick={c.toggleMaximize}
        className={`${btn} hover:bg-zinc-800 hover:text-zinc-100`}
      >
        <Icon d={maximized ? RESTORE_ICON : MAXIMIZE_ICON} />
      </button>
      <button
        title="关闭"
        onClick={c.close}
        className={`${btn} hover:bg-red-600 hover:text-white`}
      >
        <Icon d={CLOSE_ICON} />
      </button>
    </div>
  );
}
