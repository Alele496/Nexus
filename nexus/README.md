<div align="center">

# Nexus (`nexus`)

**开源智能 AI 编程助手 — 终端原生 TUI 界面**

[English](#english) | 中文

支持 DeepSeek V4 等主流模型。代码理解、文件编辑、Shell 执行、
多 Agent 协作、Agent Graph 可视化、Agent Client Protocol (ACP) 扩展。

[快速开始](#快速开始) ·
[特性](#特性) ·
[命令参考](#命令参考) ·
[从源码构建](#从源码构建) ·
[配置](#配置) ·
[开发](#开发)

</div>

---

## 快速开始

### 方式一：下载预编译二进制（推荐）

从 [GitHub Releases](https://github.com/Alele496/Nexus/releases) 下载最新版本，直接运行即可，无需安装。

| 平台 | 文件 |
|------|------|
| Windows | `nexus.exe` (129 MB, 免安装便携版) |
| Linux | `nexus` (197 MB) |

### 方式二：包管理器安装 (Windows)

```powershell
# winget（审核通过后可用）
winget install Alele496.Nexus

# Scoop
scoop bucket add nexus https://github.com/Alele496/Nexus
scoop install nexus
```

### 方式三：命令行快速安装

**Windows (PowerShell)：**
```powershell
irm https://raw.githubusercontent.com/Alele496/Nexus/agent-dev/nexus/crates/codegen/nexus-pager/scripts/install.ps1 | iex
```

**Linux / macOS：**
```bash
curl -fsSL https://raw.githubusercontent.com/Alele496/Nexus/agent-dev/nexus/crates/codegen/nexus-pager/scripts/install.sh | bash
```

安装脚本自动下载最新版本，添加到 PATH，并生成 Shell 补全。

### 从源码构建

```sh
# 开发模式
cargo run -p nexus-bin

# Release 构建
cargo build -p nexus-bin --release
```

**环境要求：** Rust (见 `rust-toolchain.toml`) + protoc

首次运行会自动弹出设置向导，引导配置 API Key 和模型。

---

## 特性

### 核心能力

| 功能 | 说明 |
|------|------|
| **Agent Graph** | `/agent-graph` 全屏节点-边关系图，彩色状态、键盘/鼠标导航、实时刷新 |
| **多 Agent 协调** | `/coordinator` 父 Agent 拆解任务，Worker 并行执行，可配置深度和并发 |
| **Workflow 引擎** | `/workflow` DAG 任务编排，串行/并行步骤，进度追踪 |
| **Fleet 舰队** | `/fleet` Agent 编队，共享项目上下文，成员状态查看 |
| **单仓库模式** | `/project` 项目初始化、结构索引、自动上下文注入 |
| **会话管理** | `/fork` 分支会话、`/rewind` 回退、多会话仪表盘 |
| **扩展系统** | MCP 服务器、Hooks 钩子、自定义 Slash 命令 |

### 模型与推理

- **多提供商支持** — DeepSeek V4、OpenAI GPT-5.2、Anthropic Claude 4.7，以及任意 OpenAI 兼容 API（通义千问、Moonshot、本地模型等）
- **DeepSeek V4 Pro** — 原生 1M token 上下文窗口
- **thinking 链式推理** — 支持 `reasoning_effort` (high/max) 映射
- **多模型切换** — `/model <name>` 运行时热切换
- **可扩展** — 架构支持接入更多模型提供商

### 终端交互

- 全屏 TUI 界面，支持极简模式
- 代码高亮、Markdown 渲染、Mermaid 图表
- 权限弹窗、计划模式、对话压缩
- 跨平台: Windows / macOS / Linux

---

## 命令参考

### 会话与导航

| 命令 | 说明 |
|------|------|
| `/new` | 新建会话 |
| `/agent-graph` (`/graph`) | 打开 Agent 关系图 |
| `/dashboard` | 多会话仪表盘 |
| `/cd <path>` | 切换工作目录 |
| `/fork` | 分支当前会话 |
| `/rewind` | 回退到之前轮次 |
| `/compact` | 压缩对话历史 |
| `/resume` | 恢复之前会话 |

### 多 Agent 与编排

| 命令 | 说明 |
|------|------|
| `/coordinator` | 启动 Coordinator 协调 Agent |
| `/workflow run <name>` | 执行工作流 |
| `/fleet create <name>` | 创建舰队 |
| `/fleet join <name>` | 加入舰队 |
| `/fleet list` | 查看舰队状态 |

### 项目与上下文

| 命令 | 说明 |
|------|------|
| `/project init` | 初始化项目上下文 |
| `/project index` | 索引项目结构 |
| `/project context` | 查看当前项目上下文 |
| `/project graph` | 查看项目结构图 |

### 模型与工具

| 命令 | 说明 |
|------|------|
| `/model <name>` | 切换模型 |
| `/effort <level>` | 调整推理深度 (high/max) |
| `/plan` | 进入计划模式 |
| `/mcps` | MCP 服务器状态 |
| `/help` | 浏览全部命令和快捷键 |

---

## 配置

首次运行自动弹出设置向导，或手动编辑 `~/.nexus/config.toml`：

```toml
# DeepSeek 示例
[models]
default = "deepseek-v4-pro"

[model."deepseek-v4-pro"]
model = "deepseek-v4-pro"
name = "DeepSeek V4 Pro"
base_url = "https://api.deepseek.com/v1"
api_backend = "chat_completions"
api_key = "sk-xxx"
context_window = 1000000
supports_reasoning_effort = true

# OpenAI 示例
[model."gpt-5.2"]
model = "gpt-5.2"
name = "GPT-5.2"
base_url = "https://api.openai.com/v1"
api_backend = "chat_completions"
api_key = "sk-xxx"
context_window = 128000

# Anthropic Claude 示例
[model."claude-sonnet-4-6"]
model = "claude-sonnet-4-6"
name = "Claude Sonnet 4.6"
base_url = "https://api.anthropic.com/v1"
api_backend = "messages"
api_key = "sk-ant-xxx"
context_window = 200000

# 任意 OpenAI 兼容 API（通义千问、Ollama 本地模型等）
[model."qwen-plus"]
model = "qwen-plus"
name = "通义千问 Plus"
base_url = "https://dashscope.aliyuncs.com/compatible-mode/v1"
api_backend = "chat_completions"
api_key = "sk-xxx"
context_window = 131072
```

**环境变量：**

| 变量 | 说明 |
|------|------|
| `NEXUS_API_KEY` | API Key |
| `NEXUS_HOME` | 数据目录 (默认 `~/.nexus/`) |

---

## 从源码构建

```sh
# 环境要求
# - Rust 工具链 (见 rust-toolchain.toml)
# - protoc (或 bin/protoc)

cargo run -p nexus-bin          # 开发运行
cargo build -p nexus-bin --release  # Release 构建
```

---

## 仓库结构

```
nexus/
├── crates/codegen/
│   ├── nexus-bin/        # 二进制入口
│   ├── nexus-pager/      # TUI 界面、Agent Graph
│   ├── nexus-shell/      # Agent 运行时、Session 管理
│   ├── nexus-tools/      # 工具实现 (终端、文件、搜索等)
│   ├── nexus-agent/      # Agent 生命周期、系统提示
│   ├── nexus-project/    # 单仓库项目管理
│   ├── nexus-fleet/      # Fleet 舰队管理
│   ├── nexus-workflow/   # Workflow 工作流引擎
│   └── ...               # 其余 80+ crate
├── crates/common/        # 共享叶 crate
├── crates/build/         # 构建辅助
├── third_party/          # 第三方源码 (Mermaid)
└── docs/                 # 文档
```

---

## 开发

```sh
cargo check -p nexus-pager       # 快速验证指定 crate
cargo test --workspace --lib     # 运行全部测试
cargo clippy -p nexus-bin        # Lint 检查
cargo fmt --all                  # 格式化
```

CI 通过 GitHub Actions 自动化运行 (Linux + Windows)。

---

## 许可证

Apache License 2.0 — 详见 [LICENSE](LICENSE)。

第三方代码遵循原始许可证：[THIRD-PARTY-NOTICES](THIRD-PARTY-NOTICES) / [third_party/NOTICE](third_party/NOTICE)。

---

<h2 id="english">English</h2>

Nexus is an open-source AI coding assistant with a native terminal TUI interface. Supports DeepSeek V4, OpenAI, Anthropic Claude, and any OpenAI-compatible API, understands your codebase, edits files, executes shell commands, and orchestrates multiple agents.

**Key Features:**
- **Multi-Provider** — DeepSeek, OpenAI, Anthropic, or any OpenAI-compatible API (Qwen, Moonshot, local models via Ollama)
- **Agent Graph** — Interactive visual graph of agent sessions and subagent relationships
- **Multi-Agent Coordination** — Coordinator/Worker mode with configurable depth
- **Workflow Engine** — DAG-based task orchestration
- **Fleet Mode** — Agent teams with shared project context
- **Project Mode** — Code indexing and automatic context injection
- **Session Management** — Fork, rewind, multi-session dashboard
- **Cross-platform** — Windows, macOS, Linux
- **Easy Install** — Single portable .exe (Windows), shell installer, winget/scoop packages

See [Chinese section](#nexus-nexus) for full documentation.

