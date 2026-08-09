import { useState } from 'react';

import { useNexusApp } from './acp/hooks';
import type { ConnectionStatus } from './acp/connection';
import ApprovalModal from './components/ApprovalModal';
import ChatView from './components/ChatView';
import MailboxView from './components/MailboxView';
import SessionDetail from './components/SessionDetail';
import SettingsPanel from './components/SettingsPanel';
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
  mailboxUnread,
  onOpenMailbox,
  onOpenSettings,
}: {
  status: ConnectionStatus;
  sessionId: string | null;
  mailboxUnread: number;
  onOpenMailbox: () => void;
  onOpenSettings: () => void;
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
        <span className="hidden truncate font-mono text-[11px] text-zinc-600 sm:block">
          {sessionId.slice(0, 8)}
        </span>
      )}
      <div className="ml-auto flex items-center gap-2">
        <button
          onClick={onOpenMailbox}
          title="信箱"
          className="relative flex h-6 w-6 items-center justify-center rounded-md border border-zinc-700 text-zinc-300 hover:border-zinc-500 hover:text-zinc-100"
        >
          <svg className="h-3.5 w-3.5" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}>
            <path
              strokeLinecap="round"
              strokeLinejoin="round"
              d="M21 8l-9 5-9-5V5a2 2 0 012-2h14a2 2 0 012 2v3zM21 8v9a2 2 0 01-2 2H5a2 2 0 01-2-2V8l9 5 9-5z"
            />
          </svg>
          {mailboxUnread > 0 && (
            <span className="absolute -top-1.5 -right-1.5 flex h-4 min-w-4 items-center justify-center rounded-full bg-amber-500 px-1 text-[9px] font-semibold text-zinc-950">
              {mailboxUnread > 99 ? '99+' : mailboxUnread}
            </span>
          )}
        </button>
        <button
          onClick={onOpenSettings}
          title="设置"
          className="flex h-6 w-6 items-center justify-center rounded-md border border-zinc-700 text-zinc-300 hover:border-zinc-500 hover:text-zinc-100"
        >
          <svg className="h-3.5 w-3.5" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}>
            <path
              strokeLinecap="round"
              strokeLinejoin="round"
              d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z"
            />
            <path strokeLinecap="round" strokeLinejoin="round" d="M15 12a3 3 0 11-6 0 3 3 0 016 0z" />
          </svg>
        </button>
      </div>
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
  const [settingsOpen, setSettingsOpen] = useState(false);

  // Connection-level failure (no key, handshake error): full-screen retry.
  // Operation-level errors (e.g. a failed prompt) render as an inline banner
  // above the input so the conversation stays visible.
  const fatal = app.status === 'error';
  const activeSession = app.roster.find((e) => e.sessionId === app.sessionId) ?? null;

  // Unread mail addressed to the active session (by id or by a label the
  // session owns), shown as the mailbox button's badge.
  const myLabels = Object.entries(app.mailboxLabels)
    .filter(([, sid]) => sid === app.sessionId)
    .map(([label]) => label);
  const mailboxUnread = app.mailboxMessages.filter(
    (m) =>
      (m.status === 'pending' || m.status === 'delivered') &&
      (m.to.sessionId === app.sessionId || myLabels.includes(m.to.sessionId)),
  ).length;

  return (
    <div className="flex h-full flex-col bg-zinc-950 text-zinc-100">
      <StatusBar
        status={fatal ? 'error' : app.status}
        sessionId={app.sessionId}
        mailboxUnread={mailboxUnread}
        onOpenMailbox={app.toggleMailbox}
        onOpenSettings={() => setSettingsOpen(true)}
      />
      <ApprovalModal request={app.pendingPermission} onRespond={app.respondPermission} />
      <MailboxView
        open={app.mailboxOpen}
        messages={app.mailboxMessages}
        labels={app.mailboxLabels}
        sessionId={app.sessionId}
        onClose={app.toggleMailbox}
        onRefresh={() => void app.refreshMailbox()}
        onMarkRead={(id) => void app.markMailboxRead(id)}
      />
      <SettingsPanel
        open={settingsOpen}
        onClose={() => setSettingsOpen(false)}
        loadApiKey={app.loadApiKey}
        saveApiKey={app.saveApiKey}
        loadAuthInfo={app.loadAuthInfo}
        logout={app.logout}
        loadCommands={app.loadCommands}
        reloadModels={app.reloadModels}
        reloadSkills={app.reloadSkills}
        sessionId={app.sessionId}
        flushMemory={(sid) => app.flushMemory(sid)}
        rewriteMemoryNote={(sid, rawText, contextSummary) =>
          app.rewriteMemoryNote(sid, rawText, contextSummary)
        }
        recap={(sid) => app.recap(sid)}
      />
      {fatal ? (
        <ErrorScreen message={app.error ?? '连接失败'} onRetry={app.retry} />
      ) : (
        <div className="flex min-h-0 flex-1">
          <Sidebar
            roster={app.roster}
            activeSessionId={app.sessionId}
            onSwitch={(id) => void app.switchSession(id)}
            onNewSession={() => void app.createNewSession()}
            onRename={(id, title) => app.renameSession(id, title)}
            onDelete={(id) => app.deleteSession(id)}
            onFork={(id) => app.forkSession(id)}
            onSearch={(q) => app.searchSessions(q)}
          />
          <div className="flex min-w-0 flex-1 flex-col">
            <ChatView
              messages={app.messages}
              connected={app.status === 'ready' && !!app.sessionId}
              streaming={app.messages.some(
                (m) =>
                  (m.kind === 'text' && m.streaming) ||
                  (m.kind === 'tool' &&
                    (m.status === 'in_progress' || m.status === 'running')),
              )}
              onSend={app.sendMessage}
              onStop={app.cancelTurn}
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
