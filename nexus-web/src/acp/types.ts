// TypeScript types for the agent-client-protocol (ACP) wire format.
//
// The browser ↔ server WebSocket carries plain JSON-RPC 2.0 (one JSON object
// per line). These types mirror `agent-client-protocol-schema` (serde
// `rename_all = "camelCase"` / `snake_case` where noted).

export type JsonValue =
  | null
  | boolean
  | number
  | string
  | JsonValue[]
  | { [key: string]: JsonValue };

export interface JsonRpcRequest {
  jsonrpc: '2.0';
  id: number;
  method: string;
  params?: JsonValue;
}

export interface JsonRpcResponse {
  jsonrpc: '2.0';
  id: number;
  result?: JsonValue;
  error?: JsonRpcError;
}

export interface JsonRpcError {
  code: number;
  message: string;
  data?: JsonValue;
}

export interface JsonRpcNotification {
  jsonrpc: '2.0';
  method: string;
  params?: JsonValue;
}

// ---- Content blocks (internally tagged by `type`, snake_case) ----

export interface TextContent {
  type: 'text';
  text: string;
}

export interface ImageContent {
  type: 'image';
  source?: {
    type?: 'data';
    data?: string;
    mimeType?: string;
  };
}

export interface ResourceContent {
  type: 'resource';
  resource?: {
    uri: string;
    text?: string;
    blob?: string;
    mimeType?: string;
  };
}

export interface ResourceLinkContent {
  type: 'resource_link';
  uri: string;
}

export type ContentBlock =
  | TextContent
  | ImageContent
  | ResourceContent
  | ResourceLinkContent;

/** One streaming chunk wraps a content block. */
export interface ContentChunk {
  content: ContentBlock;
}

/** Extract displayable text from a chunk or a raw content block (best effort). */
export function chunkText(chunk: ContentChunk | ContentBlock | undefined | null): string {
  if (!chunk) return '';
  const block: ContentBlock | undefined =
    'content' in chunk && chunk.content ? chunk.content : (chunk as ContentBlock);
  if (!block) return '';
  if (block.type === 'text') return block.text ?? '';
  if (block.type === 'resource' && block.resource?.text) return block.resource.text;
  return '';
}

/** Extract images from a chunk or raw content block, as base64 data URLs. */
export function contentImages(
  chunk: ContentChunk | ContentBlock | undefined | null,
): { src: string; mimeType?: string }[] {
  if (!chunk) return [];
  const block: ContentBlock | undefined =
    'content' in chunk && chunk.content ? chunk.content : (chunk as ContentBlock);
  if (!block || block.type !== 'image' || !block.source?.data) return [];
  const { data, mimeType } = block.source;
  return [{ src: `data:${mimeType ?? 'image/png'};base64,${data}`, mimeType }];
}

// ---- Tool calls (camelCase fields, snake_case status values) ----

export type ToolCallStatus =
  | 'in_progress'
  | 'running'
  | 'completed'
  | 'error'
  | 'cancelled'
  | (string & {});

export interface ToolCall {
  toolCallId: string;
  name?: string;
  title?: string;
  kind?: string;
  status: ToolCallStatus;
  content?: ContentBlock[];
  locations?: JsonValue[];
  rawInput?: JsonValue;
  rawOutput?: JsonValue;
}

export interface ToolCallUpdate {
  toolCallId: string;
  status?: ToolCallStatus;
  rawOutput?: JsonValue;
}

// ---- Session updates (internally tagged `sessionUpdate`, snake_case) ----

export interface UserMessageChunk {
  sessionUpdate: 'user_message_chunk';
  content: ContentBlock;
}

export interface AgentMessageChunk {
  sessionUpdate: 'agent_message_chunk';
  content: ContentBlock;
}

export interface AgentThoughtChunk {
  sessionUpdate: 'agent_thought_chunk';
  content: ContentBlock;
}

export interface ToolCallInitiated {
  sessionUpdate: 'tool_call';
  toolCallId: string;
  name?: string;
  title?: string;
  status: ToolCallStatus;
  rawInput?: JsonValue;
}

export interface ToolCallUpdated {
  sessionUpdate: 'tool_call_update';
  toolCallId: string;
  status?: ToolCallStatus;
  rawOutput?: JsonValue;
}

export interface PlanEntry {
  id: string;
  title?: string;
  description?: string;
  status?: string;
  isActive?: boolean;
}

export interface PlanUpdated {
  sessionUpdate: 'plan';
  entries: PlanEntry[];
}

export interface CurrentModeUpdated {
  sessionUpdate: 'current_mode_update';
  currentModeId: string;
}

export interface SessionInfo {
  sessionId: string;
  title?: string;
  cwd?: string;
  lastUpdatedAt?: string;
  createdAt?: string;
  [key: string]: JsonValue | undefined;
}

