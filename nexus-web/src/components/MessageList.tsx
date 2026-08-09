import { useEffect, useRef, useState } from 'react';

import type { ChatMsg } from '../acp/hooks';
import type { JsonValue, ToolCallStatus } from '../acp/types';
import Markdown from './Markdown';
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

const MAX_DETAIL_CHARS = 4000;

function ToolBadge({ status }: { status: ToolCallStatus }) {
  return (
    <span
      className={`inline-block rounded border px-1.5 py-0.5 text-[11px] font-medium ${TOOL_STATUS_STYLE[status] ?? TOOL_STATUS_STYLE.running}`}
    >
      {status}
    </span>
  );
}

function formatJson(v: JsonValue): string {
  try {
    return JSON.stringify(v, null, 2);
  } catch {
    return String(v);
  }
}

function CopyButton({ text, className }: { text: string; className?: string }) {
  const [copied, setCopied] = useState(false);
  const copy = async () => {
    try {
      await navigator.clipboard.writeText(text);
      setCopied(true);
      window.setTimeout(() => setCopied(false), 1200);
    } catch {
      // Clipboard unavailable (e.g. non-secure context) — ignore.
    }
  };
  return (
    <button
      onClick={copy}
      title="复制"
      className={`flex items-center gap-1 rounded-md border border-zinc-700/80 px-1.5 py-0.5 text-[10px] text-zinc-400 hover:border-zinc-500 hover:text-zinc-100 ${className ?? ''}`}
    >
      {copied ? (
        <svg className="h-2.5 w-2.5" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2.5}>
          <path strokeLinecap="round" strokeLinejoin="round" d="M5 13l4 4L19 7" />
        </svg>
      ) : (
        <svg className="h-2.5 w-2.5" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}>
          <path
            strokeLinecap="round"
            strokeLinejoin="round"
            d="M8 7V5a2 2 0 012-2h9a2 2 0 012 2v9a2 2 0 01-2 2h-2M6 7h9a2 2 0 012 2v9a2 2 0 01-2 2H6a2 2 0 01-2-2V9a2 2 0 012-2z"
          />
        </svg>
      )}
      {copied ? '已复制' : '复制'}
    </button>
  );
}

function DetailBlock({ label, value }: { label: string; value: JsonValue }) {
  const text = formatJson(value);
  const truncated = text.length > MAX_DETAIL_CHARS;
  return (
    <div>
      <div className="mb-1 flex items-center justify-between">
        <span className="text-[11px] font-medium text-zinc-500">{label}</span>
        <CopyButton text={truncated ? text.slice(0, MAX_DETAIL_CHARS) : text} />
      </div>
      <pre className="max-h-64 overflow-auto rounded-md bg-zinc-950/80 p-2 font-mono text-[11px] leading-relaxed text-zinc-300">
        {truncated ? `${text.slice(0, MAX_DETAIL_CHARS)}\n… (已截断)` : text}
      </pre>
    </div>
  );
}

function ToolCard({
  name,
  status,
  rawInput,
  rawOutput,
}: {
  name: string;
  status: ToolCallStatus;
  rawInput?: JsonValue;
  rawOutput?: JsonValue;
}) {
  const [open, setOpen] = useState(false);
  const hasDetails = rawInput !== undefined || rawOutput !== undefined;
  return (
    <div className="my-1 rounded-lg border border-zinc-800 bg-zinc-900/60">
      <button
        onClick={() => hasDetails && setOpen((v) => !v)}
        className={`flex w-full items-center gap-2 px-3 py-2 text-left ${hasDetails ? 'cursor-pointer hover:bg-zinc-900' : 'cursor-default'}`}
      >
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
        {hasDetails && (
          <svg
            className={`ml-auto h-3.5 w-3.5 shrink-0 text-zinc-500 transition-transform ${open ? 'rotate-90' : ''}`}
            fill="none"
            viewBox="0 0 24 24"
            stroke="currentColor"
            strokeWidth={2}
          >
            <path strokeLinecap="round" strokeLinejoin="round" d="M9 5l7 7-7 7" />
          </svg>
        )}
      </button>
      {open && (
        <div className="space-y-2 border-t border-zinc-800/70 px-3 py-2">
          {rawInput !== undefined && <DetailBlock label="参数" value={rawInput} />}
          {rawOutput !== undefined && <DetailBlock label="结果" value={rawOutput} />}
        </div>
      )}
    </div>
  );
}

function RecapLine({ text, auto }: { text: string; auto: boolean }) {
  return (
    <div className="my-1 flex items-start gap-2 rounded-lg border border-amber-500/20 bg-amber-500/5 px-3 py-2">
      <span className="shrink-0 rounded bg-amber-500/15 px-1.5 py-0.5 text-[10px] font-semibold text-amber-300">
        回览
      </span>
      <p className="min-w-0 flex-1 text-[12.5px] italic leading-relaxed text-amber-200/80">
        {text}
        {auto && <span className="ml-1.5 text-[10px] not-italic text-amber-400/60">（自动）</span>}
      </p>
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
    <div className={`group relative flex ${isUser ? 'justify-end' : 'justify-start'}`}>
      <div
        className={`relative max-w-[85%] rounded-xl px-4 py-2.5 ${
          isUser
            ? 'bg-indigo-600/90 text-white'
            : 'bg-zinc-900 border border-zinc-800 text-zinc-100'
        }`}
      >
        {isUser ? (
          <StreamingText text={msg.text} streaming={msg.streaming} />
        ) : (
          <>
            <Markdown text={msg.text} streaming={msg.streaming} />
            {msg.images.length > 0 && (
              <div className="mt-2 flex flex-col gap-2">
                {msg.images.map((img, i) => (
                  <img
                    key={i}
                    src={img.src}
                    alt=""
                    className="max-w-full rounded-lg border border-zinc-800"
                  />
                ))}
              </div>
            )}
          </>
        )}
        {!msg.streaming && (
          <div className="absolute -top-2 right-2 opacity-0 transition-opacity group-hover:opacity-100">
            <CopyButton
              text={msg.text}
              className="border-zinc-700 bg-zinc-950/90 backdrop-blur"
            />
          </div>
        )}
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
      return (
        <ToolCard
          name={msg.name}
          status={msg.status}
          rawInput={msg.rawInput}
          rawOutput={msg.rawOutput}
        />
      );
    case 'recap':
      return <RecapLine text={msg.text} auto={msg.auto} />;
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
