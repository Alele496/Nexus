# Nexus

> 终端原生的 AI 编程助手。DeepSeek V4 驱动，多 Agent 协作，键盘即一切。

Nexus 在你的终端里提供一个完整的 AI 编程环境——不只是聊天，而是让多个 AI Agent 并行审查代码、自动推送、分析架构。没有浏览器标签页，没有鼠标操作，全部由键盘驱动。

## 快速上手

### 下载二进制（Windows）

从 [Releases](https://github.com/Alele496/Nexus/releases) 下载 `nexus-v0.2.105-windows-x64.zip`，解压后双击运行。首次启动会自动弹出设置向导，引导你配置 DeepSeek API Key。

### 从源码构建

**前置依赖：**
- [Rust](https://rustup.rs) 1.80+
- Windows SDK（Windows 用户需要）

```bash
git clone https://github.com/Alele496/Nexus.git
cd Nexus/nexus

# 编译检查
cargo check -p nexus-bin

# Release 构建
cargo build -p nexus-bin --release

# 二进制输出位置：
#   Windows: target/release/nexus.exe
#   Linux/macOS: target/release/nexus
```

> **Windows 用户注意：** 如果遇到 `rc.exe` 找不到的错误，需要安装 [Windows SDK](https://developer.microsoft.com/windows/downloads/windows-sdk/)。

## 功能亮点

**超长上下文，不会忘**
1M token 上下文窗口，整个项目丢进去都撑不满。支持 thinking 链式推理，复杂问题自动深度思考。

**终端原生，快且美**
Rust + ratatui 构建，启动秒开，键盘驱动。语法高亮、多会话切换、主题切换，全程不用离开终端。

**多 Agent 团队协作**
一个 Project Lead + 四个角色（Developer、Reviewer、Operator、Advisor）组成你的 AI 开发团队。不是单打独斗，是团队作战。

**代码审查闸门**
代码推送前必须经过安全、性能、可读性三维并行审查。任一维度不通过就打回重做，确保不会把问题代码推上去。

**可扩展**
MCP 协议连接外部工具，Hooks 在关键节点触发自动化脚本，Skills 把重复任务封装成可复用技能包。

**舰队管理**
同时管理多个代码仓库，一键健康巡检，项目注册表记录所有依赖关系。

## 多 Agent 协作

| 角色 | 做什么 |
|------|--------|
| **Developer** | 写代码、修 bug、实现功能 |
| **Reviewer** | 安全 / 性能 / 可读性 三维并行审查 |
| **Operator** | 代码推送、发布、打 tag |
| **Advisor** | 技术趋势分析、方案可行性评估 |

五种编排模式——Workflow 固定流水线、Council 并行审查、Supervisor 动态调度、Handoff 分类移交、Hybrid 嵌套组合——根据任务复杂度灵活选择。

## 常用命令

| 命令 | 说明 |
|------|------|
| `/new` | 新建会话 |
| `/model` | 切换模型 |
| `/effort` | 调整推理深度 |
| `/review` | 触发代码审查 |
| `/resume` | 恢复历史会话 |
| `/compact` | 压缩上下文，节省窗口 |
| `/theme` | 切换主题 |
| `/help` | 命令和快捷键一览 |

## 项目结构

```
Nexus/
├── nexus/                         # 核心源码（Rust workspace，~85 个 crate）
│   ├── crates/codegen/
│   │   ├── nexus-bin/             # 程序入口，CLI 参数解析
│   │   ├── nexus-pager/           # TUI 渲染引擎，全屏终端界面
│   │   ├── nexus-shell/           # Agent 运行时，会话管理、工具调度
│   │   ├── nexus-agent/           # 系统提示词模板、Agent 发现与注册
│   │   ├── nexus-config/          # 配置文件加载、合并、热更新
│   │   ├── nexus-mcp/             # MCP 协议客户端，连接外部工具服务
│   │   ├── nexus-hooks/           # 生命周期钩子系统，事件驱动脚本
│   │   ├── nexus-tools/           # 内置工具集（文件读写、终端、Git 等）
│   │   └── ...
│   └── Cargo.toml                 # Workspace 清单，定义所有 crate 依赖
├── fleet/                         # 多仓库舰队管理
│   ├── fleet-registry.json        # 项目注册表（路径、端口、依赖关系）
│   ├── docs/                      # 舰队策略文档和 Spawn 协议
│   └── scripts/                   # 健康巡检、Cron 调度脚本
├── scripts/                       # 构建脚本（build-nexus.sh/.ps1）
├── AGENTS.md                      # Project Lead 身份定义和行为规范
└── README.md
```

> `.sage/` 目录是本地配置（Agent 定义、Skills、Hooks），通过 `.gitignore` 忽略，不会提交到仓库。

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
