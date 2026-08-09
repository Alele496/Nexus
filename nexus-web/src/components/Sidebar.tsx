// Session roster sidebar (FleetView). Lists every resident + recent dormant
// session from `sage.local/sessions/list`, reconciled live by
// `sage.local/sessions/changed` broadcasts. Hover a row for rename / fork /
// delete; the search box does full-text search over past sessions via
// `sage.local/session/search` (debounced 300ms).

import { useEffect, useRef, useState } from 'react';

import type { RosterEntry, SessionSearchHit } from '../acp/types';

interface Props {
  roster: RosterEntry[];
  activeSessionId: string | null;
  onSwitch: (sessionId: string) => void;
  onNewSession: () => void;
  onRename: (sessionId: string, title: string) => Promise<void>;
  onDelete: (sessionId: string) => Promise<void>;
  onFork: (sessionId: string) => Promise<void>;
  onSearch: (query: string) => Promise<SessionSearchHit[]>;
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
  onRename,
  onDelete,
  onFork,
}: {
  entry: RosterEntry;
  active: boolean;
  onSwitch: (id: string) => void;
  onRename: (id: string, title: string) => Promise<void>;
  onDelete: (id: string) => Promise<void>;
  onFork: (id: string) => Promise<void>;
}) {
  const a = ACTIVITY_STYLE[entry.activity] ?? ACTIVITY_STYLE.idle;
  const [editing, setEditing] = useState(false);
  const [draft, setDraft] = useState('');
  const [confirming, setConfirming] = useState(false);
  const inputRef = useRef<HTMLInputElement | null>(null);
  // Guards the double-commit that happens when Enter fires then input blur.
  const doneRef = useRef(false);

  useEffect(() => {
    if (editing) inputRef.current?.select();
  }, [editing]);

  const startRename = () => {
    setDraft(entry.title || basename(entry.cwd) || '');
    doneRef.current = false;
    setConfirming(false);
    setEditing(true);
  };

  const commitRename = (cancel = false) => {
    if (doneRef.current) return;
    doneRef.current = true;
    setEditing(false);
    if (cancel) return;
    const t = draft.trim();
    if (t && t !== entry.title) void onRename(entry.sessionId, t);
  };

  const handleKey = (e: React.KeyboardEvent<HTMLInputElement>) => {
    if (e.key === 'Enter') {
      e.preventDefault();
      commitRename();
    } else if (e.key === 'Escape') {
      commitRename(true);
    }
  };

  return (
    <div
      className={`group relative rounded-lg border px-3 py-2 transition-colors ${
        active
          ? 'border-indigo-500/50 bg-indigo-500/10'
          : 'border-transparent hover:border-zinc-800 hover:bg-zinc-900/60'
      }`}
    >
      {confirming ? (
        <div className="flex items-center justify-between gap-1">
          <span className="text-[12px] text-red-300">删除该会话？</span>
          <div className="flex shrink-0 gap-1">
            <button
              onClick={() => {
                setConfirming(false);
                void onDelete(entry.sessionId);
              }}
              className="rounded bg-red-600 px-1.5 py-0.5 text-[11px] font-medium text-white hover:bg-red-500"
            >
              删除
            </button>
            <button
              onClick={() => setConfirming(false)}
              className="rounded border border-zinc-700 px-1.5 py-0.5 text-[11px] text-zinc-300 hover:border-zinc-500"
            >
              取消
            </button>
          </div>
        </div>
      ) : editing ? (
        <input
          ref={inputRef}
          value={draft}
          onChange={(e) => setDraft(e.target.value)}
          onKeyDown={handleKey}
          onBlur={() => commitRename()}
          className="w-full rounded border border-indigo-500/60 bg-zinc-900 px-1.5 py-0.5 text-[13px] text-zinc-100 focus:outline-none"
        />
      ) : (
        <>
          <button onClick={() => onSwitch(entry.sessionId)} className="block w-full text-left">
            <div className="flex items-center gap-1.5">
              <span className={`h-1.5 w-1.5 shrink-0 rounded-full ${a.dot}`} />
              <span className="truncate pr-14 text-[13px] font-medium text-zinc-100">
                {entry.title || basename(entry.cwd) || entry.sessionId.slice(0, 8)}
              </span>
            </div>
            <div className="mt-0.5 flex items-center gap-2 pl-3">
              <span className="truncate font-mono text-[11px] text-zinc-500">{entry.cwd}</span>
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
          <div className="absolute right-1.5 top-1.5 hidden items-center gap-0.5 rounded-md bg-zinc-900/95 p-0.5 group-hover:flex">
            <button
              title="重命名"
              onClick={startRename}
              className="flex h-5 w-5 items-center justify-center rounded text-zinc-400 hover:bg-zinc-700 hover:text-zinc-100"
            >
              <svg className="h-3 w-3" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}>
                <path
                  strokeLinecap="round"
                  strokeLinejoin="round"
                  d="M16.86 4.49l2.65 2.65M7.5 20.5l-3.5.5.5-3.5L15.5 6.5l3 3L7.5 20.5z"
                />
              </svg>
            </button>
            <button
              title="分叉会话"
              onClick={() => void onFork(entry.sessionId)}
              className="flex h-5 w-5 items-center justify-center rounded text-zinc-400 hover:bg-zinc-700 hover:text-zinc-100"
            >
              <svg className="h-3 w-3" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}>
                <path
                  strokeLinecap="round"
                  strokeLinejoin="round"
                  d="M10 6h4a4 4 0 014 4v4M6 6v.01M6 18v.01M6 6a3 3 0 100-6 3 3 0 000 6zM6 18a3 3 0 100-6 3 3 0 000 6zM18 18a3 3 0 100-6 3 3 0 000 6zM6 6v4a4 4 0 004 4h4"
                />
              </svg>
            </button>
            <button
              title="删除会话"
              onClick={() => setConfirming(true)}
              className="flex h-5 w-5 items-center justify-center rounded text-zinc-400 hover:bg-zinc-700 hover:text-red-300"
            >
              <svg className="h-3 w-3" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}>
                <path
                  strokeLinecap="round"
                  strokeLinejoin="round"
                  d="M19 7l-.9 12.1A2 2 0 0116.1 21H7.9a2 2 0 01-2-1.9L5 7m5 4v6m4-6v6M9 7V4a1 1 0 011-1h4a1 1 0 011 1v3M4 7h16"
                />
              </svg>
            </button>
          </div>
        </>
      )}
    </div>
  );
}

