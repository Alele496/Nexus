// Right-hand detail panel: model selector + metadata for the active session.

import type { ModelInfo, RosterEntry } from '../acp/types';

interface Props {
  models: ModelInfo[];
  currentModelId: string | null;
  session: RosterEntry | null;
  onSwitchModel: (modelId: string) => void;
}

function Row({ label, value }: { label: string; value: string }) {
  return (
    <div>
      <div className="text-[10px] uppercase tracking-wide text-zinc-600">{label}</div>
      <div className="mt-0.5 break-all font-mono text-[12px] text-zinc-300">{value}</div>
    </div>
  );
}

export default function SessionDetail({
  models,
  currentModelId,
  session,
  onSwitchModel,
}: Props) {
  return (
    <aside className="hidden w-64 shrink-0 flex-col border-l border-zinc-800 bg-zinc-950 lg:flex">
      <div className="border-b border-zinc-800 px-3 py-2.5">
        <span className="text-[13px] font-semibold text-zinc-200">会话详情</span>
      </div>
      <div className="flex-1 space-y-4 overflow-y-auto p-3">
        {models.length > 0 && (
          <div>
            <label
              htmlFor="model-select"
              className="text-[10px] uppercase tracking-wide text-zinc-600"
            >
              模型
            </label>
            <select
              id="model-select"
              value={currentModelId ?? ''}
              onChange={(e) => onSwitchModel(e.target.value)}
              className="mt-1 w-full rounded-lg border border-zinc-800 bg-zinc-900 px-2 py-1.5 text-[12px] text-zinc-100 outline-none focus:border-indigo-500/60"
            >
              {models.map((m) => (
                <option key={m.modelId} value={m.modelId}>
                  {m.name}
                </option>
              ))}
            </select>
          </div>
        )}

        {session ? (
          <>
            <Row label="工作目录" value={session.cwd} />
            <Row label="会话 ID" value={session.sessionId} />
            {session.title && <Row label="标题" value={session.title} />}
            <div>
              <div className="text-[10px] uppercase tracking-wide text-zinc-600">状态</div>
              <div className="mt-0.5 text-[12px] text-zinc-300">{session.activity}</div>
            </div>
          </>
        ) : (
          <p className="text-[12px] text-zinc-600">未连接会话</p>
        )}
      </div>
    </aside>
  );
}
