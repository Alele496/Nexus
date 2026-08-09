import type { ChatMsg } from '../acp/hooks';
import InputBox from './InputBox';
import MessageList from './MessageList';

interface Props {
  messages: ChatMsg[];
  connected: boolean;
  streaming: boolean;
  onSend: (text: string) => void;
  onStop: () => void;
  error?: string;
}

export default function ChatView({
  messages,
  connected,
  streaming,
  onSend,
  onStop,
  error,
}: Props) {
  return (
    <div className="flex min-h-0 flex-1 flex-col">
      <MessageList messages={messages} />
      {error && (
        <div className="border-t border-red-900/60 bg-red-950/30 px-4 py-2 text-center text-[12px] text-red-300">
          {error}
        </div>
      )}
      {streaming && (
        <div className="flex justify-center border-t border-zinc-800/70 bg-zinc-950/60 py-1.5">
          <button
            onClick={onStop}
            title="中止当前生成"
            className="flex items-center gap-1.5 rounded-full border border-red-500/40 bg-red-500/10 px-3 py-1 text-[12px] text-red-300 transition hover:bg-red-500/20"
          >
            <svg className="h-2.5 w-2.5" fill="currentColor" viewBox="0 0 24 24">
              <rect x="6" y="6" width="12" height="12" rx="1.5" />
            </svg>
            停止生成
          </button>
        </div>
      )}
      <InputBox disabled={!connected} onSend={onSend} />
    </div>
  );
}
