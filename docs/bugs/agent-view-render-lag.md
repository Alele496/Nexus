# 性能调查报告：Agent 视图渲染卡顿（5-16fps）+ 后台线程空转

> 状态：待修复 ｜ 报告日期：2026-08-04 ｜ 报告人：agent-dev 会话（技术主管侧）
> 涉及 crate：`nexus-pager`、`nexus-pager-render`
> 复现环境：Windows 11 / Windows Terminal，`nexus 0.2.105 (1e69ead)`（安装版）与 `agent-dev` HEAD

## 用户可感知的现象

- Agent 视图（会话界面）：**5-16fps**，帧时间 p50 90-183ms，p95 至 235ms
- Dashboard：**60fps**，切换过去帧率缓慢回升
- 空会话（无内容）同样慢（~16fps）——**与内容量无关，是固定每帧开销**
- 流式回复、鼠标悬停时更卡；后台 cargo 编译（rustc）高峰期雪上加霜
- 主线程（事件循环）在 agent 视图下 **100% CPU**，dashboard 下仅 4%

## 已确认的事实（实验证据）

1. **fps HUD 语义**：`views/fps_hud.rs` 文档明确 —— "fps 是渲染吞吐（1/平均帧成本），空闲 UI 不绘制"。所以 `fps:10` ⟺ **每帧绘制真实花费 ~100ms**，且界面持续被触发重绘。
2. **视图对照实验**（受控采样，同一进程同一机器）：
   | 视图 | 主线程 CPU |
   |------|-----------|
   | Dashboard | 4%（5 秒 203ms） |
   | Agent 视图 | **100%**（5 秒 5000ms） |
3. **鼠标离开窗口后主线程归零**：主线程从 100% → 0%。证明"鼠标悬停 scrollback"是持续重绘的主触发器之一（日志中 `scrollback_mouse_moved` 事件 229 条）。
4. **流式 token 洪水**：DEBUG 日志显示 agent 流式响应时每 token 一个 `agent_thought_chunk`（1-14 字节），本会话累计 `seq=33万+`；每个 chunk 到达 → 触发重绘。日志统计：一个响应分钟内 1543 条 chunk。
5. **后台线程空转**：重启后新进程中线程（如 29256）**从进程启动起持续 100% CPU**（累计 352 秒+，两次 3-5 秒采样均 101%），与视图、鼠标无关，日志安静（3 秒仅 1KB）——独立于渲染问题的第二个 bug。
6. **重放不是主因**：超大会话（33 万事件）的 transcript 重放实测 2.25 秒完成（`session.load_session_replay elapsed_us=2255449`），一次性成本。

## 已排除的嫌疑（源码验证）

| 嫌疑 | 结论 |
|------|------|
| 全帧重绘 | 排除——ratatui 差异渲染，只输出变更单元格（`draw.rs`） |
| 布局每帧全量重建 | 排除——`prepare_layout` 流式快路径 O(1) 补丁 + LayoutCache |
| markdown 每 token 全量解析 | 排除——`streaming.rs` 只重渲未冻结尾部 |
| 写终端阻塞 | 排除——TermWriter 无界通道 + 后台写线程，`flush()` 不阻塞 |
| 渲染路径同步 git | 排除——`git_info.rs` 离线程 + TTL 缓存（`CWD_GIT_REFRESH_TTL`） |
| 内存释放钩子 | 排除——`memory_release.rs` 边沿触发，非每帧 |
| MCP 卡死重连 | 排除——MCP 初始化一次性完成（日志确认） |

## 根因假设（高置信度，待精确定位）

Agent 视图**每帧绘制成本 60-100ms**（dashboard 仅 16ms），叠加**高频率重绘触发**（鼠标悬停 229 事件/时段、流式每 token 一帧、动画 tick），导致主线程 100% + 5-16fps。空会话也慢说明存在**固定每帧开销**（与内容量无关）。

具体热点未定位（需要堆栈采样）。候选方向：
- scrollback 鼠标悬停命中测试路径（若"鼠标离开 + 打字"实验显示 fps 回升，则悬停路径为主）
- scrollback 绘制/测量路径的固定开销（若打字也慢，则绘制路径为主）

## 修复建议（按优先级）

1. **降低 agent 视图每帧绘制成本**（60-100ms → <16ms）：定位热点后针对性优化；目标是与 dashboard 同级的固定开销
2. **悬停重绘节流**：鼠标悬停状态变化不立即全帧重绘，按帧节流（已有 `request_throttled` 机制可复用）
3. **流式重绘合并**：token chunk 到达不逐 token 重绘，按 16-33ms 帧节奏合并（当前似乎每个事件都触发绘制）
4. **后台线程空转**：定位 29256 类线程（进程启动即 100%）——需要堆栈采样，见下方"定位手段"

## 定位手段（任选其一）

- **提权采样**：管理员终端运行 `xperf -start capture -on PROC_THREAD+LOADER+Profile -stackwalk Profile`，agent 视图停 15 秒后 `xperf -stop -d cpu.etl`，分析主线程与空转线程堆栈（安装版无 PDB，需 `nexus.exe` 符号或改用 debug 构建）
- **debug 构建**：`nexus/crates/codegen/nexus-bin` debug 构建（带符号），配合 xperf 或 VS 分析器采样
- **临时插桩**：在 `AppView::draw` / `draw_frame` 内加分段计时（绘制 / diff / 悬停命中测试），运行后输出各段耗时

## 备注

- 与 Web UI 的关系：渲染架构本身合格（差异渲染/缓存/增量），问题是个别热点 + 触发频率，属可修复 bug，不建议为此重写 Web UI
- 用户机器 12 核，dashboard 60fps 证明机器性能充足
- 后台 cargo 编译（rustc 全核）会放大卡顿，修复前可先错峰编译缓解