export interface SessionInfoUpdated {
  sessionUpdate: 'session_info_update';
  session: SessionInfo;
  _meta?: JsonValue;
}

export interface ConfigOptionUpdated {
  sessionUpdate: 'config_option_update';
  configOptions: JsonValue[];
}

/** Unknown/future update kinds are kept verbatim. */
export interface UnknownUpdate {
  sessionUpdate: string;
  [key: string]: JsonValue | undefined;
}

export type SessionUpdate =
  | UserMessageChunk
  | AgentMessageChunk
  | AgentThoughtChunk
  | ToolCallInitiated
  | ToolCallUpdated
  | PlanUpdated
  | CurrentModeUpdated
  | SessionInfoUpdated
  | ConfigOptionUpdated
  | UnknownUpdate;

export interface SessionUpdateNotification {
  jsonrpc: '2.0';
  method: 'session/update';
  params: {
    sessionId: string;
    update: SessionUpdate;
    _meta?: JsonValue;
  };
}

// ---- Permission requests (session/request_permission, server→client) ----

export type PermissionOptionKind =
  | 'allow_once'
  | 'allow_always'
  | 'reject_once'
  | 'reject_always'
  | (string & {});

export interface PermissionOption {
  optionId: string;
  name: string;
  kind: PermissionOptionKind;
  _meta?: JsonValue;
}

export interface RequestPermissionToolCall {
  toolCallId: string;
  status?: ToolCallStatus;
  title?: string;
  kind?: string;
  content?: ContentBlock[];
  rawInput?: JsonValue;
  rawOutput?: JsonValue;
  _meta?: JsonValue;
}

/** Params of the `session/request_permission` request the server sends us. */
export interface RequestPermissionParams {
  sessionId: string;
  toolCall: RequestPermissionToolCall;
  options: PermissionOption[];
  _meta?: JsonValue;
}

/** Result payload we reply with (outcome internally tagged by `outcome`). */
export interface RequestPermissionOutcomeResponse {
  outcome:
    | { outcome: 'selected'; optionId: string }
    | { outcome: 'cancelled' };
}

// ---- Mailbox (sage.local/mailbox/list + mark_read) ----

export interface MailboxAddress {
  sessionId: string;
  label?: string;
}

export type MailboxMessageStatus =
  | 'pending'
  | 'delivered'
  | 'read'
  | 'archived'
  | (string & {});

export interface MailboxMessage {
  id: string;
  from: MailboxAddress;
  to: MailboxAddress;
  subject: string;
  body: string;
  priority: string;
  status: MailboxMessageStatus;
  sentAt: string;
  readAt?: string;
  inReplyTo?: string;
  expiresAt?: string;
}

export interface MailboxListResult {
  messages: MailboxMessage[];
  labels: Record<string, string>;
}

// ---- Request params (camelCase per serde rename_all) ----

export interface InitializeParams {
  protocolVersion: number;
  clientCapabilities?: {
    fs?: { readTextFile?: boolean; writeTextFile?: boolean };
    terminal?: boolean;
  };
  clientInfo?: { name: string; version: string };
  _meta?: JsonValue;
}

export interface AuthenticateParams {
  methodId: string;
  _meta?: JsonValue;
}

export interface NewSessionParams {
  cwd: string;
  mcpServers?: JsonValue[];
  _meta?: JsonValue;
}

export interface PromptParams {
  sessionId: string;
  prompt: ContentBlock[];
  _meta?: JsonValue;
}

export interface NewSessionResult {
  sessionId: string;
  modes?: JsonValue;
  configOptions?: JsonValue[];
  models?: SessionModelState;
}

// ---- Session roster (sage.local/sessions/list + sage.local/sessions/changed) ----

export type RosterActivity =
  | 'working'
  | 'idle'
  | 'needs_input'
  | 'dormant'
  | 'completed'
  | 'dead'
  | (string & {});

export interface RosterOrigin {
  kind: 'local' | 'remote';
  host?: string;
}

export interface RosterEntry {
  sessionId: string;
  title?: string;
  cwd: string;
  isWorktree: boolean;
  modelId?: string;
  reasoningEffort?: string;
  yolo: boolean;
  activity: RosterActivity;
  resident: boolean;
  lastChangeUnixMs: number;
  origin: RosterOrigin;
}

export interface RosterListResult {
  sessions: RosterEntry[];
}

/** Payload of the `sage.local/sessions/changed` broadcast (wire `_`-prefixed). */
export interface RosterChanged {
  upserted: RosterEntry[];
  removed: string[];
}

// ---- Models (session/new and session/load `models` field) ----

export interface ModelInfo {
  modelId: string;
  name: string;
  description?: string;
  _meta?: JsonValue;
}

export interface SessionModelState {
  currentModelId: string;
  availableModels: ModelInfo[];
}
