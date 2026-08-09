// React glue: connection lifecycle + a frame-throttled chat message store.
//
// Streaming chunks arrive at very high frequency (one WS message per token
// chunk). Rather than calling setState per chunk (which re-renders the whole
// list each time), actions are queued and flushed once per animation frame, so
// the UI renders at display refresh rate regardless of chunk rate.

import { useCallback, useEffect, useRef, useState } from 'react';

import {
  AcpConnection,
  resolveInitialSession,
  resolveServerCwd,
  resolveServerKey,
  wsUrlFor,
  type ConnectionStatus,
} from './connection';
import {
  chunkText,
  contentImages,
  type MailboxMessage,
  type ModelInfo,
  type RequestPermissionOutcomeResponse,
  type RequestPermissionParams,
  type JsonValue,
  type RosterChanged,
  type RosterEntry,
  type SessionSearchHit,
  type SessionUpdate,
  type ToolCallStatus,
} from './types';

// ---- Message model ----

/** An image embedded in a message, as a base64 data URL. */
export interface ImageBlock {
  src: string;
  mimeType?: string;
}

export interface TextMsg {
  kind: 'text';
  id: number;
  role: 'user' | 'assistant';
  text: string;
  streaming: boolean;
  images: ImageBlock[];
}

export interface ThoughtMsg {
  kind: 'thought';
  id: number;
  text: string;
}

export interface ToolMsg {
  kind: 'tool';
  id: number;
  toolCallId: string;
  name: string;
  status: ToolCallStatus;
  rawInput?: JsonValue;
  rawOutput?: JsonValue;
}

export type ChatMsg = TextMsg | ThoughtMsg | ToolMsg;

interface ChatState {
  messages: ChatMsg[];
  nextId: number;
  agentIndex: number | null;
  thoughtIndex: number | null;
}

type ChatAction =
  | { type: 'session-reset' }
  | { type: 'user-optimistic'; text: string }
  | { type: 'user-echo'; text: string }
  | { type: 'agent-chunk'; text: string }
  | { type: 'agent-image'; image: ImageBlock }
  | { type: 'thought-chunk'; text: string }
  | { type: 'tool-start'; update: SessionUpdate }
  | { type: 'tool-update'; update: SessionUpdate }
  | { type: 'close-streaming' };

const initialState: ChatState = {
  messages: [],
  nextId: 1,
  agentIndex: null,
  thoughtIndex: null,
};

