// Session roster sidebar (FleetView). Lists every resident + recent dormant
// session from `sage.local/sessions/list`, reconciled live by
// `sage.local/sessions/changed` broadcasts.

import type { RosterEntry } from '../acp/types';

interface Props {
  roster: RosterEntry[];
  activeSessionId: string | null;
  onSwitch: (sessionId: string) => void;
  onNewSession: () => void;
}

const ACTIVITY_STYLE: Record<string, { label: string; dot: string }> = {
  working: { label: '运行中', dot: 'bg-amber-400 animate-pulse' },
  idle: { label: '空闲', dot: 'bg-zinc-500' },
  needs_input: { label: '待确认', dot: 'bg-red-400' },
  dormant: { label: '休眠', dot: 'bg-zinc-600' },
  completed: { label: '已完成', dot: 'bg-emerald-500' },
  dead: { label: '异常', dot: 'bg-red-500' },
};

function basename(p: string): string {
  const t = p.replace(/[\\/]+$/, '');
  const i = t.lastIndexOf('/');
  const j = t.lastIndexOf('\\');
  return t.slice(Math.max(i, j) + 1) || t;
}

function relativeTime(unixMs: number): string {
  if (!unixMs) return '';
  const diff = Date.now() - unixMs;
  const m = Math.floor(diff / 60_000);
  if (m < 1) return '刚刚';
  if (m < 60) return `${m} 分钟前`;
  const h = Math.floor(m / 60);
  if (h < 24) return `${h} 小时前`;
  return `${Math.floor(h / 24)} 天前`;
}

function SessionRow({
  entry,
  active,
  onSwitch,
}: {
  entry: RosterEntry;
  active: boolean;
  onSwitch: (id: string) => void;
}) {
  const a = ACTIVITY_STYLE[entry.activity] ?? ACTIVITY_STYLE.idle;
  return (
    <button
      onClick={() => onSwitch(entry.sessionId)}
      className={`block w-full rounded-lg border px-3 py-2 text-left transition-colors ${
        active
          ? 'border-indigo-500/50 bg-indigo-500/10'
          : 'border-transparent hover:border-zinc-800 hover:bg-zinc-900/60'
      }`}
    >
      <div className="flex items-center gap-1.5">
        <span className={`h-1.5 w-1.5 shrink-0 rounded-full ${a.dot}`} />
        <span className="truncate text-[13px] font-medium text-zinc-100">
          {entry.title || basename(entry.cwd) || entry.sessionId.slice(0, 8)}
        </span>
      </div>
      <div className="mt-0.5 flex items-center gap-2 pl-3">
        <span className="truncate font-mono text-[11px] text-zinc-500">
          {entry.cwd}
        </span>
        <span className="ml-auto shrink-0 text-[10px] text-zinc-600">
          {relativeTime(entry.lastChangeUnixMs)}
        </span>
      </div>
      <div className="mt-1 flex items-center gap-2 pl-3">
        {entry.modelId && (
          <span className="truncate font-mono text-[10px] text-zinc-600">
            {entry.modelId}
          </span>
        )}
        <span className="ml-auto shrink-0 rounded bg-zinc-800/80 px-1 py-px text-[10px] text-zinc-400">
          {a.label}
        </span>
      </div>
    </button>
  );
}

export default function Sidebar({ roster, activeSessionId, onSwitch, onNewSession }: Props) {
  return (
    <aside className="flex w-64 shrink-0 flex-col border-r border-zinc-800 bg-zinc-950">
      <div className="flex items-center gap-2 border-b border-zinc-800 px-3 py-2.5">
        <span className="text-[13px] font-semibold text-zinc-200">会话</span>
        <span className="text-[11px] text-zinc-600">{roster.length}</span>
        <button
          onClick={onNewSession}
          title="新建会话"
          className="ml-auto flex h-6 w-6 items-center justify-center rounded-md border border-zinc-700 text-zinc-300 hover:border-zinc-500 hover:text-zinc-100"
        >
          <svg className="h-3.5 w-3.5" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}>
            <path strokeLinecap="round" d="M12 5v14M5 12h14" />
          </svg>
        </button>
      </div>
      <div className="flex-1 space-y-1 overflow-y-auto p-2">
        {roster.length === 0 && (
          <p className="px-2 py-6 text-center text-[12px] text-zinc-600">暂无会话</p>
        )}
        {roster.map((e) => (
          <SessionRow
            key={e.sessionId}
            entry={e}
            active={e.sessionId === activeSessionId}
            onSwitch={onSwitch}
          />
        ))}
      </div>
    </aside>
  );
}
