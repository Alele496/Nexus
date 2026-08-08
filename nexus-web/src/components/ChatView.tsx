import type { ChatMsg } from '../acp/hooks';
import InputBox from './InputBox';
import MessageList from './MessageList';

interface Props {
  messages: ChatMsg[];
  connected: boolean;
  onSend: (text: string) => void;
  error?: string;
}

export default function ChatView({ messages, connected, onSend, error }: Props) {
  return (
    <div className="flex min-h-0 flex-1 flex-col">
      <MessageList messages={messages} />
      {error && (
        <div className="border-t border-red-900/60 bg-red-950/30 px-4 py-2 text-center text-[12px] text-red-300">
          {error}
        </div>
      )}
      <InputBox disabled={!connected} onSend={onSend} />
    </div>
  );
}