export function reducer(state: ChatState, action: ChatAction): ChatState {
  switch (action.type) {
    case 'session-reset':
      // New session or session switch: drop all accumulated messages.
      return { ...state, messages: [], nextId: 1, agentIndex: null, thoughtIndex: null };

    case 'user-echo':
      // A user message replayed from session/load history (vs. the optimistic
      // echo of our own prompt, which arrives as 'user-optimistic').
      messagesPush(state, {
        kind: 'text',
        id: state.nextId,
        role: 'user',
        text: action.text,
        streaming: false,
        images: [],
      });
      return { ...state, nextId: state.nextId + 1 };

    case 'user-optimistic': {
      // A new turn starts: close out the previous streaming assistant message.
      const messages = [...state.messages];
      if (state.agentIndex !== null && messages[state.agentIndex]?.kind === 'text') {
        const prev = messages[state.agentIndex] as TextMsg;
        messages[state.agentIndex] = { ...prev, streaming: false };
      }
      const id = state.nextId;
      messages.push({ kind: 'text', id, role: 'user', text: action.text, streaming: false, images: [] });
      return { ...state, messages, nextId: id + 1, agentIndex: null, thoughtIndex: null };
    }

    case 'agent-chunk': {
      const messages = [...state.messages];
      const current =
        state.agentIndex !== null ? messages[state.agentIndex] : undefined;
      let agentIndex: number;
      if (
        state.agentIndex === null ||
        current?.kind !== 'text' ||
        current.role !== 'assistant' ||
        !current.streaming
      ) {
        const id = state.nextId;
        messages.push({ kind: 'text', id, role: 'assistant', text: '', streaming: true, images: [] });
        agentIndex = messages.length - 1;
      } else {
        agentIndex = state.agentIndex;
      }
      const msg = messages[agentIndex] as TextMsg;
      messages[agentIndex] = { ...msg, text: msg.text + action.text };
      return { ...state, messages, nextId: state.nextId, agentIndex };
    }

    case 'agent-image': {
      const messages = [...state.messages];
      const current =
        state.agentIndex !== null ? messages[state.agentIndex] : undefined;
      let agentIndex: number;
      if (
        state.agentIndex === null ||
        current?.kind !== 'text' ||
        current.role !== 'assistant' ||
        !current.streaming
      ) {
        const id = state.nextId;
        messages.push({ kind: 'text', id, role: 'assistant', text: '', streaming: true, images: [action.image] });
        agentIndex = messages.length - 1;
      } else {
        agentIndex = state.agentIndex;
        const msg = messages[agentIndex] as TextMsg;
        messages[agentIndex] = { ...msg, images: [...msg.images, action.image] };
      }
      return { ...state, messages, nextId: state.nextId, agentIndex };
    }

    case 'thought-chunk': {
      const messages = [...state.messages];
      const current =
        state.thoughtIndex !== null ? messages[state.thoughtIndex] : undefined;
      let thoughtIndex: number;
      if (state.thoughtIndex === null || current?.kind !== 'thought') {
        const id = state.nextId;
        messages.push({ kind: 'thought', id, text: '' });
        thoughtIndex = messages.length - 1;
      } else {
        thoughtIndex = state.thoughtIndex;
      }
      const msg = messages[thoughtIndex] as ThoughtMsg;
      messages[thoughtIndex] = { ...msg, text: msg.text + action.text };
      return { ...state, messages, nextId: state.nextId, thoughtIndex };
    }

    case 'tool-start': {
      const u = action.update as Extract<SessionUpdate, { sessionUpdate: 'tool_call' }>;
      const id = state.nextId;
      messagesPush(state, {
        kind: 'tool',
        id,
        toolCallId: u.toolCallId,
        name: u.name ?? u.title ?? 'tool',
        status: u.status ?? 'running',
        ...(u.rawInput !== undefined ? { rawInput: u.rawInput } : {}),
      });
      // Text after a tool call renders as a fresh assistant message.
      return { ...state, nextId: id + 1, agentIndex: null };
    }

    case 'tool-update': {
      const u = action.update as Extract<SessionUpdate, { sessionUpdate: 'tool_call_update' }>;
      const messages = state.messages.map((m) =>
        m.kind === 'tool' && m.toolCallId === u.toolCallId
          ? {
              ...m,
              status: u.status ?? m.status,
              ...(u.rawOutput !== undefined ? { rawOutput: u.rawOutput } : {}),
            }
          : m,
      );
      return { ...state, messages };
    }

    case 'close-streaming': {
      // Turn ended (prompt response / history replay): drop the caret and the
      // streaming message anchors so the next chunk starts a fresh message.
      const messages = [...state.messages];
      if (state.agentIndex !== null && messages[state.agentIndex]?.kind === 'text') {
        const msg = messages[state.agentIndex] as TextMsg;
        if (msg.streaming) messages[state.agentIndex] = { ...msg, streaming: false };
      }
      return { ...state, messages, agentIndex: null, thoughtIndex: null };
    }
  }
}

function messagesPush(state: ChatState, msg: ChatMsg): void {
  state.messages.push(msg);
}

// ---- Frame-throttled dispatch ----

function useThrottledChat() {
  const [state, setState] = useState<ChatState>(initialState);
  const queueRef = useRef<ChatAction[]>([]);
  const rafRef = useRef<number | null>(null);

  const dispatch = useCallback((action: ChatAction) => {
    queueRef.current.push(action);
    if (rafRef.current === null) {
      rafRef.current = requestAnimationFrame(() => {
        rafRef.current = null;
        const batch = queueRef.current;
        queueRef.current = [];
        setState((prev) => batch.reduce(reducer, prev));
      });
    }
  }, []);

  useEffect(() => {
    return () => {
      if (rafRef.current !== null) cancelAnimationFrame(rafRef.current);
    };
  }, []);

  return { state, dispatch };
}

