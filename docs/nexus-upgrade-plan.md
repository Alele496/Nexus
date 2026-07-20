# Nexus 功能升级 — 实施计划

> 目标：将舰队管理融入 Nexus TUI，移植 CCB 核心能力，最终将 Nexus 打造为 Agent 中台。
> 创建日期：2026-07-20
> 当前版本：v0.2.105

---

## 总体规划

```
Phase 1: 项目工作区管理（单仓库 + Fleet 多仓库）
    ↓
Phase 2: Workflow Engine 工作流引擎
    ↓
Phase 3: Coordinator 多 Agent 协调
    ↓
Phase 4: 多 Provider 兼容层
    ↓
Phase 5: 中台化（ACP 桥接 + Remote Control + Daemon）
```

> **注意：内置 Cron 定时任务 Nexus 已完整实现**（`SchedulerActor` + `SchedulerCreate/Delete/List` 工具 + TUI 任务面板），不再列入计划。

---

## Phase 1: 项目工作区管理

**目标**：让 Nexus 原生支持两种项目管理模式——
- **单仓库模式**（默认）：自动识别当前 Git 仓库，加载项目配置和上下文，适合日常开发
- **Fleet 舰队模式**：多仓库编排，健康巡检，项目间切换，适合跨仓库工作流

单仓库模式轻量、不烧 Token，是 90% 用户的日常入口；Fleet 模式在需要跨项目管理时按需启用。

---

### 1A. 单仓库模式（默认）

#### 1A.1 项目自动识别

| # | 任务 | 文件 | 说明 |
|---|------|------|------|
| 1A.1.1 | Git 仓库根目录检测 | `nexus-workspace/src/` (扩展) | 从当前目录向上查找 `.git`，确定项目根目录 |
| 1A.1.2 | 项目名称推断 | 同上 | 从目录名或 `Cargo.toml`/`package.json`/`pyproject.toml` 推断项目名 |
| 1A.1.3 | 项目类型检测 | 同上 | 根据文件特征检测 Rust/JS/Python/Go 等项目类型 |
| 1A.1.4 | 启动时自动加载 | `nexus-bin/src/main.rs` | 启动时自动检测项目并注入上下文 |

#### 1A.2 项目配置

| # | 任务 | 文件 | 说明 |
|---|------|------|------|
| 1A.2.1 | `.sage/` 项目配置目录 | `nexus-workspace/src/` | 每个项目根目录下 `.sage/` 存放项目级配置 |
| 1A.2.2 | `/project init` 命令 | `nexus-pager/src/slash/commands/project.rs` | 在项目根目录初始化 `.sage/`（创建 config.toml、AGENTS.md 模板） |
| 1A.2.3 | 项目级 config.toml | 同上 | 项目专属模型、工具白名单、Hooks 配置，覆盖全局设置 |
| 1A.2.4 | `/project info` 命令 | 同上 | 展示当前项目信息：名称、类型、路径、Git 分支、未提交变更数 |

#### 1A.3 上下文注入

| # | 任务 | 文件 | 说明 |
|---|------|------|------|
| 1A.3.1 | 项目 AGENTS.md 加载 | `nexus-agent/src/prompt/context.rs` | 检测项目根目录 `AGENTS.md`，注入到 System Prompt |
| 1A.3.2 | 项目结构摘要 | 同上 | 自动生成项目目录树摘要（过滤 node_modules/target/.git） |
| 1A.3.3 | Git 状态上下文 | 同上 | 当前分支、未提交文件列表、最近 3 条 commit |
| 1A.3.4 | 依赖文件上下文 | 同上 | `Cargo.toml`/`package.json` 关键依赖摘要 |

#### 1A.4 项目导航

| # | 任务 | 文件 | 说明 |
|---|------|------|------|
| 1A.4.1 | `/cd <path>` 命令 | `nexus-pager/src/slash/commands/project.rs` | 切换工作目录，自动重新检测项目 |
| 1A.4.2 | 最近项目列表 | `nexus-shell/src/session/` | 记录最近打开的项目路径，下次启动快速恢复 |
| 1A.4.3 | 欢迎界面项目入口 | `views/welcome/mod.rs` | 欢迎屏幕显示"当前项目"卡片 + 最近项目列表 |

---

### 1B. Fleet 舰队模式（多仓库编排）

#### 1B.1 Fleet 数据层

