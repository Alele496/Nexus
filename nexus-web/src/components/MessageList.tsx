import { useEffect, useRef } from 'react';

import type { ChatMsg } from '../acp/hooks';
import type { ToolCallStatus } from '../acp/types';
import StreamingText from './StreamingText';

interface Props {
  messages: ChatMsg[];
}

const TOOL_STATUS_STYLE: Record<string, string> = {
  in_progress: 'bg-amber-500/15 text-amber-300 border-amber-500/30',
  running: 'bg-amber-500/15 text-amber-300 border-amber-500/30',
  completed: 'bg-emerald-500/15 text-emerald-300 border-emerald-500/30',
  error: 'bg-red-500/15 text-red-300 border-red-500/30',
  cancelled: 'bg-zinc-500/15 text-zinc-400 border-zinc-500/30',
};

function ToolBadge({ status }: { status: ToolCallStatus }) {
  return (
    <span
      className={`inline-block rounded border px-1.5 py-0.5 text-[11px] font-medium ${TOOL_STATUS_STYLE[status] ?? TOOL_STATUS_STYLE.running}`}
    >
      {status}
    </span>
  );
}

function ToolCard({ name, status }: { name: string; status: ToolCallStatus }) {
  return (
    <div className="my-1 flex items-center gap-2 rounded-lg border border-zinc-800 bg-zinc-900/60 px-3 py-2">
      <svg
        className="h-4 w-4 shrink-0 text-zinc-400"
        fill="none"
        viewBox="0 0 24 24"
        stroke="currentColor"
        strokeWidth={2}
      >
        <path
          strokeLinecap="round"
          strokeLinejoin="round"
          d="M10.34 15.66a5.5 5.5 0 10-1.42-5.82M17.66 8.34a5.5 5.5 0 11-1.42 5.82M12 3v3M12 18v3M3 12h3M18 12h3"
        />
      </svg>
      <span className="truncate font-mono text-[13px] text-zinc-200">{name}</span>
      <ToolBadge status={status} />
    </div>
  );
}

function ThoughtBlock({ text }: { text: string }) {
  return (
    <details className="my-1 rounded-lg border border-zinc-800/80 bg-zinc-900/40">
      <summary className="cursor-pointer select-none px-3 py-1.5 text-[12px] text-zinc-500 hover:text-zinc-300">
        思考过程
      </summary>
      <div className="border-t border-zinc-800/60 px-3 py-2 text-[13px] leading-relaxed text-zinc-400">
        <StreamingText text={text} />
      </div>
    </details>
  );
}

function Bubble({ msg }: { msg: Extract<ChatMsg, { kind: 'text' }> }) {
  const isUser = msg.role === 'user';
  return (
    <div className={`flex ${isUser ? 'justify-end' : 'justify-start'}`}>
      <div
        className={`max-w-[85%] rounded-xl px-4 py-2.5 ${
          isUser
            ? 'bg-indigo-600/90 text-white'
            : 'bg-zinc-900 border border-zinc-800 text-zinc-100'
        }`}
      >
        <StreamingText text={msg.text} streaming={msg.streaming} />
      </div>
    </div>
  );
}

function MessageItem({ msg }: { msg: ChatMsg }) {
  switch (msg.kind) {
    case 'text':
      return <Bubble msg={msg} />;
    case 'thought':
      return <ThoughtBlock text={msg.text} />;
    case 'tool':
      return <ToolCard name={msg.name} status={msg.status} />;
  }
}

export default function MessageList({ messages }: Props) {
  const containerRef = useRef<HTMLDivElement>(null);
  const stickRef = useRef(true);

  useEffect(() => {
    const el = containerRef.current;
    if (el && stickRef.current) {
      el.scrollTop = el.scrollHeight;
    }
  }, [messages]);

  const handleScroll = () => {
    const el = containerRef.current;
    if (!el) return;
    stickRef.current = el.scrollHeight - el.scrollTop - el.clientHeight < 48;
  };

  return (
    <div
      ref={containerRef}
      onScroll={handleScroll}
      className="flex-1 overflow-y-auto px-4 py-4"
    >
      <div className="mx-auto flex max-w-3xl flex-col gap-3">
        {messages.length === 0 && (
          <div className="mt-16 text-center text-zinc-500">
            <p className="text-lg">Nexus Web UI</p>
            <p className="mt-1 text-sm">输入消息开始对话。</p>
          </div>
        )}
        {messages.map((m) => (
          <MessageItem key={m.id} msg={m} />
        ))}
      </div>
    </div>
  );
}
