// Settings panel (`x.ai/getApiKey|setApiKey|auth/info|auth/logout|commands/list`
// + `x.ai/internal/reload_models|reload_skills`). Data is fetched on open and
// held locally so account edits don't re-render the whole app.

import { useEffect, useMemo, useState } from 'react';

import type { AuthInfo, AvailableCommand } from '../acp/types';

interface Props {
  open: boolean;
  onClose: () => void;
  loadApiKey: () => Promise<string | null>;
  saveApiKey: (key: string) => Promise<void>;
  loadAuthInfo: () => Promise<AuthInfo | null>;
  logout: () => Promise<void>;
  loadCommands: () => Promise<AvailableCommand[]>;
  reloadModels: () => Promise<boolean>;
  reloadSkills: () => Promise<boolean>;
}

function maskKey(key: string): string {
  if (key.length <= 8) return '••••••••';
  return `••••••••${key.slice(-4)}`;
}

export default function SettingsPanel({
  open,
  onClose,
  loadApiKey,
  saveApiKey,
  loadAuthInfo,
  logout,
  loadCommands,
  reloadModels,
  reloadSkills,
}: Props) {
  const [authInfo, setAuthInfo] = useState<AuthInfo | null>(null);
  const [apiKey, setApiKey] = useState<string | null>(null);
  const [keyInput, setKeyInput] = useState('');
  const [keyBusy, setKeyBusy] = useState(false);
  const [keyMsg, setKeyMsg] = useState<string | null>(null);
  const [confirmLogout, setConfirmLogout] = useState(false);
  const [commands, setCommands] = useState<AvailableCommand[]>([]);
  const [cmdQuery, setCmdQuery] = useState('');
  const [cmdBusy, setCmdBusy] = useState(false);
  const [reloadBusy, setReloadBusy] = useState<'models' | 'skills' | null>(null);
  const [reloadMsg, setReloadMsg] = useState<string | null>(null);

  useEffect(() => {
    if (!open) return;
    setKeyMsg(null);
    setConfirmLogout(false);
    void (async () => {
      setAuthInfo(await loadAuthInfo());
      setApiKey(await loadApiKey());
    })();
  }, [open, loadAuthInfo, loadApiKey]);

  const filteredCommands = useMemo(() => {
    const q = cmdQuery.trim().toLowerCase();
    if (!q) return commands;
    return commands.filter(
      (c) =>
        c.name.toLowerCase().includes(q) ||
        c.description.toLowerCase().includes(q),
    );
  }, [commands, cmdQuery]);

  const fetchCommands = async () => {
    setCmdBusy(true);
    try {
      setCommands(await loadCommands());
    } finally {
      setCmdBusy(false);
    }
  };

  useEffect(() => {
    if (open) void fetchCommands();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [open]);

  if (!open) return null;

  const saveKey = async () => {
    const key = keyInput.trim();
    if (!key) return;
    setKeyBusy(true);
    setKeyMsg(null);
    await saveApiKey(key);
    setKeyBusy(false);
    setApiKey(key);
    setKeyInput('');
    setKeyMsg('已保存');
  };

  const clearKey = async () => {
    setKeyBusy(true);
    setKeyMsg(null);
    await saveApiKey('');
    setKeyBusy(false);
    setApiKey(null);
    setKeyMsg('已清除');
  };

  const doLogout = async () => {
    setConfirmLogout(false);
    await logout();
    setAuthInfo(await loadAuthInfo());
    setApiKey(await loadApiKey());
  };

  const doReload = async (kind: 'models' | 'skills') => {
    setReloadBusy(kind);
    setReloadMsg(null);
    const ok = kind === 'models' ? await reloadModels() : await reloadSkills();
    setReloadBusy(null);
    setReloadMsg(ok ? '已重载' : '重载失败');
    window.setTimeout(() => setReloadMsg(null), 2500);
  };

  const sectionTitle = 'mb-2 text-[11px] font-semibold uppercase tracking-wide text-zinc-500';

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 p-4">
      <div className="flex h-[70vh] w-full max-w-2xl flex-col overflow-hidden rounded-xl border border-zinc-700 bg-zinc-900 shadow-2xl">
        <header className="flex items-center gap-2 border-b border-zinc-800 bg-zinc-950/60 px-4 py-3">
          <span className="text-[13px] font-semibold text-zinc-100">设置</span>
          <span className="text-[11px] text-zinc-600">账户 · 命令 · 通用</span>
          <button
            onClick={onClose}
            className="ml-auto flex h-6 w-6 items-center justify-center rounded-md border border-zinc-700 text-zinc-300 hover:border-zinc-500 hover:text-zinc-100"
            title="关闭"
          >
            <svg className="h-3.5 w-3.5" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}>
              <path strokeLinecap="round" d="M6 6l12 12M18 6L6 18" />
            </svg>
          </button>
        </header>

        <div className="flex-1 space-y-5 overflow-y-auto p-4">
          {/* 账户 */}
          <section>
            <h3 className={sectionTitle}>账户</h3>
            <div className="space-y-3 rounded-lg border border-zinc-800 bg-zinc-950/40 p-3">
              {authInfo?.email ? (
                <div className="flex items-center gap-2.5">
                  {authInfo.profileImageUrl && (
                    <img
                      src={authInfo.profileImageUrl}
                      alt=""
                      className="h-8 w-8 rounded-full border border-zinc-700"
                    />
                  )}
                  <div className="min-w-0">
                    <p className="truncate text-[13px] font-medium text-zinc-100">
                      {[authInfo.firstName, authInfo.lastName].filter(Boolean).join(' ') ||
                        authInfo.email}
                    </p>
                    <p className="truncate font-mono text-[11px] text-zinc-500">
                      {authInfo.email}
                      {authInfo.teamName ? ` · ${authInfo.teamName}` : ''}
                    </p>
                  </div>
                  <span className="ml-auto shrink-0 rounded bg-zinc-800/80 px-1.5 py-px font-mono text-[10px] text-zinc-400">
                    {authInfo.methodId || 'account'}
                  </span>
                </div>
              ) : (
                <p className="text-[12px] text-zinc-500">
                  未登录（使用 API Key 或运行 <code className="font-mono text-zinc-400">sage login</code> 登录）
                </p>
              )}

              <div className="flex items-center gap-2 border-t border-zinc-800/70 pt-2.5">
                <span className="text-[12px] text-zinc-400">API Key</span>
                <span className="font-mono text-[12px] text-zinc-200">
                  {apiKey ? maskKey(apiKey) : '未设置'}
                </span>
                {apiKey && (
                  <button
                    onClick={clearKey}
                    disabled={keyBusy}
                    className="ml-auto rounded-md border border-zinc-700 px-2 py-1 text-[11px] text-zinc-300 hover:border-zinc-500 hover:text-zinc-100 disabled:opacity-50"
                  >
                    清除
                  </button>
                )}
              </div>
              <div className="flex items-center gap-2">
                <input
                  type="password"
                  value={keyInput}
                  onChange={(e) => setKeyInput(e.target.value)}
                  onKeyDown={(e) => e.key === 'Enter' && void saveKey()}
                  placeholder="粘贴新 API Key…"
                  autoComplete="off"
                  className="min-w-0 flex-1 rounded-md border border-zinc-700 bg-zinc-900 px-2.5 py-1.5 text-[12px] text-zinc-100 placeholder-zinc-600 focus:border-indigo-500/60 focus:outline-none"
                />
                <button
                  onClick={saveKey}
                  disabled={keyBusy || !keyInput.trim()}
                  className="rounded-md bg-indigo-600 px-3 py-1.5 text-[12px] font-medium text-white hover:bg-indigo-500 disabled:opacity-40"
                >
                  保存
                </button>
              </div>
              {keyMsg && <p className="text-[11px] text-emerald-400">{keyMsg}</p>}

              {authInfo?.email && (
                <div className="border-t border-zinc-800/70 pt-2.5">
                  {confirmLogout ? (
                    <div className="flex items-center gap-2">
                      <span className="text-[12px] text-red-300">退出登录？</span>
                      <button
                        onClick={doLogout}
                        className="rounded-md bg-red-600 px-2.5 py-1 text-[11px] font-medium text-white hover:bg-red-500"
                      >
                        退出
                      </button>
                      <button
                        onClick={() => setConfirmLogout(false)}
                        className="rounded-md border border-zinc-700 px-2.5 py-1 text-[11px] text-zinc-300 hover:border-zinc-500"
                      >
                        取消
                      </button>
                    </div>
                  ) : (
                    <button
                      onClick={() => setConfirmLogout(true)}
                      className="rounded-md border border-zinc-700 px-2.5 py-1 text-[11px] text-zinc-300 hover:border-red-500/60 hover:text-red-300"
                    >
                      退出登录
                    </button>
                  )}
                </div>
              )}
            </div>
          </section>

          {/* 命令 */}
          <section>
            <div className="mb-2 flex items-center gap-2">
              <h3 className="text-[11px] font-semibold uppercase tracking-wide text-zinc-500">
                命令
              </h3>
              <span className="text-[11px] text-zinc-600">{commands.length}</span>
              <button
                onClick={fetchCommands}
                disabled={cmdBusy}
                className="ml-auto rounded-md border border-zinc-700 px-2 py-0.5 text-[11px] text-zinc-400 hover:border-zinc-500 hover:text-zinc-200 disabled:opacity-50"
              >
                {cmdBusy ? '加载中…' : '刷新'}
              </button>
            </div>
            <input
              value={cmdQuery}
              onChange={(e) => setCmdQuery(e.target.value)}
              placeholder="过滤命令…"
              className="mb-2 w-full rounded-md border border-zinc-800 bg-zinc-950/60 px-2.5 py-1.5 text-[12px] text-zinc-200 placeholder-zinc-600 focus:border-indigo-500/60 focus:outline-none"
            />
            <div className="max-h-52 space-y-1 overflow-y-auto rounded-lg border border-zinc-800 bg-zinc-950/40 p-2">
              {filteredCommands.length === 0 && (
                <p className="px-2 py-6 text-center text-[12px] text-zinc-600">暂无命令</p>
              )}
              {filteredCommands.map((c) => (
                <div key={c.name} className="flex items-start gap-2 rounded-md px-2 py-1 hover:bg-zinc-800/50">
                  <span className="shrink-0 font-mono text-[12px] text-indigo-300">/{c.name}</span>
                  <span className="min-w-0 flex-1 truncate text-[12px] text-zinc-400">
                    {c.description}
                  </span>
                </div>
              ))}
            </div>
          </section>

          {/* 通用 */}
          <section>
            <h3 className={sectionTitle}>通用</h3>
            <div className="space-y-2 rounded-lg border border-zinc-800 bg-zinc-950/40 p-3">
              <div className="flex items-center gap-2">
                <span className="text-[12px] text-zinc-300">重载模型目录</span>
                <button
                  onClick={() => void doReload('models')}
                  disabled={reloadBusy !== null}
                  className="ml-auto rounded-md border border-zinc-700 px-2.5 py-1 text-[11px] text-zinc-300 hover:border-zinc-500 hover:text-zinc-100 disabled:opacity-50"
                >
                  {reloadBusy === 'models' ? '重载中…' : '重载'}
                </button>
              </div>
              <div className="flex items-center gap-2 border-t border-zinc-800/70 pt-2">
                <span className="text-[12px] text-zinc-300">重载技能与插件</span>
                <button
                  onClick={() => void doReload('skills')}
                  disabled={reloadBusy !== null}
                  className="ml-auto rounded-md border border-zinc-700 px-2.5 py-1 text-[11px] text-zinc-300 hover:border-zinc-500 hover:text-zinc-100 disabled:opacity-50"
                >
                  {reloadBusy === 'skills' ? '重载中…' : '重载'}
                </button>
              </div>
              {reloadMsg && <p className="text-[11px] text-emerald-400">{reloadMsg}</p>}
            </div>
          </section>
        </div>

        <footer className="flex items-center gap-2 border-t border-zinc-800 bg-zinc-950/60 p-3">
          <button
            onClick={onClose}
            className="ml-auto rounded-lg border border-zinc-700 px-3 py-1.5 text-[12px] text-zinc-300 hover:border-zinc-500 hover:text-zinc-100"
          >
            关闭
          </button>
        </footer>
      </div>
    </div>
  );
}
