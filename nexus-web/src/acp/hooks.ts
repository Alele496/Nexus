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
  type SessionUpdate,
  type ToolCallStatus,
} from './types';

// ---- Message model ----

export interface TextMsg {
  kind: 'text';
  id: number;
  role: 'user' | 'assistant';
  text: string;
  streaming: boolean;
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
}

export type ChatMsg = TextMsg | ThoughtMsg | ToolMsg;

interface ChatState {
  messages: ChatMsg[];
  nextId: number;
  agentIndex: number | null;
  thoughtIndex: number | null;
}

type ChatAction =
  | { type: 'user-optimistic'; text: string }
  | { type: 'agent-chunk'; text: string }
  | { type: 'thought-chunk'; text: string }
  | { type: 'tool-start'; update: SessionUpdate }
  | { type: 'tool-update'; update: SessionUpdate };

const initialState: ChatState = {
  messages: [],
  nextId: 1,
  agentIndex: null,
  thoughtIndex: null,
};

function reducer(state: ChatState, action: ChatAction): ChatState {
  switch (action.type) {
    case 'user-optimistic': {
      // A new turn starts: close out the previous streaming assistant message.
      const messages = [...state.messages];
      if (state.agentIndex !== null && messages[state.agentIndex]?.kind === 'text') {
        const prev = messages[state.agentIndex] as TextMsg;
        messages[state.agentIndex] = { ...prev, streaming: false };
      }
      const id = state.nextId;
      messages.push({ kind: 'text', id, role: 'user', text: action.text, streaming: false });
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
        messages.push({ kind: 'text', id, role: 'assistant', text: '', streaming: true });
        agentIndex = messages.length - 1;
      } else {
        agentIndex = state.agentIndex;
      }
      const msg = messages[agentIndex] as TextMsg;
      messages[agentIndex] = { ...msg, text: msg.text + action.text };
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
      });
      // Text after a tool call renders as a fresh assistant message.
      return { ...state, nextId: id + 1, agentIndex: null };
    }

    case 'tool-update': {
      const u = action.update as Extract<SessionUpdate, { sessionUpdate: 'tool_call_update' }>;
      const messages = state.messages.map((m) =>
        m.kind === 'tool' && m.toolCallId === u.toolCallId
          ? { ...m, status: u.status ?? m.status }
          : m,
      );
      return { ...state, messages };
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

function routeUpdate(update: SessionUpdate, dispatch: (a: ChatAction) => void): void {
  switch (update.sessionUpdate) {
    case 'user_message_chunk':
      // The user's own text is rendered optimistically; ignore the echo.
      break;
    case 'agent_message_chunk': {
      const u = update as Extract<SessionUpdate, { sessionUpdate: 'agent_message_chunk' }>;
      dispatch({ type: 'agent-chunk', text: chunkText(u.content) });
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

export interface NexusApp {
  status: ConnectionStatus;
  error: string | null;
  sessionId: string | null;
  messages: ChatMsg[];
  sendMessage: (text: string) => void;
  retry: () => void;
}

export function useNexusApp(): NexusApp {
  const [status, setStatus] = useState<ConnectionStatus>('disconnected');
  const [error, setError] = useState<string | null>(null);
  const [sessionId, setSessionId] = useState<string | null>(null);
  const connRef = useRef<AcpConnection | null>(null);
  const { state, dispatch } = useThrottledChat();

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
          if (update) routeUpdate(update, dispatch);
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

  // Once ready: load the existing session (reconnect) or create a new one.
  useEffect(() => {
    const conn = connRef.current;
    if (status !== 'ready' || !conn) return;
    let cancelled = false;
    (async () => {
      try {
        const cwd = resolveServerCwd();
        const knownId = conn.getSessionId() ?? resolveInitialSession();
        if (knownId) {
          await conn.loadSession(knownId, cwd);
        } else {
          await conn.createSession(cwd);
        }
        if (!cancelled) setSessionId(conn.getSessionId());
      } catch (err) {
        if (!cancelled) setError(String(err));
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [status]);

  const sendMessage = useCallback(
    (text: string) => {
      const conn = connRef.current;
      const trimmed = text.trim();
      if (!conn || !trimmed) return;
      dispatch({ type: 'user-optimistic', text: trimmed });
      void conn.prompt(trimmed).catch((err) => setError(String(err)));
    },
    [dispatch],
  );

  const retry = useCallback(() => {
    setError(null);
    connRef.current?.close();
    connRef.current?.connect();
  }, []);

  return { status, error, sessionId, messages: state.messages, sendMessage, retry };
}
