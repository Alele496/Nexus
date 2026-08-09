// Modal shown when the agent requests permission for a sensitive tool call
// (`session/request_permission`, a server→client JSON-RPC request). Picking an
// option replies to that request with the selected option; 取消 replies
// `cancelled`. The turn stays suspended until one or the other happens.

import type { PendingPermission } from '../acp/hooks';
import type { PermissionOption } from '../acp/types';

interface Props {
  request: PendingPermission | null;
  onRespond: (optionId: string | null) => void;
}

const KIND_STYLE: Record<string, string> = {
  allow_once:
    'border-emerald-700/70 bg-emerald-950/60 text-emerald-100 hover:bg-emerald-900/70',
  allow_always:
    'border-emerald-800/50 bg-emerald-950/30 text-emerald-200 hover:bg-emerald-900/50',
  reject_once:
    'border-red-700/70 bg-red-950/60 text-red-100 hover:bg-red-900/70',
  reject_always:
    'border-red-800/50 bg-red-950/30 text-red-200 hover:bg-red-900/50',
};

function formatInput(raw: unknown): string | null {
  if (raw === undefined || raw === null) return null;
  if (typeof raw === 'string') return raw;
  try {
    const s = JSON.stringify(raw, null, 2);
    return s.length > 2000 ? `${s.slice(0, 2000)}\n…` : s;
  } catch {
    return String(raw);
  }
}

function OptionButton({ option, onRespond }: { option: PermissionOption; onRespond: (id: string) => void }) {
  const style = KIND_STYLE[option.kind] ?? KIND_STYLE.reject_once;
  return (
    <button
      onClick={() => onRespond(option.optionId)}
      className={`flex-1 rounded-lg border px-3 py-2 text-left text-[12px] transition-colors ${style}`}
    >
      {option.name}
    </button>
  );
}

export default function ApprovalModal({ request, onRespond }: Props) {
  if (!request) return null;
  const { sessionId, toolCall, options } = request.params;
  const title = toolCall.title || toolCall.kind || toolCall.toolCallId;
  const input = formatInput(toolCall.rawInput);

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 p-4">
      <div className="w-full max-w-lg overflow-hidden rounded-xl border border-zinc-700 bg-zinc-900 shadow-2xl">
        <header className="border-b border-zinc-800 bg-zinc-950/60 px-4 py-3">
          <div className="flex items-center gap-2">
            <span className="h-2 w-2 rounded-full bg-amber-400 animate-pulse" />
            <span className="text-[13px] font-semibold text-zinc-100">需要授权</span>
          </div>
          <div className="mt-1 truncate font-mono text-[11px] text-zinc-500">{title}</div>
        </header>

        <div className="max-h-[50vh] space-y-2.5 overflow-y-auto p-4">
          <div className="grid grid-cols-2 gap-2 text-[11px]">
            <div className="min-w-0">
              <div className="text-[10px] uppercase tracking-wide text-zinc-600">会话</div>
              <div className="truncate font-mono text-zinc-300">{sessionId}</div>
            </div>
            <div className="min-w-0">
              <div className="text-[10px] uppercase tracking-wide text-zinc-600">工具调用</div>
              <div className="truncate font-mono text-zinc-300">{toolCall.toolCallId}</div>
            </div>
          </div>
          {input && (
            <pre className="max-h-56 overflow-auto rounded-lg border border-zinc-800 bg-zinc-950 p-2.5 font-mono text-[11px] leading-relaxed text-zinc-300 whitespace-pre-wrap break-all">
              {input}
            </pre>
          )}
        </div>

        <footer className="flex flex-wrap items-center gap-2 border-t border-zinc-800 bg-zinc-950/60 p-3">
          {options.map((o) => (
            <OptionButton key={o.optionId} option={o} onRespond={onRespond} />
          ))}
          <button
            onClick={() => onRespond(null)}
            className="rounded-lg border border-zinc-700 px-3 py-2 text-[12px] text-zinc-300 hover:border-zinc-500 hover:text-zinc-100"
          >
            取消
          </button>
        </footer>
      </div>
    </div>
  );
}
