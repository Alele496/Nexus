# Nexus — 开源 AI 编程助手

Nexus 是一个终端原生的 AI 编程助手 TUI，专为 DeepSeek V4 适配，提供 1M token 上下文窗口和链式推理能力。

## 为什么选择 Nexus

- **终端原生 TUI** — 流畅的键盘驱动界面，支持多会话管理、分屏、代码高亮
- **DeepSeek V4 深度集成** — 原生 thinking 链式推理，1M token 超长上下文
- **多 Agent 编排** — 开发/审查/运维/顾问四角色团队，Council 并行审查，Workflow 固定流水线，Supervisor 动态调度
- **扩展生态** — MCP 协议服务器、Hooks 生命周期钩子、Skills 可复用技能包、Plugins 插件系统
- **完全开源** — Apache-2.0 协议，源码开放，自由定制

## 多 Agent 团队

| 角色 | 能力 | 权限 |
|------|------|------|
| Developer | 写代码、修 bug、加功能、文档 | 读写文件 + 终端 |
| Reviewer | 安全审查、性能审查、可读性审查（三维并行） | 只读 |
| Operator | 代码推送、发布、打 tag | 只读 + 终端 |
| Advisor | 深度分析、技术趋势、可行性评估 | 只读 + 网络搜索 |

## 五种编排模式

- **Workflow** — 固定步骤流水线，适合确定性任务（如"推送代码"）
- **Council** — 多 Agent 并行审查，结果汇总裁决
- **Supervisor** — 动态调度，主 Agent 在每步运行时决定下一步
- **Handoff** — 先分类问题领域，再移交给对应专家
- **Hybrid** — 复杂多阶段任务，不同阶段用不同模式

## 快速开始

### 1. 构建

```bash
cd nexus/
cargo build -p nexus-bin --release
# 输出: target/release/nexus.exe (Windows) / nexus (Linux/macOS)
```

### 2. 配置

首次运行自动弹出设置向导，或手动编辑 `~/.sage/config.toml`：

```toml
default_model = "deepseek-v4-pro"

[model.deepseek-v4-pro]
model = "deepseek-v4-pro"
base_url = "https://api.deepseek.com/v1"
api_key = "sk-xxx"
context_window = 1000000
reasoning_effort = "high"
supports_reasoning_effort = true
```

### 3. 启动

```bash
./target/release/nexus
```

## 常用命令

| 命令 | 说明 |
|------|------|
| `/new` | 新建会话 |
| `/model` | 切换模型 |
| `/effort` | 调整推理深度 (high/max) |
| `/fork` | 当前会话分支为并行 Agent |
| `/resume` | 恢复历史会话 |
| `/compact` | 压缩对话历史以节省上下文 |
| `/settings` | 打开设置面板 |
| `/theme` | 切换主题 |
| `/help` | 浏览命令和键盘快捷键 |

## 目录结构

```
Nexus/
├── nexus/                         # Nexus TUI 源码（Rust workspace）
│   ├── crates/codegen/
│   │   ├── nexus-bin/             # 入口 → nexus.exe
│   │   ├── nexus-pager/           # TUI 界面
│   │   ├── nexus-shell/           # Agent 运行时
│   │   ├── nexus-agent/           # 系统提示词 & 模板
│   │   └── ...
│   ├── Cargo.toml                 # Workspace 清单
│   └── README.md                  # Nexus 源码文档
├── fleet/                         # 多仓库舰队管理
│   ├── README.md
│   ├── fleet-registry.json        # 项目注册表
│   └── docs/                      # 舰队策略和 Spawn 协议
├── .sage/                         # Nexus 项目配置（本地，不提交）
│   ├── config.toml
│   ├── agents/                    # 子 Agent 定义
│   ├── skills/                    # 可复用技能
│   ├── personas/
│   ├── hooks/
│   └── rules/
├── AGENTS.md                      # 主 Agent 身份和行为规范
├── scripts/                       # 构建和验证脚本
└── README.md                      # 本文件
```

## 技术栈

- **语言**: Rust
- **TUI 框架**: ratatui
- **默认模型**: DeepSeek V4 Pro（1M token 上下文）
- **Agent 编排**: Subagent + Skills + Personas + Plan Mode
- **扩展协议**: ACP (Agent Client Protocol), MCP (Model Context Protocol)

## 致谢

Nexus 基于 [xAI grok-build](https://github.com/xai/grok-build)（Apache-2.0）分支而来，感谢 xAI 团队出色的终端 TUI 框架和 Agent 基础设施。

本项目在此之上完成了以下工作：

- **DeepSeek V4 原生适配** — thinking 链式推理、1M token 上下文、reasoning_effort 深度映射
- **中文本地化** — 系统提示词、全部命令描述、TUI 界面全面汉化
- **首次运行向导** — 自动检测配置、引导设置 API Key 和模型参数
- **多 Agent 编排系统** — 开发/审查/运维/顾问四角色团队，Council 并行审查等五种编排模式
- **舰队管理系统** — 多仓库健康巡检、项目注册表、定时任务调度

## License

Nexus 源码：Apache-2.0
项目配置文件和文档：MIT
