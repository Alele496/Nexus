// Cross-process mailbox viewer (`sage.local/mailbox/list` + `mark_read`).
// The mailbox is a shared file every nexus process reads/writes, so this
// shows messages between sessions — including ones owned by other processes.
// Filter tabs: all messages, the current session's inbox, and its sent mail.

import { useMemo, useState } from 'react';

import type { MailboxMessage } from '../acp/types';

interface Props {
  open: boolean;
  messages: MailboxMessage[];
  labels: Record<string, string>;
  sessionId: string | null;
  onClose: () => void;
  onRefresh: () => void;
  onMarkRead: (messageId: string) => void;
}

type Tab = 'all' | 'inbox' | 'sent';

function displayName(sessionId: string, label?: string): string {
  return label || sessionId.slice(0, 8);
}

function relativeTime(iso: string): string {
  const t = Date.parse(iso);
  if (!t) return '';
  const diff = Date.now() - t;
  const m = Math.floor(diff / 60_000);
  if (m < 1) return '刚刚';
  if (m < 60) return `${m} 分钟前`;
  const h = Math.floor(m / 60);
  if (h < 24) return `${h} 小时前`;
  return `${Math.floor(h / 24)} 天前`;
}

const STATUS_STYLE: Record<string, { label: string; cls: string }> = {
  pending: { label: '未送达', cls: 'bg-amber-500/15 text-amber-300' },
  delivered: { label: '未读', cls: 'bg-amber-500/15 text-amber-300' },
  read: { label: '已读', cls: 'bg-zinc-800 text-zinc-400' },
  archived: { label: '已归档', cls: 'bg-zinc-800/60 text-zinc-500' },
};

function MessageRow({
  message,
  expanded,
  onToggle,
  onMarkRead,
}: {
  message: MailboxMessage;
  expanded: boolean;
  onToggle: () => void;
  onMarkRead: (id: string) => void;
}) {
  const s = STATUS_STYLE[message.status] ?? STATUS_STYLE.delivered;
  const unread = message.status === 'pending' || message.status === 'delivered';
  const from = displayName(message.from.sessionId, message.from.label);
  const to = displayName(message.to.sessionId, message.to.label);
  const urgent = message.priority === 'urgent';

  return (
    <div className="rounded-lg border border-zinc-800 bg-zinc-900/60">
      <button onClick={onToggle} className="block w-full px-3 py-2 text-left">
        <div className="flex items-center gap-2">
          {unread && <span className="h-1.5 w-1.5 shrink-0 rounded-full bg-amber-400" />}
          <span
            className={`truncate text-[12px] ${unread ? 'font-semibold text-zinc-100' : 'font-medium text-zinc-300'}`}
          >
            {message.subject || '(无主题)'}
          </span>
          {urgent && (
            <span className="shrink-0 rounded bg-red-500/15 px-1 py-px text-[9px] text-red-300">
              紧急
            </span>
          )}
          <span className={`ml-auto shrink-0 rounded px-1 py-px text-[9px] ${s.cls}`}>
            {s.label}
          </span>
        </div>
        <div className="mt-0.5 flex items-center gap-2 pl-3.5">
          <span className="truncate font-mono text-[10px] text-zinc-500">
            {from} → {to}
          </span>
          <span className="ml-auto shrink-0 text-[10px] text-zinc-600">
            {relativeTime(message.sentAt)}
          </span>
        </div>
      </button>
      {expanded && (
        <div className="border-t border-zinc-800 px-3 py-2">
          <pre className="max-h-48 overflow-auto font-mono text-[11px] leading-relaxed text-zinc-300 whitespace-pre-wrap break-all">
            {message.body || '(空)'}
          </pre>
          <div className="mt-2 flex items-center gap-2">
            <span className="font-mono text-[10px] text-zinc-600">{message.id}</span>
            {unread && (
              <button
                onClick={() => onMarkRead(message.id)}
                className="ml-auto rounded-md border border-zinc-700 px-2 py-1 text-[10px] text-zinc-300 hover:border-zinc-500 hover:text-zinc-100"
              >
                标为已读
              </button>
            )}
          </div>
        </div>
      )}
    </div>
  );
}

export default function MailboxView({
  open,
  messages,
  labels,
  sessionId,
  onClose,
  onRefresh,
  onMarkRead,
}: Props) {
  const [tab, setTab] = useState<Tab>('inbox');
  const [expandedId, setExpandedId] = useState<string | null>(null);

  const filtered = useMemo(() => {
    if (!sessionId) return tab === 'all' ? messages : [];
    const myLabels = Object.entries(labels)
      .filter(([, sid]) => sid === sessionId)
      .map(([label]) => label);
    switch (tab) {
      case 'inbox':
        return messages.filter(
          (m) => m.to.sessionId === sessionId || myLabels.includes(m.to.sessionId),
        );
      case 'sent':
        return messages.filter((m) => m.from.sessionId === sessionId);
      default:
        return messages;
    }
  }, [tab, messages, labels, sessionId]);

  if (!open) return null;

  const tabCls = (t: Tab) =>
    `rounded-md px-2.5 py-1 text-[11px] ${
      tab === t
        ? 'bg-indigo-500/15 text-indigo-200'
        : 'text-zinc-400 hover:bg-zinc-800 hover:text-zinc-200'
    }`;

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 p-4">
      <div className="flex h-[70vh] w-full max-w-2xl flex-col overflow-hidden rounded-xl border border-zinc-700 bg-zinc-900 shadow-2xl">
        <header className="flex items-center gap-2 border-b border-zinc-800 bg-zinc-950/60 px-4 py-3">
          <span className="text-[13px] font-semibold text-zinc-100">信箱</span>
          <span className="text-[11px] text-zinc-600">{filtered.length}</span>
          <div className="ml-auto flex items-center gap-1">
            {(['inbox', 'sent', 'all'] as Tab[]).map((t) => (
              <button key={t} onClick={() => setTab(t)} className={tabCls(t)}>
                {t === 'inbox' ? '收件箱' : t === 'sent' ? '已发送' : '全部'}
              </button>
            ))}
          </div>
        </header>

        <div className="flex-1 space-y-1.5 overflow-y-auto p-3">
          {filtered.length === 0 && (
            <p className="px-2 py-10 text-center text-[12px] text-zinc-600">暂无消息</p>
          )}
          {filtered.map((m) => (
            <MessageRow
              key={m.id}
              message={m}
              expanded={expandedId === m.id}
              onToggle={() => setExpandedId(expandedId === m.id ? null : m.id)}
              onMarkRead={(id) => void onMarkRead(id)}
            />
          ))}
        </div>

        <footer className="flex items-center gap-2 border-t border-zinc-800 bg-zinc-950/60 p-3">
          <button
            onClick={onRefresh}
            className="rounded-lg border border-zinc-700 px-3 py-1.5 text-[12px] text-zinc-300 hover:border-zinc-500 hover:text-zinc-100"
          >
            刷新
          </button>
          <button
            onClick={onClose}
            className="ml-auto rounded-lg border border-zinc-700 px-3 py-1.5 text-[12px] text-zinc-300 hover:border-zinc-500 hover:text-zinc-100"
          >
            关闭
          </button>
        </footer>
      </div>
    </div>
  );
}
