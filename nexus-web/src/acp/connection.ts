// ACP WebSocket client for the Nexus agent server.
//
// Wire format: JSON-RPC 2.0, one JSON object per line over a single WebSocket.
// Handshake (mirrors the TUI pager): initialize → authenticate (cached_token)
// → session/new or session/load. The agent instance persists across WS
// reconnects, so on reconnect we only need to re-run the handshake and reload
// the same session id.

import type {
  AuthenticateParams,
  InitializeParams,
  JsonRpcNotification,
  JsonRpcResponse,
  NewSessionParams,
  NewSessionResult,
  PromptParams,
} from './types';

export type ConnectionStatus =
  | 'disconnected'
  | 'connecting'
  | 'initializing'
  | 'ready'
  | 'error';

const CACHED_TOKEN = 'cached_token';
const PROTOCOL_VERSION = 1;
const MAX_RECONNECT_DELAY_MS = 15_000;
const INITIAL_RECONNECT_DELAY_MS = 500;

/** Locate the server key: injected meta tag → URL `?key=` → sessionStorage. */
export function resolveServerKey(): string | null {
  const meta = document
    .querySelector('meta[name="nexus-server-key"]')
    ?.getAttribute('content');
  if (meta) return meta;
  const urlKey = new URLSearchParams(window.location.search).get('key');
  if (urlKey) return urlKey;
  return sessionStorage.getItem('nexus.serverKey');
}

/** Default session cwd injected by the Rust server (falls back to URL or `/`). */
export function resolveServerCwd(): string {
  const meta = document
    .querySelector('meta[name="nexus-server-cwd"]')
    ?.getAttribute('content');
  if (meta) return meta;
  const urlCwd = new URLSearchParams(window.location.search).get('cwd');
  if (urlCwd) return urlCwd;
  return '/';
}

/** A session id to load instead of creating a new one (URL `?session=`). */
export function resolveInitialSession(): string | null {
  return new URLSearchParams(window.location.search).get('session');
}

/** Build the WebSocket URL for the current origin. */
export function wsUrlFor(key: string | null): string {
  const proto = window.location.protocol === 'https:' ? 'wss' : 'ws';
  let url = `${proto}://${window.location.host}/ws`;
  if (key) url += `?server-key=${encodeURIComponent(key)}`;
  return url;
}

export interface AcpConnectionOptions {
  url: string;
  clientType?: string;
  clientVersion?: string;
  /** Called with a raw parsed notification (method + params). */
  onNotification?: (method: string, params: JsonRpcNotification['params']) => void;
  onStatusChange?: (status: ConnectionStatus) => void;
}

interface PendingRequest {
  resolve: (result: any) => void;
  reject: (err: Error) => void;
}

export class AcpConnection {
  private readonly options: AcpConnectionOptions;
  private ws: WebSocket | null = null;
  private nextId = 1;
  private pending = new Map<number, PendingRequest>();
  private reconnectTimer: number | null = null;
  private reconnectDelay = INITIAL_RECONNECT_DELAY_MS;
  private intentionallyClosed = false;
  private status: ConnectionStatus = 'disconnected';

  /** Session bound after handshake; reloaded on reconnect. */
  private sessionId: string | null = null;

  constructor(options: AcpConnectionOptions) {
    this.options = options;
  }

  getStatus(): ConnectionStatus {
    return this.status;
  }

  getSessionId(): string | null {
    return this.sessionId;
  }

  connect(): void {
    if (this.ws && (this.ws.readyState === WebSocket.OPEN || this.ws.readyState === WebSocket.CONNECTING)) {
      return;
    }
    this.intentionallyClosed = false;
    this.setStatus('connecting');

    const ws = new WebSocket(this.options.url);
    this.ws = ws;

    ws.onopen = () => {
      this.reconnectDelay = INITIAL_RECONNECT_DELAY_MS;
      this.setStatus('initializing');
      void this.runHandshake();
    };

    ws.onmessage = (event) => this.handleMessage(event.data);

    ws.onerror = () => {
      // onclose follows; schedule reconnect there.
    };

    ws.onclose = () => {
      this.ws = null;
      this.rejectAllPending(new Error('connection closed'));
      if (!this.intentionallyClosed) {
        this.setStatus(this.sessionId ? 'connecting' : 'disconnected');
        this.scheduleReconnect();
      } else {
        this.setStatus('disconnected');
      }
    };
  }