// ---- Notification routing ----

function routeUpdate(
  update: SessionUpdate,
  dispatch: (a: ChatAction) => void,
  replay: boolean,
): void {
  switch (update.sessionUpdate) {
    case 'user_message_chunk': {
      // Live turns render the user's own text optimistically, so the echo is
      // ignored. During session/load the chunks ARE the history — render them.
      const u = update as Extract<SessionUpdate, { sessionUpdate: 'user_message_chunk' }>;
      if (replay) {
        const text = chunkText(u.content);
        if (text) dispatch({ type: 'user-echo', text });
      }
      break;
    }
    case 'agent_message_chunk': {
      const u = update as Extract<SessionUpdate, { sessionUpdate: 'agent_message_chunk' }>;
      const text = chunkText(u.content);
      if (text) dispatch({ type: 'agent-chunk', text });
      for (const image of contentImages(u.content)) {
        dispatch({ type: 'agent-image', image });
      }
      break;
    }
    case 'agent_thought_chunk': {
      const u = update as Extract<SessionUpdate, { sessionUpdate: 'agent_thought_chunk' }>;
      dispatch({ type: 'thought-chunk', text: chunkText(u.content) });
      break;
    }
    case 'tool_call':
      dispatch({ type: 'tool-start', update });
      break;
    case 'tool_call_update':
      dispatch({ type: 'tool-update', update });
      break;
    default:
      break; // plan / mode / info / config — Phase 2+
  }
}

// ---- Top-level hook ----

/** A pending `session/request_permission` awaiting the user's decision. */
export interface PendingPermission {
  requestId: number;
  params: RequestPermissionParams;
}

export interface NexusApp {
  status: ConnectionStatus;
  error: string | null;
  sessionId: string | null;
  messages: ChatMsg[];
  roster: RosterEntry[];
  models: ModelInfo[];
  currentModelId: string | null;
  /** Oldest pending permission request, or null. */
  pendingPermission: PendingPermission | null;
  respondPermission: (optionId: string | null) => void;
  mailboxOpen: boolean;
  mailboxMessages: MailboxMessage[];
  mailboxLabels: Record<string, string>;
  toggleMailbox: () => void;
  refreshMailbox: () => Promise<void>;
  markMailboxRead: (messageId: string) => Promise<void>;
  sendMessage: (text: string) => void;
  retry: () => void;
  switchSession: (sessionId: string) => Promise<void>;
  createNewSession: () => Promise<void>;
  renameSession: (sessionId: string, title: string) => Promise<void>;
  deleteSession: (sessionId: string) => Promise<void>;
  forkSession: (sessionId: string) => Promise<void>;
  searchSessions: (query: string) => Promise<SessionSearchHit[]>;
  cancelTurn: () => void;
  switchModel: (modelId: string) => Promise<void>;
}

/** Merge a `sage.local/sessions/changed` delta into the current roster. */
export function mergeRoster(prev: RosterEntry[], changed: RosterChanged): RosterEntry[] {
  let next = changed.removed.length
    ? prev.filter((e) => !changed.removed.includes(e.sessionId))
    : prev;
  for (const up of changed.upserted) {
    const i = next.findIndex((e) => e.sessionId === up.sessionId);
    if (i >= 0) next = next.map((e, j) => (j === i ? up : e));
    else next = [...next, up];
  }
  return [...next].sort((a, b) => b.lastChangeUnixMs - a.lastChangeUnixMs);
}

/**
 * Handle a `sage.local/session_notification` broadcast. The leader relays ext
 * notifications wrapped as `{method, params}` under a `_`-prefixed method, so
 * both that and the direct `{sessionId, update}` form are unwrapped here. The
 * ext `SessionUpdate` uses snake_case field names (unlike the ACP
 * `session/update` shape), so the title field is read as `session_summary`
 * with a camelCase fallback.
 */
