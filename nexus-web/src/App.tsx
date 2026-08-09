import { useNexusApp } from './acp/hooks';
import type { ConnectionStatus } from './acp/connection';
import ChatView from './components/ChatView';
import SessionDetail from './components/SessionDetail';
import Sidebar from './components/Sidebar';

const STATUS_LABEL: Record<ConnectionStatus, { text: string; dot: string }> = {
  disconnected: { text: '未连接', dot: 'bg-zinc-500' },
  connecting: { text: '连接中…', dot: 'bg-amber-400 animate-pulse' },
  initializing: { text: '握手…', dot: 'bg-amber-400 animate-pulse' },
  ready: { text: '已连接', dot: 'bg-emerald-400' },
  error: { text: '连接错误', dot: 'bg-red-500' },
};

function StatusBar({
  status,
  sessionId,
}: {
  status: ConnectionStatus;
  sessionId: string | null;
}) {
  const s = STATUS_LABEL[status];
  return (
    <header className="flex items-center gap-3 border-b border-zinc-800 bg-zinc-950/80 px-4 py-2.5">
      <span className="flex items-center gap-2">
        <span className={`h-2 w-2 rounded-full ${s.dot}`} />
        <span className="text-[13px] font-medium text-zinc-300">{s.text}</span>
      </span>
      <span className="text-[13px] font-semibold text-zinc-100">Nexus</span>
      {sessionId && (
        <span className="ml-auto hidden truncate font-mono text-[11px] text-zinc-600 sm:block">
          {sessionId.slice(0, 8)}
        </span>
      )}
    </header>
  );
}

function ErrorScreen({ message, onRetry }: { message: string; onRetry: () => void }) {
  return (
    <div className="flex h-full items-center justify-center">
      <div className="max-w-md rounded-xl border border-red-900/60 bg-red-950/30 p-6 text-center">
        <p className="text-[14px] text-red-200">{message}</p>
        <button
          onClick={onRetry}
          className="mt-4 rounded-lg bg-zinc-800 px-4 py-2 text-[13px] text-zinc-100 hover:bg-zinc-700"
        >
          重试
        </button>
      </div>
    </div>
  );
}

export default function App() {
  const app = useNexusApp();

  // Connection-level failure (no key, handshake error): full-screen retry.
  // Operation-level errors (e.g. a failed prompt) render as an inline banner
  // above the input so the conversation stays visible.
  const fatal = app.status === 'error';
  const activeSession = app.roster.find((e) => e.sessionId === app.sessionId) ?? null;

  return (
    <div className="flex h-full flex-col bg-zinc-950 text-zinc-100">
      <StatusBar status={fatal ? 'error' : app.status} sessionId={app.sessionId} />
      {fatal ? (
        <ErrorScreen message={app.error ?? '连接失败'} onRetry={app.retry} />
      ) : (
        <div className="flex min-h-0 flex-1">
          <Sidebar
            roster={app.roster}
            activeSessionId={app.sessionId}
            onSwitch={(id) => void app.switchSession(id)}
            onNewSession={() => void app.createNewSession()}
          />
          <div className="flex min-w-0 flex-1 flex-col">
            <ChatView
              messages={app.messages}
              connected={app.status === 'ready' && !!app.sessionId}
              onSend={app.sendMessage}
              error={app.error ?? undefined}
            />
          </div>
          <SessionDetail
            models={app.models}
            currentModelId={app.currentModelId}
            session={activeSession}
            onSwitchModel={(m) => void app.switchModel(m)}
          />
        </div>
      )}
    </div>
  );
}