| # | 任务 | 文件 | 说明 |
|---|------|------|------|
| 1B.1.1 | 定义 `FleetRegistry` 数据结构 | `nexus-fleet-types/` (新 crate) | 从 `fleet-registry.json` 解析，包含项目名、路径、层级、依赖、健康状态 |
| 1B.1.2 | 实现舰队配置加载 | 同上 | 从 `$NEXUS_HOME/fleet-registry.json` 或项目本地 `fleet-registry.json` 加载 |
| 1B.1.3 | 实现健康检查逻辑 | 同上 | 把 `health-check.sh` 的逻辑翻译成 Rust：git status、分支状态、未提交变更数、ahead/behind |
| 1B.1.4 | 单元测试 | `nexus-fleet-types/src/__tests__/` | 覆盖注册表解析、健康检查各场景 |

#### 1B.2 Fleet 命令

| # | 任务 | 文件 | 说明 |
|---|------|------|------|
| 1B.2.1 | `/fleet` — 舰队仪表盘 | `nexus-pager/src/slash/commands/fleet.rs` | 展示所有注册项目：名称、层级、git 状态、最后活动时间 |
| 1B.2.2 | `/fleet switch <project>` | 同上 | 切换工作目录到指定项目，自动加载该项目的 AGENTS.md |
| 1B.2.3 | `/fleet health` | 同上 | 执行全舰队健康巡检，彩色输出通过/警告/失败 |
| 1B.2.4 | `/fleet add` / `/fleet remove` | 同上 | 从 TUI 内添加/移除注册项目 |
| 1B.2.5 | 注册命令到 `builtin_commands()` | `mod.rs` | 添加 `Arc::new(fleet::FleetCommand)` |

#### 1B.3 Fleet 欢迎界面

| # | 任务 | 文件 | 说明 |
|---|------|------|------|
| 1B.3.1 | 欢迎屏幕增加"舰队项目"入口 | `views/welcome/mod.rs` | 在菜单中添加舰队快捷入口 |
| 1B.3.2 | 舰队项目选择器 | `views/welcome/` 或新组件 | 类似会话选择器，列出舰队项目并支持快速切换 |
| 1B.3.3 | 项目状态图标 | 同上 | 用颜色/图标标示各项目健康状态 |

#### 1B.4 Fleet 上下文注入

| # | 任务 | 文件 | 说明 |
|---|------|------|------|
| 1B.4.1 | Fleet 上下文注入到 System Prompt | `nexus-agent/src/prompt/context.rs` | 当会话在舰队项目下时，自动注入舰队上下文（依赖项目、端口、环境变量） |
| 1B.4.2 | 跨项目依赖感知 | 同上 | 当操作涉及其他项目时，Agent 自动查阅 fleet-registry 获取路径/端口 |

---

## Phase 2: Workflow Engine 工作流引擎

**目标**：支持结构化的多步骤工作流定义和执行。用户/Agent 可以定义 Workflow 模板，按步骤自动执行。

参考：CCB `packages/workflow-engine/`

### 2.1 Workflow 定义

| # | 任务 | 文件 | 说明 |
|---|------|------|------|
| 2.1.1 | Workflow DSL 定义 | `nexus-workflow/` (新 crate) | YAML/TOML 格式：`name`、`steps[]`、`on_error`、`timeout` |
| 2.1.2 | Workflow 模板加载 | 同上 | 从 `$NEXUS_HOME/workflows/` 或项目 `.sage/workflows/` 加载 |
| 2.1.3 | 步骤类型定义 | 同上 | Agent 任务、Bash 命令、用户确认、条件分支、并行执行 |

### 2.2 Workflow 执行

| # | 任务 | 文件 | 说明 |
|---|------|------|------|
| 2.2.1 | Workflow 执行引擎 | 同上 | 按 DAG 依赖顺序执行步骤，支持并行和串行 |
| 2.2.2 | 步骤状态追踪 | 同上 | pending → running → success/failed/skipped |
| 2.2.3 | 错误恢复 | 同上 | on_error: stop/retry/skip/rollback |
| 2.2.4 | Workflow 暂停/恢复 | 同上 | 支持在步骤间暂停，等待用户确认后继续 |

### 2.3 Workflow 工具

| # | 任务 | 文件 | 说明 |
|---|------|------|------|
| 2.3.1 | `WorkflowRun` 工具 | `nexus-tools/src/implementations/workflow/` | Agent 可调用，按名称启动一个 Workflow |
| 2.3.2 | `/workflow` slash 命令 | `nexus-pager/src/slash/commands/workflow.rs` | 列出可用 Workflow，手动触发 |

---

## Phase 3: Coordinator 多 Agent 协调

**目标**：解除当前"深度限制 1"的硬约束，实现可控的多层 Agent 嵌套。一个 Coordinator 可以调度多个 Worker 并行工作。

