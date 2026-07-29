# Nexus

<p align="center">
  <strong>AI 驱动的开发中台</strong>
</p>

<p align="center">
  多模型 · 多 Agent · 端到端自动化 — 从需求到发布，一个平台完成
</p>

<p align="center">
  <a href="https://github.com/Alele496/Nexus/actions/workflows/ci.yml"><img src="https://github.com/Alele496/Nexus/actions/workflows/ci.yml/badge.svg" alt="CI"></a>
  <a href="LICENSE"><img src="https://img.shields.io/badge/license-Apache%202.0-blue.svg" alt="License"></a>
  <a href="https://github.com/Alele496/Nexus/releases"><img src="https://img.shields.io/github/v/release/Alele496/Nexus" alt="Release"></a>
</p>

---

## 定位

Nexus 不是又一个 AI 编码助手。

它是一个**开发中台**——连接模型、Agent 和你的代码仓库，把"写一行代码"变成"完成一个需求"。从模型选择到 Agent 调度，从代码审查到自动发布，Nexus 提供完整的基础设施。

**你只需要告诉它做什么，Nexus 负责模型适配、Agent 调度、审查闸门——把基础设施搭好，你专注需求本身。**

---

## 模型无关

不会被任何一家模型厂商绑定。Nexus 是一个**模型中台**：

| 提供商 | 支持模型 | 特色 |
|--------|----------|------|
| **DeepSeek** | V4 | 1M token 上下文，原生 thinking 链式推理 |
| **OpenAI** | GPT-5, o4 系列 | 最广的生态兼容 |
| **Anthropic** | Claude 4.7 Opus, 4.6 Sonnet | 最强的代码理解能力 |
| **自定义** | 任何 OpenAI 兼容 API | 私有机房、代理网关、内网模型 |

> 只要模型提供 OpenAI 兼容的 API，Nexus 就能用——不限上表所列。

你可以在**不同任务用不同模型**——复杂重构用思考最深的，批量格式化用最快的，敏感项目用内网部署的。一切在 Settings（F2）里随时切换。

首次运行自动弹出设置向导，选提供商、填 Key，30 秒完成。

---

## Agent 中台

Nexus 的核心不是聊天，而是**多 Agent 协作**。

### 默认 Agent 矩阵

| 角色 | 职责 | 典型场景 |
|------|------|----------|
| **Project Lead** | 需求理解、方案设计、任务分配 | 你说"加个支付功能"，它来拆任务 |
| **Developer** | 编码实现、Bug 修复 | 写代码、跑测试、回滚 |
| **Reviewer** | 安全、性能、可读性三维审查 | 代码合并前自动审查，不通过打回 |
| **Operator** | 推送、发布、打 Tag | 审查通过后自动发布 |
| **Advisor** | 技术选型、架构评估 | "这个方案有什么风险？" |
| **Coordinator** | 动态编排、子 Agent 调度 | 大任务自动拆分→分配→汇总 |

### 五种编排模式

不同任务用不同编排——不是一刀切。

```
Workflow    →  固定流水线：A→B→C，步骤严格有序（CI/CD、数据处理）
Council     →  并行审查：三个 Reviewer 同时审，任一不过就驳回
Supervisor  →  动态调度：Coordinator 持续监控、随时调整分配
Handoff     →  分类移交：路由到最擅长的 Agent 处理
Hybrid      →  嵌套组合：以上四种随意拼装
```

---

## 不只是聊天

### 设置面板（F2）

所有配置集中管理。API Key 遮罩显示、代理设置、模型切换、主题预览——不需要翻配置文件，不用重启。

### Agent 拓扑图（Ctrl+G）

实时看你的 Agent 团队在干什么。谁在跑、谁在等、子 Agent 的父子关系——一张图一目了然。

### Agent 信箱

Agent 之间可以互相发消息。Coordinator 给 Worker 派任务，Worker 完成后汇报——全部异步，不阻塞主流程。

### 共享上下文 + 冲突检测

多个 Agent 看同一个项目时，共享一份文件级上下文。谁读了什么、谁改了哪里，Dashboard 上实时显示。两个 Agent 同时改同一个文件？系统自动标记冲突。

### 审查闸门 + 审批作用域

代码推送前强制通过三维审查。通过 `/approve-scope` 精细控制每个 Agent 的权限——哪些目录可以读、哪些文件可以写、哪些命令可以执行。

---

## 常用命令