export function applySessionNotification(
  method: string,
  payload: unknown,
  setRoster: (fn: (prev: RosterEntry[]) => RosterEntry[]) => void,
): void {
  if (method !== '_sage.local/session_notification' && method !== 'sage.local/session_notification') {
    return;
  }
  let inner: unknown = payload;
  if (
    payload &&
    typeof payload === 'object' &&
    'method' in payload &&
    (payload as { method?: unknown }).method === 'sage.local/session_notification'
  ) {
    inner = (payload as { params?: unknown }).params;
  }
  if (!inner || typeof inner !== 'object') return;
  const notif = inner as { sessionId?: unknown; update?: unknown };
  if (typeof notif.sessionId !== 'string' || !notif.update || typeof notif.update !== 'object') {
    return;
  }
  const update = notif.update as Record<string, unknown>;
  if (update.sessionUpdate !== 'session_summary_generated') return;
  const summary =
    typeof update.session_summary === 'string'
      ? update.session_summary
      : typeof update.sessionSummary === 'string'
        ? update.sessionSummary
        : null;
  if (!summary) return;
  const sid = notif.sessionId;
  setRoster((prev) => prev.map((e) => (e.sessionId === sid ? { ...e, title: summary } : e)));
}

export function useNexusApp(): NexusApp {
  const [status, setStatus] = useState<ConnectionStatus>('disconnected');
  const [error, setError] = useState<string | null>(null);
  const [sessionId, setSessionId] = useState<string | null>(null);
  const [roster, setRoster] = useState<RosterEntry[]>([]);
  const [models, setModels] = useState<ModelInfo[]>([]);
  const [currentModelId, setCurrentModelId] = useState<string | null>(null);
  const [permissionQueue, setPermissionQueue] = useState<PendingPermission[]>([]);
  const [mailboxOpen, setMailboxOpen] = useState(false);
  const [mailboxMessages, setMailboxMessages] = useState<MailboxMessage[]>([]);
  const [mailboxLabels, setMailboxLabels] = useState<Record<string, string>>({});
  const connRef = useRef<AcpConnection | null>(null);
  // True while `session/load` history chunks are streaming in, so
  // `user_message_chunk` is rendered as history instead of ignored as an echo.
  const replayRef = useRef(false);
  const { state, dispatch } = useThrottledChat();

  const refreshRoster = useCallback(async () => {
    const conn = connRef.current;
    if (!conn) return;
    try {
      const { sessions } = await conn.listSessions();
      setRoster(sessions);
    } catch {
      // Non-fatal: the next poll or broadcast will retry.
    }
  }, []);

  // Create the connection once.
  useEffect(() => {
    const key = resolveServerKey();
    if (!key) {
      setError('未找到服务器密钥（server key）。请通过带 ?key= 的链接访问，或重新从终端打开 Web UI。');
      setStatus('error');
      return;
    }
    const conn = new AcpConnection({
      url: wsUrlFor(key),
      clientType: 'nexus-web',
      clientVersion: '0.1.0',
      onStatusChange: (s) => setStatus(s),
      onNotification: (method, params) => {
        if (method === 'session/update' && params && typeof params === 'object') {
          const update = (params as { update?: SessionUpdate }).update;
          if (update) routeUpdate(update, dispatch, replayRef.current);
        } else if (
          method === '_sage.local/sessions/changed' &&
          params &&
          typeof params === 'object'
        ) {
          setRoster((prev) => mergeRoster(prev, params as unknown as RosterChanged));
        } else if (
          (method === '_sage.local/session_notification' ||
            method === 'sage.local/session_notification') &&
          params &&
          typeof params === 'object'
        ) {
          applySessionNotification(method, params, setRoster);
        }
      },
      onServerRequest: (id, method, params) => {
        if (method === 'session/request_permission' && params && typeof params === 'object') {
          setPermissionQueue((q) => [
            ...q,
            { requestId: id, params: params as unknown as RequestPermissionParams },
          ]);
        }
      },
    });
    connRef.current = conn;
    conn.connect();
    return () => {
      conn.close();
      connRef.current = null;
    };
  }, [dispatch]);

  // Once ready: load the existing session (reconnect) or create a new one,
  // then start the roster poll.
  useEffect(() => {
    const conn = connRef.current;
    if (status !== 'ready' || !conn) return;
    let cancelled = false;
    (async () => {
      try {
        const cwd = resolveServerCwd();
        const knownId = conn.getSessionId() ?? resolveInitialSession();
        replayRef.current = !!knownId;
        // A reconnect replays the full history from the server; drop whatever
        // the previous connection left in the store so it doesn't duplicate.
        dispatch({ type: 'session-reset' });
        const result = knownId
          ? await conn.loadSession(knownId, cwd)
          : await conn.createSession(cwd);
        if (!cancelled) {
          setSessionId(conn.getSessionId());
          if (result?.models) {
            setModels(result.models.availableModels);
            setCurrentModelId(result.models.currentModelId);
          }
          void refreshRoster();
        }
      } catch (err) {
        if (!cancelled) setError(String(err));
      } finally {
        replayRef.current = false;
        if (!cancelled) dispatch({ type: 'close-streaming' });
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [status, refreshRoster, dispatch]);

  // Slow safety poll for the roster (broadcasts cover turn transitions; the
  // poll catches anything missed, e.g. an external client's sessions).
  useEffect(() => {
    if (status !== 'ready') return;
    void refreshRoster();
    const t = window.setInterval(() => void refreshRoster(), 30_000);
    return () => window.clearInterval(t);
  }, [status, refreshRoster]);

  const sendMessage = useCallback(
    (text: string) => {
      const conn = connRef.current;
      const trimmed = text.trim();
      if (!conn || !trimmed) return;
      dispatch({ type: 'user-optimistic', text: trimmed });
      void conn
        .prompt(trimmed)
        .catch((err) => setError(String(err)))
        .finally(() => dispatch({ type: 'close-streaming' }));
    },
    [dispatch],
  );

  const switchSession = useCallback(
    async (sid: string) => {
      const conn = connRef.current;
      if (!conn || sid === sessionId) return;
      replayRef.current = true;
      dispatch({ type: 'session-reset' });
      try {
        const result = await conn.loadSession(sid, resolveServerCwd());
        setSessionId(sid);
        if (result?.models) {
          setModels(result.models.availableModels);
          setCurrentModelId(result.models.currentModelId);
        }
      } catch (err) {
        setError(String(err));
      } finally {
        replayRef.current = false;
        dispatch({ type: 'close-streaming' });
        void refreshRoster();
      }
    },
    [sessionId, refreshRoster, dispatch],
  );

  const createNewSession = useCallback(async () => {
    const conn = connRef.current;
    if (!conn) return;
    dispatch({ type: 'session-reset' });
    try {
      const result = await conn.createSession(resolveServerCwd());
      setSessionId(result.sessionId);
      if (result?.models) {
        setModels(result.models.availableModels);
        setCurrentModelId(result.models.currentModelId);
      }
      void refreshRoster();
    } catch (err) {
      setError(String(err));
    }
  }, [refreshRoster, dispatch]);

  const renameSession = useCallback(async (sid: string, title: string) => {
    const conn = connRef.current;
    const trimmed = title.trim();
    if (!conn || !trimmed) return;
    try {
      await conn.renameSession(sid, trimmed);
      setRoster((prev) =>
        prev.map((e) => (e.sessionId === sid ? { ...e, title: trimmed } : e)),
      );
    } catch (err) {
      setError(String(err));
    }
  }, []);

  const forkSession = useCallback(
    async (sid: string) => {
      const conn = connRef.current;
      const entry = roster.find((e) => e.sessionId === sid);
      if (!conn || !entry) return;
      try {
        const result = await conn.forkSession({
          sourceSessionId: sid,
          sourceCwd: entry.cwd,
          newCwd: entry.cwd,
        });
        await refreshRoster();
        if (result?.newSessionId) await switchSession(result.newSessionId);
      } catch (err) {
        setError(String(err));
      }
    },
    [roster, refreshRoster, switchSession],
  );

  const deleteSession = useCallback(
    async (sid: string) => {
      const conn = connRef.current;
      if (!conn) return;
      const wasActive = sid === sessionId;
      try {
        await conn.deleteSession(sid);
        const remaining = roster.filter((e) => e.sessionId !== sid);
        setRoster(remaining);
        if (wasActive) {
          const next = remaining[0];
          if (next) await switchSession(next.sessionId);
          else await createNewSession();
        }
      } catch (err) {
        setError(String(err));
      }
    },
    [sessionId, roster, switchSession, createNewSession],
  );

  const searchSessions = useCallback(async (query: string) => {
    const conn = connRef.current;
    const trimmed = query.trim();
    if (!conn || !trimmed) return [];
    try {
      const result = await conn.searchSessions(trimmed, { limit: 10 });
      return result.results ?? [];
    } catch {
      return [];
    }
  }, []);

  const cancelTurn = useCallback(() => {
    const conn = connRef.current;
    if (!conn || !sessionId) return;
    conn.cancelTurn(sessionId);
  }, [sessionId]);

  const switchModel = useCallback(
    async (modelId: string) => {
      const conn = connRef.current;
      if (!conn || !sessionId || modelId === currentModelId) return;
      try {
        await conn.setSessionModel(sessionId, modelId);
        setCurrentModelId(modelId);
        // The server only broadcasts roster changes on death/spawn/turn
        // transitions, not on set_model — refresh so the sidebar modelId
        // updates immediately instead of at the next poll.
        void refreshRoster();
      } catch (err) {
        setError(String(err));
      }
    },
    [sessionId, currentModelId, refreshRoster],
  );

  const respondPermission = useCallback(
    (optionId: string | null) => {
      const conn = connRef.current;
      if (!conn || permissionQueue.length === 0) return;
      const [first, ...rest] = permissionQueue;
      setPermissionQueue(rest);
      const result: RequestPermissionOutcomeResponse = optionId
        ? { outcome: { outcome: 'selected', optionId } }
        : { outcome: { outcome: 'cancelled' } };
      conn.respond(first.requestId, result);
    },
    [permissionQueue],
  );

  const refreshMailbox = useCallback(async () => {
    const conn = connRef.current;
    if (!conn) return;
    try {
      const { messages, labels } = await conn.listMailbox();
      setMailboxMessages(messages);
      setMailboxLabels(labels);
    } catch {
      // Non-fatal: stale list until the next refresh.
    }
  }, []);

  const toggleMailbox = useCallback(() => {
    setMailboxOpen((open) => {
      if (!open) void refreshMailbox();
      return !open;
    });
  }, [refreshMailbox]);

  const markMailboxRead = useCallback(async (messageId: string) => {
    const conn = connRef.current;
    if (!conn) return;
    try {
      await conn.markMailboxRead(messageId);
      setMailboxMessages((msgs) =>
        msgs.map((m) =>
          m.id === messageId
            ? { ...m, status: 'read', readAt: new Date().toISOString() }
            : m,
        ),
      );
    } catch {
      // Non-fatal: unread badge stays until the next refresh.
    }
  }, []);

  const retry = useCallback(() => {
    setError(null);
    connRef.current?.close();
    connRef.current?.connect();
  }, []);

  return {
    status,
    error,
    sessionId,
    messages: state.messages,
    roster,
    models,
    currentModelId,
    pendingPermission: permissionQueue[0] ?? null,
    respondPermission,
    mailboxOpen,
    mailboxMessages,
    mailboxLabels,
    toggleMailbox,
    refreshMailbox,
    markMailboxRead,
    sendMessage,
    retry,
    switchSession,
    createNewSession,
    renameSession,
    deleteSession,
    forkSession,
    searchSessions,
    cancelTurn,
    switchModel,
  };
}
