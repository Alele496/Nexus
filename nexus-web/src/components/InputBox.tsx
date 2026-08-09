import { useRef, useState } from 'react';

import { useDesktop } from '../desktop';

interface Props {
  disabled?: boolean;
  onSend: (text: string) => void;
}

export default function InputBox({ disabled, onSend }: Props) {
  const [value, setValue] = useState('');
  const [dragOver, setDragOver] = useState(false);
  const taRef = useRef<HTMLTextAreaElement>(null);
  const { desktop } = useDesktop();

  const submit = () => {
    const trimmed = value.trim();
    if (!trimmed || disabled) return;
    onSend(trimmed);
    setValue('');
    if (taRef.current) {
      taRef.current.style.height = 'auto';
      taRef.current.focus();
    }
  };

  const handleKeyDown = (e: React.KeyboardEvent<HTMLTextAreaElement>) => {
    if (e.key === 'Enter' && !e.shiftKey && !e.nativeEvent.isComposing) {
      e.preventDefault();
      submit();
    }
  };

  const handleInput = (e: React.ChangeEvent<HTMLTextAreaElement>) => {
    setValue(e.target.value);
    const el = e.target;
    el.style.height = 'auto';
    el.style.height = `${Math.min(el.scrollHeight, 200)}px`;
  };

  // Desktop-only: dropping files inserts their real paths as `@path …` tokens
  // (the CLI's file-reference convention). In a plain browser the drop is
  // consumed so the browser doesn't navigate to the file, but nothing is
  // inserted.
  const handleDrop = (e: React.DragEvent<HTMLDivElement>) => {
    e.preventDefault();
    setDragOver(false);
    if (disabled) return;
    const files = Array.from(e.dataTransfer.files ?? []);
    if (!files.length || !desktop) return;
    const paths = files
      .map((f) => desktop.getPathForFile(f))
      .filter((p): p is string => Boolean(p));
    if (!paths.length) return;

    const ta = taRef.current;
    const start = ta?.selectionStart ?? value.length;
    const end = ta?.selectionEnd ?? value.length;
    const tokens = paths.map((p) => `@${p}`).join(' ');
    const before = value.slice(0, start);
    const after = value.slice(end);
    const lead = before && !/\s$/.test(before) ? ' ' : '';
    const trail = after && !/^\s/.test(after) ? ' ' : '';
    const next = before + lead + tokens + trail + after;
    setValue(next);
    requestAnimationFrame(() => {
      const pos = start + lead.length + tokens.length;
      if (ta) {
        ta.focus();
        ta.setSelectionRange(pos, pos);
        ta.style.height = 'auto';
        ta.style.height = `${Math.min(ta.scrollHeight, 200)}px`;
      }
    });
  };

  return (
    <div className="border-t border-zinc-800 bg-zinc-950/80 px-4 py-3">
      <div
        onDragOver={(e) => {
          e.preventDefault();
          setDragOver(true);
        }}
        onDragLeave={() => setDragOver(false)}
        onDrop={handleDrop}
        className={`mx-auto flex max-w-3xl items-end gap-2 rounded-xl transition ${
          dragOver ? 'outline-2 outline-dashed outline-indigo-500' : ''
        }`}
      >
        <textarea
          ref={taRef}
          value={value}
          disabled={disabled}
          onChange={handleInput}
          onKeyDown={handleKeyDown}
          rows={1}
          placeholder={disabled ? '连接中…' : '输入消息，Enter 发送，Shift+Enter 换行；拖入文件插入 @路径'}
          className="max-h-[200px] flex-1 resize-none rounded-xl border border-zinc-700 bg-zinc-900 px-4 py-2.5 text-[14px] text-zinc-100 placeholder:text-zinc-500 focus:border-indigo-500 focus:outline-none disabled:opacity-60"
        />
        <button
          onClick={submit}
          disabled={disabled || !value.trim()}
          className="rounded-xl bg-indigo-600 px-4 py-2.5 text-[14px] font-medium text-white transition hover:bg-indigo-500 disabled:cursor-not-allowed disabled:opacity-40"
        >
          发送
        </button>
      </div>
      <p className="mx-auto mt-1.5 max-w-3xl text-right text-[11px] text-zinc-600">
        Enter 发送 · Shift+Enter 换行
      </p>
    </div>
  );
}