```
/new              新建会话，Agent 矩阵就位
/model            切换模型（DeepSeek / OpenAI / Anthropic / 自定义）
/effort           调整推理深度（high → xhigh）
/review           触发代码审查（安全 / 性能 / 可读性）
/coordinator      启动 Coordinator，拆分并分配大任务
/approve-scope    设置 Agent 审批作用域
/resume           恢复历史会话，上下文完整保留
/compact          压缩上下文，释放 token 窗口
/theme            切换主题，实时预览
/dashboard        打开团队看板
/help             命令和快捷键一览
```

## 快捷键

```
F2      设置面板
Ctrl+G  Agent 拓扑图
Ctrl+L  新建会话
```

---

## 安装

### 下载二进制

从 [Releases](https://github.com/Alele496/Nexus/releases) 下载对应平台的最新版本：

| 平台 | 文件 |
|------|------|
| Windows | `nexus.exe` |
| Linux | `nexus` |
| macOS | `nexus` |

### 从源码构建

```bash
# 前置条件：Rust 1.80+、protoc 27.2+
git clone https://github.com/Alele496/Nexus.git
cd Nexus/nexus
cargo build -p nexus-bin --release
```

> Windows 用户注意：需要 Windows SDK。中文用户名可能引发路径编码问题，执行前设置 `TMP=C:/tmp` 即可。

---

## 架构

Nexus 的架构设计遵循**关注点分离**——每个组件有明确的职责边界，通过协议通信。

```
┌─────────────────────────────────────────────────────────┐
│                    nexus-bin (CLI)                      │
│                  程序入口 · 参数解析 · 守护进程               │
├─────────────────────────────────────────────────────────┤
│                   nexus-pager (TUI)                      │
│         终端渲染 · 键盘输入 · 面板系统 · 主题引擎             │
├──────────────┬──────────────────┬───────────────────────┤
│ nexus-shell  │  nexus-agent     │  nexus-mailbox         │
│ Agent 运行时  │  提示词模板       │  Agent 异步消息          │
│ 会话管理      │  Agent 注册      │  Coordinator ↔ Worker  │
│ 工具调度      │  模型适配        │                       │
├──────────────┴──────────────────┴───────────────────────┤
│                    nexus-config                          │
│            配置文件加载 · 合并 · 热更新 · 验证                │
├─────────────────────────────────────────────────────────┤
│         nexus-tools · nexus-mcp · nexus-hooks            │
│         内置工具  ·  外部协议  ·  生命周期钩子                │
├─────────────────────────────────────────────────────────┤
│    nexus-workflow (工作流引擎)    nexus-project (单仓库)    │
│    DAG 管道 · 步骤编排          项目上下文 · 依赖分析         │
├─────────────────────────────────────────────────────────┤
│              nexus-fleet-types (舰队管理)                  │
│         多仓库注册 · 健康巡检 · 批量操作                      │
└─────────────────────────────────────────────────────────┘
```

---

## 与同类工具对比

| | Nexus | Claude Code | OpenCode | Codex |
|------|-------|-------------|----------|-------|
| **定位** | 开发中台 | 编码助手 | 开源助手 | 云端平台 |
| **多 Agent 协作** | 六角色矩阵 | 子 Agent | — | 子任务 |
| **模型灵活性** | 多提供商，按任务切换 | Claude 系列 | 多提供商 | GPT 系列 |
| **运行环境** | 本地终端 | 本地终端 | 本地终端 | 云端 |
| **Agent 拓扑** | 实时可视化 | — | — | — |
| **审查闸门** | 三维并行审查 | 基础审查 | — | — |
| **Agent 信箱** | 异步消息传递 | — | — | — |
| **审批作用域** | 文件级权限控制 | 全局审批 | — | — |
| **舰队管理** | 多仓库批量操作 | — | — | ✓（组织级） |
| **工作流引擎** | DAG 可编排 | — | — | — |

---

## 致谢

Nexus 基于 [xAI grok-build](https://github.com/xai/grok-build)（Apache-2.0）构建，xAI 团队的开源工作让这一切成为可能。

本项目在此之上实现了：

- **模型中台**：多提供商统一接口，首次运行向导，按任务切换模型
- **Agent 中台**：六角色矩阵，五种编排模式，信箱异步通信
- **审查中台**：三维并行审查、审批作用域、冲突检测
- **管理面板**：Dashboard 团队看板、Settings 配置中心、Agent 拓扑图
- **中文原生**：系统提示词、命令描述、TUI 界面全面汉化
- **CI/CD**：跨平台自动构建，推送 tag 自动发布制品

## License

Apache-2.0