参考：CCB `src/coordinator/coordinatorMode.ts`、`workerAgent.ts`

### 3.1 核心改造

| # | 任务 | 文件 | 说明 |
|---|------|------|------|
| 3.1.1 | 子 Agent 深度限制改为可配置 | `nexus-shell/src/session/` | 新增 `max_subagent_depth` 配置项，默认 1，可设为 2-5 |
| 3.1.2 | Coordinator Agent 类型 | `nexus-agent/src/` | 新增 `coordinator` subagent_type，拥有 spawn 子 Agent 的权限 |
| 3.1.3 | Worker Agent 类型 | 同上 | 新增 `worker` subagent_type，执行具体任务，不能 spawn |

### 3.2 Coordinator 模式

| # | 任务 | 文件 | 说明 |
|---|------|------|------|
| 3.2.1 | `/coordinator` 命令 | `nexus-pager/src/slash/commands/coordinator.rs` | 切换 Coordinator 模式，进入后自动使用 coordinator agent |
| 3.2.2 | 任务分配逻辑 | coordinator 系统 prompt | 自动分析任务，拆分为子任务分发给 Worker |
| 3.2.3 | Worker 结果汇总 | 同上 | 收集所有 Worker 输出，汇总为统一报告 |
| 3.2.4 | Worker 间通信 | 文件/内存邮箱 | 参考 CCB 的 teammateMailbox |

### 3.3 安全约束

| # | 任务 | 文件 | 说明 |
|---|------|------|------|
| 3.3.1 | Worker 权限隔离 | 同上 | Worker 默认 read-only + execute，写权限需显式授权 |
| 3.3.2 | 超时控制 | 同上 | 每个 Worker 有独立超时，超时后自动终止 |
| 3.3.3 | 螺旋检测 | 同上 | 检测 Worker 是否陷入修复-失败的无限循环 |

---

## Phase 4: 多 Provider 兼容层

**目标**：参考 CCB 的多 API 兼容层，让 Nexus 支持切换不同模型提供商（OpenAI、Gemini 等），不局限于 DeepSeek。

参考：CCB `src/services/api/openai/`、`gemini/`、`grok/`

### 4.1 Provider 抽象

| # | 任务 | 文件 | 说明 |
|---|------|------|------|
| 4.1.1 | Provider trait 定义 | `nexus-sampling-types/` 或新 crate | 统一接口：`chat_completion()`、`list_models()`、`stream()` |
| 4.1.2 | DeepSeek Provider（重构） | 同上 | 把现有 DeepSeek 调用封装为 Provider 实现 |
| 4.1.3 | OpenAI Provider | 同上 | OpenAI Chat Completions 协议适配 |
| 4.1.4 | Gemini Provider | 同上 | Google Gemini API 适配 |

### 4.2 配置和切换

| # | 任务 | 文件 | 说明 |
|---|------|------|------|
| 4.2.1 | 多 Provider 配置格式 | `config.toml` | 每个 Provider 独立 `[provider.xxx]` 段 |
| 4.2.2 | `/provider` 命令 | slash 命令 | 查看/切换当前 Provider |
| 4.2.3 | 模型列表自动发现 | Provider trait | 从各 API 端点拉取可用模型列表 |

---

## Phase 5: 中台化

**目标**：Nexus 不再只是一个终端工具，而是一个 Agent 运行时平台。CCB 或其他前端可以通过 ACP/MCP 接入，把 Nexus 当作后端 Agent 引擎。

### 5.1 ACP 桥接

| # | 任务 | 文件 | 说明 |
|---|------|------|------|
| 5.1.1 | Nexus ACP Server 模式 | `nexus-acp-lib/` 扩展 | 让 Nexus 作为 ACP Server 运行，接受外部 Agent 连接 |
| 5.1.2 | CCB ↔ Nexus 桥接 | 桥接配置 | CCB 通过 ACP 发送任务给 Nexus，Nexus 执行后返回结果 |
| 5.1.3 | 工具透传 | 同上 | CCB 可以调用 Nexus 的所有工具（Sandbox、Memory、舰队管理等） |

### 5.2 Remote Control Server

| # | 任务 | 文件 | 说明 |
|---|------|------|------|
| 5.2.1 | Web UI 控制面板 | 独立前端项目 | 参考 CCB `packages/remote-control-server/`，React + Vite |
| 5.2.2 | REST API | Nexus 内嵌 HTTP Server | 会话管理、任务提交、状态查询 |
| 5.2.3 | WebSocket 实时推送 | 同上 | Agent 输出流式推送到 Web UI |
| 5.2.4 | Docker 部署 | Dockerfile | 一键部署 Remote Control Server |