  close(): void {
    this.intentionallyClosed = true;
    if (this.reconnectTimer !== null) {
      window.clearTimeout(this.reconnectTimer);
      this.reconnectTimer = null;
    }
    this.ws?.close();
    this.ws = null;
    this.setStatus('disconnected');
  }

  /** JSON-RPC request; resolves with `result`, rejects with `error`. */
  request<T = any>(method: string, params?: unknown): Promise<T> {
    if (!this.ws || this.ws.readyState !== WebSocket.OPEN) {
      return Promise.reject(new Error('not connected'));
    }
    const id = this.nextId++;
    const payload = JSON.stringify({ jsonrpc: '2.0', id, method, params });
    this.ws.send(payload);

    return new Promise<T>((resolve, reject) => {
      this.pending.set(id, { resolve, reject });
    });
  }

  /** Send a one-way message (no response expected). */
  notify(method: string, params?: unknown): void {
    if (!this.ws || this.ws.readyState !== WebSocket.OPEN) return;
    this.ws.send(JSON.stringify({ jsonrpc: '2.0', method, params }));
  }

  async initialize(): Promise<void> {
    const params: InitializeParams = {
      protocolVersion: PROTOCOL_VERSION,
      clientCapabilities: {
        fs: { readTextFile: true, writeTextFile: true },
        terminal: false,
      },
      clientInfo: {
        name: this.options.clientType ?? 'nexus-web',
        version: this.options.clientVersion ?? '0.1.0',
      },
      _meta: {
        clientType: this.options.clientType ?? 'nexus-web',
        clientVersion: this.options.clientVersion ?? '0.1.0',
      },
    };
    await this.request('initialize', params);
  }

  async authenticate(): Promise<void> {
    const params: AuthenticateParams = { methodId: CACHED_TOKEN };
    await this.request('authenticate', params);
  }

  async createSession(cwd: string, meta?: NewSessionParams['_meta']): Promise<string> {
    const params: NewSessionParams = { cwd, mcpServers: [], _meta: meta };
    const result = await this.request<NewSessionResult>('session/new', params);
    if (!result.sessionId) throw new Error('session/new returned no sessionId');
    this.sessionId = result.sessionId;
    return result.sessionId;
  }

  async loadSession(sessionId: string, cwd: string): Promise<void> {
    await this.request('session/load', { sessionId, cwd, mcpServers: [] });
    this.sessionId = sessionId;
  }

  /** Send a user prompt to the current session. */
  prompt(text: string): Promise<unknown> {
    if (!this.sessionId) return Promise.reject(new Error('no session'));
    const params: PromptParams = {
      sessionId: this.sessionId,
      prompt: [{ type: 'text', text }],
    };
    return this.request('session/prompt', params);
  }

  private async runHandshake(): Promise<void> {
    try {
      await this.initialize();
      await this.authenticate();
      this.setStatus('ready');
    } catch (err) {
      console.error('handshake failed:', err);
      this.setStatus('error');
      this.ws?.close();
    }
  }

  private handleMessage(data: unknown): void {
    let msg: JsonRpcResponse | JsonRpcNotification | null = null;
    try {
      msg = JSON.parse(String(data));
    } catch {
      return; // keepalive or malformed — ignore
    }
    if (!msg) return;

    if ('id' in msg && typeof msg.id === 'number') {
      const resp = msg as JsonRpcResponse;
      const entry = this.pending.get(resp.id);
      if (entry) {
        this.pending.delete(resp.id);
        if (resp.error) {
          entry.reject(
            new Error(`${resp.error.message} (${resp.error.code})`),
          );
        } else {
          entry.resolve(resp.result);
        }
      }
      return;
    }

    if ('method' in msg && typeof msg.method === 'string') {
      this.options.onNotification?.(msg.method, msg.params);
    }
  }

  private scheduleReconnect(): void {
    if (this.reconnectTimer !== null) return;
    this.reconnectTimer = window.setTimeout(() => {
      this.reconnectTimer = null;
      this.setStatus('connecting');
      this.connect();
    }, this.reconnectDelay);
    this.reconnectDelay = Math.min(this.reconnectDelay * 2, MAX_RECONNECT_DELAY_MS);
  }

  private rejectAllPending(err: Error): void {
    for (const [, entry] of this.pending) {
      entry.reject(err);
    }
    this.pending.clear();
  }

  private setStatus(status: ConnectionStatus): void {
    if (this.status !== status) {
      this.status = status;
      this.options.onStatusChange?.(status);
    }
  }
}
