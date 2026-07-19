# Agent-SYS — 开源多 Agent 编排系统

基于 [grok-build](https://github.com/xai-org/grok-build)（Apache-2.0）的多 Agent 团队协作系统。

## 为什么做

CCB（Claude Code Best）是闭源工具，多 Agent 编排受限于安全分类器和闭源许可。Agent-SYS 用开源框架 grok-build 重建完整的 Agent 团队协作能力——从单仓库开发到多仓库舰队管理。

## 目录结构

```
Agent-SYS/
├── AGENTS.md                     # 主 Agent 身份定义（= Project Lead）
├── project-memory.md             # 跨会话持久化记忆
├── .grok/
│   ├── config.toml               # grok-build 配置（角色/Personas）
│   ├── agents/                   # 子 Agent 定义
│   │   ├── developer.md          # 代码开发者
│   │   ├── reviewer.md           # 代码审查员
│   │   ├── operator.md           # 运维操作员
│   │   └── advisor.md            # 战略顾问
│   ├── skills/                   # 可复用技能
│   │   ├── dispatch/SKILL.md     # 自适应任务调度
│   │   ├── ship/SKILL.md         # 完整交付流程（五阶段）
│   │   └── council-review/SKILL.md # 三维并行审查
│   ├── personas/                 # 未使用（Personas 定义在 config.toml 中）
│   ├── hooks/                    # 生命周期安全钩子
│   │   └── safety-gates.json     # PreToolUse 安全检查
│   └── rules/                    # 项目约定
│       └── project-conventions.md
├── fleet/                        # 多仓库舰队管理
│   ├── README.md
│   └── docs/
│       ├── fleet-policy.md       # 跨项目架构原则
│       ├── fleet-registry.json   # 项目注册表
│       └── spawn-protocol.md     # Agent spawn 行为规范
└── grok-build-main/              # grok-build 源码（可二次开发）
```

## 快速开始

### 1. 安装 grok-build

```bash
# 从源码编译（推荐，支持二次开发）
cd grok-build-main/
cargo build -p xai-grok-pager-bin --release

# 或从 xAI 安装预编译版本
# curl -fsSL https://x.ai/cli/install.sh | bash
```

### 2. 配置 Agent-SYS

Agent-SYS 是一个 grok-build **项目**，不是独立工具。在任意项目目录下，复制 `.grok/` 目录和 `AGENTS.md` 即可获得多 Agent 能力：

```bash
cp -r Agent-SYS/.grok/ <你的项目>/
cp Agent-SYS/AGENTS.md <你的项目>/
```

### 3. 启动

```bash
cd <你的项目>/
grok    # 启动 grok-build TUI，自动加载 AGENTS.md 作为系统提示
```

### 4. 使用 Skills

```
/dispatch 修复登录页面的bug
/ship 实现用户权限管理功能
/council-review 审查最近的代码改动
```

## 与 agent-crew 的关系

Agent-SYS 是 agent-crew 的 grok-build 移植版本：
- Agent 定义、编排模式、审查闸门等核心概念保持一致
- Workflow 脚本（JS）→ Skills（Markdown）+
- CCB 专属特性（TeamCreate、Cron）→ grok-build 原生特性（Subagent、Worktree）

## 技术栈

- **运行时**：grok-build (Rust, Apache-2.0)
- **Agent 编排**：grok-build Subagent + Skills + Personas + Plan Mode
- **安全策略**：grok-build Hooks + capability_mode
- **舰队管理**：待开发（基于 grok-build headless mode + ACP）

## License

Agent-SYS 配置文件和文档：MIT
grok-build：Apache-2.0