### 5.3 Daemon 模式

| # | 任务 | 文件 | 说明 |
|---|------|------|------|
| 5.3.1 | `nexus daemon start/stop/status` | `nexus-bin/src/` | 长驻后台进程管理 |
| 5.3.2 | 后台会话（BG Sessions） | `nexus-shell/src/` | 不占 TUI 的后台 Agent 执行 |
| 5.3.3 | 自动重启 | Daemon 监督者 | Worker 崩溃后自动重启，指数退避 |

---

## 进度追踪

| Phase | 状态 | 开始日期 | 完成日期 |
|-------|------|---------|---------|
| Phase 1: 工作区管理 | 🟢 核心完成 | 2026-07-20 | - |
| Phase 2: Workflow Engine | 🟢 核心完成 | 2026-07-20 | - |
| Phase 3: Coordinator | 🟢 核心完成 | 2026-07-20 | - |
| Phase 4: 多 Provider | ⬜ 待开始 | - | - |
| Phase 5: 中台化 | ⬜ 待开始 | - | - |

### Phase 1 完成清单

| 子任务 | 状态 | 说明 |
|--------|------|------|
| `nexus-project` crate | ✅ | 项目检测、类型推断、.sage/ 初始化、上下文生成 |
| `nexus-fleet-types` crate | ✅ | FleetRegistry 解析、健康检查（git2）、单元测试 |
| `/project info\|init` 命令 | ✅ | TUI 内置 slash 命令，查看/初始化项目 |
| `/fleet dashboard\|health\|list\|switch` 命令 | ✅ | TUI 内置 slash 命令，舰队管理 |
| 项目上下文注入 System Prompt | ⏳ 后续 | 需要修改 PromptContext 构建流程 |
| 欢迎界面项目入口 | ⏳ 后续 | 需要修改 welcome view 组件 |

### Phase 2 完成清单

| 子任务 | 状态 | 说明 |
|--------|------|------|
| `nexus-workflow` crate | ✅ | YAML DSL 定义、DAG 拓扑排序、循环检测、状态追踪 |
| `/workflow list\|run <name>\|status` 命令 | ✅ | TUI 内置 slash 命令 |
| 示例工作流 | ✅ | review-pipeline.yml（审查+推送）、quick-deploy.yml（快速部署） |
| WorkflowRun Agent 工具 | ⏳ 后续 | 需要在 nexus-tools 中注册，让 Agent 可调用 |

### Phase 3 完成清单

| 子任务 | 状态 | 说明 |
|--------|------|------|
| `MaxSubagentDepthResource` 资源 | ✅ | 替代硬编码 `MAX_SUBAGENT_DEPTH = 1`，默认 1，可动态配置 |
| `max_subagent_depth` 贯穿层 | ✅ | ToolContext → SubagentSpawnContext → AgentRebuildSpec → ToolBridge |
| `SubagentDepthCounter` 检查改为动态 | ✅ | TaskTool::invoke() 和 handle_request.rs 都从资源读取 max depth |
| `/coordinator start\|stop\|status` 命令 | ✅ | TUI 内置 slash 命令，注入 Coordinator 系统提示 |
| Coordinator 角色 prompt | ✅ | 任务拆分 → Worker 并行 → 结果汇总的工作流指令 |
| Workflow serde 修复 | ✅ | StepType 的 `#[serde(flatten)]` 修复，15/15 测试通过 |

---

## 已有功能（无需重新实现）

| 功能 | 状态 | 说明 |
|------|------|------|
| 内置 Cron 定时任务 | ✅ 已完成 | `SchedulerActor` + `SchedulerCreate/Delete/List` 工具 + TUI `/tasks` 面板 |
| MCP 协议客户端 | ✅ 已完成 | HTTP、stdio、ACP bridge 三种传输 |
| Hooks 生命周期 | ✅ 已完成 | 14 种事件，blocking/non-blocking |
| 多 Agent 角色 | ✅ 已完成 | Developer/Reviewer/Operator/Advisor + 3 种 Personas |

---

## 依赖关系

```
Phase 1 (工作区管理) ──→ Phase 3 (Coordinator) ──→ Phase 5 (中台化)
                              ↗
Phase 2 (Workflow Engine) ────┘
Phase 4 (多 Provider) ──→ 独立，可随时插入
```

Phase 1、2、4 相对独立，可以并行推进。Phase 3 依赖 Phase 1 的工作区基础设施。Phase 5 依赖 Phase 3 的 Coordinator 能力。