function SearchBox({
  onSearch,
  onSwitch,
}: {
  onSearch: (query: string) => Promise<SessionSearchHit[]>;
  onSwitch: (sessionId: string) => void;
}) {
  const [query, setQuery] = useState('');
  const [results, setResults] = useState<SessionSearchHit[]>([]);
  const [searching, setSearching] = useState(false);
  const debounceRef = useRef<number | null>(null);
  // Bumped on every input change and on jump-to, so a stale in-flight search
  // (older query resolving after a newer one, or after the box was cleared)
  // can't overwrite the current results.
  const seqRef = useRef(0);

  useEffect(() => {
    return () => {
      if (debounceRef.current !== null) window.clearTimeout(debounceRef.current);
    };
  }, []);

  const handleChange = (value: string) => {
    setQuery(value);
    const seq = ++seqRef.current;
    if (debounceRef.current !== null) window.clearTimeout(debounceRef.current);
    const trimmed = value.trim();
    if (!trimmed) {
      setResults([]);
      setSearching(false);
      return;
    }
    setSearching(true);
    debounceRef.current = window.setTimeout(async () => {
      const hits = await onSearch(trimmed);
      if (seqRef.current === seq) {
        setResults(hits);
        setSearching(false);
      }
    }, 300);
  };

  const jumpTo = (hit: SessionSearchHit) => {
    seqRef.current++;
    if (debounceRef.current !== null) window.clearTimeout(debounceRef.current);
    onSwitch(hit.sessionId);
    setQuery('');
    setResults([]);
    setSearching(false);
  };

  return (
    <div className="relative border-b border-zinc-800 p-2">
      <div className="relative">
        <svg
          className="pointer-events-none absolute left-2 top-1/2 h-3.5 w-3.5 -translate-y-1/2 text-zinc-600"
          fill="none"
          viewBox="0 0 24 24"
          stroke="currentColor"
          strokeWidth={2}
        >
          <path strokeLinecap="round" strokeLinejoin="round" d="M21 21l-4.35-4.35M17 10a7 7 0 11-14 0 7 7 0 0114 0z" />
        </svg>
        <input
          value={query}
          onChange={(e) => handleChange(e.target.value)}
          placeholder="搜索历史会话…"
          className="w-full rounded-md border border-zinc-800 bg-zinc-900 py-1.5 pl-7 pr-2 text-[12px] text-zinc-200 placeholder-zinc-600 focus:border-indigo-500/60 focus:outline-none"
        />
        {searching && (
          <span className="absolute right-2 top-1/2 h-3 w-3 -translate-y-1/2 animate-spin rounded-full border border-zinc-600 border-t-zinc-300" />
        )}
      </div>
      {results.length > 0 && (
        <div className="absolute left-2 right-2 top-full z-20 mt-1 max-h-64 overflow-y-auto rounded-lg border border-zinc-700 bg-zinc-900 shadow-xl">
          {results.map((hit) => (
            <button
              key={hit.sessionId}
              onClick={() => jumpTo(hit)}
              className="block w-full border-b border-zinc-800 px-3 py-2 text-left last:border-b-0 hover:bg-zinc-800"
            >
              <div className="truncate text-[12px] font-medium text-zinc-100">
                {hit.summary || basename(hit.cwd)}
              </div>
              <div className="truncate font-mono text-[10px] text-zinc-500">{hit.cwd}</div>
              {hit.snippet && (
                <div className="mt-0.5 line-clamp-2 text-[11px] text-zinc-400">{hit.snippet}</div>
              )}
            </button>
          ))}
        </div>
      )}
    </div>
  );
}

export default function Sidebar({
  roster,
  activeSessionId,
  onSwitch,
  onNewSession,
  onRename,
  onDelete,
  onFork,
  onSearch,
}: Props) {
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
      <SearchBox onSearch={onSearch} onSwitch={onSwitch} />
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
            onRename={onRename}
            onDelete={onDelete}
            onFork={onFork}
          />
        ))}
      </div>
    </aside>
  );
}
