import { useRef, useState } from 'react';

interface Props {
  disabled?: boolean;
  onSend: (text: string) => void;
}

export default function InputBox({ disabled, onSend }: Props) {
  const [value, setValue] = useState('');
  const taRef = useRef<HTMLTextAreaElement>(null);

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

  return (
    <div className="border-t border-zinc-800 bg-zinc-950/80 px-4 py-3">
      <div className="mx-auto flex max-w-3xl items-end gap-2">
        <textarea
          ref={taRef}
          value={value}
          disabled={disabled}
          onChange={handleInput}
          onKeyDown={handleKeyDown}
          rows={1}
          placeholder={disabled ? '连接中…' : '输入消息，Enter 发送，Shift+Enter 换行'}
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
