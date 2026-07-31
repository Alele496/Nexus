# Nexus

<p align="center">
  <strong>开源 · 本地优先 · 多 Agent 开发中台</strong>
</p>

<p align="center">
  <strong>你的代码库管家。</strong>说一句需求，从拆任务到发布，全流程跑完。
</p>

<p align="center">
  <a href="https://github.com/Alele496/Nexus/actions/workflows/ci.yml"><img src="https://github.com/Alele496/Nexus/actions/workflows/ci.yml/badge.svg" alt="CI"></a>
  <a href="LICENSE"><img src="https://img.shields.io/badge/license-Apache%202.0-blue.svg" alt="License"></a>
  <a href="https://github.com/Alele496/Nexus/releases"><img src="https://img.shields.io/github/v/release/Alele496/Nexus" alt="Release"></a>
</p>

---

## 你现在的处境

你手上有好几个仓库，每个都跑着 AI 编码助手，但它们各干各的：

- **每个助手只看得见一个仓库**——跨仓库改个字段，5 个服务要手动一个个去改，漏掉一个就是线上事故
- **Agent 是黑盒**——跑起来你不知道它在干什么，也没法中途插话、单独聊天
- **记忆不带走**——这次会话踩的坑，换个会话全忘了，同一个错误犯三遍
- **发布没有把关**——代码改完谁都能直接推，出问题才想起来没审查

Nexus 把这些串成一条流水线：**你只负责说需求，剩下的交给它。**

它不是"更好的聊天工具"——它是管理整个系统的工作台：多个仓库、多个 Agent、一条从需求到发布的完整流水线。

---

## 一个需求怎么走完全程

周五下午，PM 丢来一句：把 user 表的 `name` 字段改成 `display_name`，所有受影响的服务一起改。

传统流程：改 4 个仓库、协调 4 个助手、手动查引用、手动发布。在 Nexus 里是这样：

```
┌─ Nexus ─────────────────────────────────────────────────────┐
│ 你: 把 user 表的 name 改成 display_name，受影响的服务一起改      │
│                                                              │
│ Project Lead: 先查哪些服务依赖 user 表……                        │
│   · 图谱找到 5 处引用: gateway / user-svc / order /            │
│     notify / admin                                           │
│   · 记忆提示: 上次改 user 表踩过 SQLite 迁移的坑                │
│   · 拆成 6 个任务，交给 Coordinator                            │
│                                                              │
│ Coordinator: 5 个 Worker 并行开工                              │
│   gateway  改字段引用 ✓      user-svc  改模型+迁移 ✓            │
│   order    改字段 ✓          notify    改模板 ✓                │
│   admin    改接口 ✓                                           │
│                                                              │
│ Reviewer: 安全 ✓ 性能 ✓ 可读性 ⚠ 命名不一致 → 打回              │
│   Developer: 已修正，重新提交 ✓                                │
│   Reviewer: 三维全过 ✓                                        │
│                                                              │
│ Operator: 发布 5 个仓库 ✓  打 tag v1.4.0 ✓                     │
│ 记忆沉淀: 「跨服务字段改名 → 先查引用再动工」已归档               │
│                                                              │
│ ✅ 5 个仓库一致变更，用时 4 分 30 秒                            │
└──────────────────────────────────────────────────────────────┘
```

### 这条流水线的六个环节

| 环节 | 系统能力 | 支撑 |
|------|---------|------|
| **获取上下文** | 读懂仓库与历史 | 记忆系统、知识图谱（CodeGraph + Graphify）、共享上下文 |
| **规划拆解** | 大需求拆成可执行任务 | Project Lead、Coordinator、Supervisor |
| **并行实现** | 多 Agent 同时改多个仓库 | 六角色矩阵、五种编排模式、Fleet 跨仓库、冲突检测 |
| **三维审查** | 合并前把关，不通过打回 | Reviewer、Council、审查闸门 |
| **审批发布** | 受控上线，权限可管 | Operator、`/approve-scope` |
| **记忆沉淀** | 经验留到下次 | dream 整合、watcher 索引、跨会话记忆 |

单点功能谁都做，把六个环节串成自动化的流水线，才是 Nexus 作为"开发中台"的含义。

---

## 你的开发团队

Nexus 不是单个助手，是一个**六人开发团队**，随时听你指挥：

| 成员 | 干什么 | 对应角色 |
|------|--------|---------|
| **项目负责人** | 理解需求、设计方案、分派任务 | Project Lead |
| **程序员** | 写代码、修 Bug、跑测试 | Developer |
| **质检** | 安全、性能、可读性三维把关，不通过打回 | Reviewer |
| **发布专员** | 审查通过后推送、发布、打 Tag | Operator |
| **顾问** | 技术选型、架构评估，回答"这方案有什么风险？" | Advisor |
| **调度** | 动态编排、拆解分配子任务 | Coordinator |

需要几个人上几个人，不需要养全职团队。

---

## 五种干活方式

不同任务用不同干法，不是一刀切：

```
流水线干活   Workflow    →  步骤严格有序：A→B→C（CI/CD、数据处理）
多人质检     Council     →  三个质检同时审，任一不过就驳回
老板盯着     Supervisor  →  调度持续监控，随时调整分配
谁擅长谁干   Handoff     →  按任务类型路由到最合适的成员
混合编排     Hybrid      →  以上四种随意拼装
```

---

## 模型自由

Nexus 不绑定任何模型厂商。DeepSeek、GPT、Claude、私有化部署——只要是 OpenAI 兼容 API 就能用，在设置里随时切换。

| 提供商 | 支持模型 | 特色 |
|--------|----------|------|
| **DeepSeek** | V4 | 1M token 上下文，原生 thinking 链式推理，国内直连零变通 |
| **OpenAI** | GPT-5, o4 系列 | 最广的生态兼容 |
| **Anthropic** | Claude 4.7 Opus, 4.6 Sonnet | 最强的代码理解能力 |
| **自定义** | 任何 OpenAI 兼容 API | 私有机房、代理网关、内网模型 |

**不同任务用不同模型**——复杂重构用思考最深的，批量格式化用最快的，敏感项目用内网部署的。首次运行自动弹出设置向导，填 Key 30 秒搞定。

---

## 不只是聊天

### Agent 信箱

Agent 之间像同事一样异步通信——`send_message` 给另一个 Agent 留言，对方随时用 `check_mailbox` 取信。跨会话、跨时间，人不在线消息也不丢。支持标签寻址（给 Agent 起名 `gateway-service`）、优先级（normal/urgent）、回复线程与消息持久化。

### 记忆系统

Markdown 存储 + 混合检索（全文 + 可选向量），带 dream 整合和文件 watcher。**Agent 跨会话记住你的偏好、项目背景和踩过的坑**——上次踩的 SQLite 迁移坑，这次自动提醒。启用：`--experimental-memory`。

### 审查闸门 + 审批作用域

代码推送前强制通过三维审查。通过 `/approve-scope` 精细控制每个 Agent 的权限——哪些目录可以读、哪些文件可以写、哪些命令可以执行。

### Agent 拓扑图（Ctrl+G）

实时看你的开发团队在干什么。谁在跑、谁在等、子 Agent 的父子关系——一张图一目了然，点开任意 Agent 还能单独聊天。

### 共享上下文 + 冲突检测

多个 Agent 看同一个项目时，共享一份文件级上下文。谁读了什么、谁改了哪里，Dashboard 实时显示。两个 Agent 同时改同一个文件？系统自动标记冲突。

### MCP 生态

完整实现 [MCP 协议](https://modelcontextprotocol.io)（stdio/HTTP、OAuth、凭据管理），兼容 Claude Code 的 MCP server 规范。项目级 `.mcp.json` 自动加载、变更热重载——已有的 MCP server（如 CodeGraph）开箱即用。

### 知识图谱（自举）

CodeGraph 符号索引 + Graphify 文件级图谱，让 Agent 理解代码库更快——而 Nexus 自己就是这个图谱的用户，也是你参与开发的入口。

---

## 快速开始

1. **下载**：从 [Releases](https://github.com/Alele496/Nexus/releases) 下载对应平台最新版
2. **配置**：首次运行自动弹出设置向导，选提供商、填 Key，30 秒
3. **说需求**：`/new` 新建会话，Agent 团队就位，直接说你想做什么

| 平台 | 文件 |
|------|------|
| Windows | `nexus-<version>-windows-x86_64.zip` |
| Linux | `nexus-<version>-linux-x86_64` |
| macOS | `nexus-<version>-macos-aarch64` |

Windows 用户请下载 zip 包（裸 exe 会被 SmartScreen 拦截）。解压后若仍提示"Windows 已保护你的电脑"：右键 exe → 属性 → 勾选"解除锁定"；或用 7-Zip 解压（不携带"来自网络"标记）。这是未签名程序的正常提示——项目构建完全开源，CI 流水线在 [GitHub Actions](https://github.com/Alele496/Nexus/actions) 公开可审计。

### 校验发布产物

每个 Release 页面都附一张**构建校验表**：源码 commit、构建环境、固定工具链（Rust 1.92.0）、构建命令，以及每个产物的 SHA256 哈希。证据链：**tag → commit → 固定工具链 + 构建命令 → 产物哈希**。

下载后对照核对，几秒钟完成：

| 平台 | 命令 |
|------|------|
| Linux / macOS | `sha256sum nexus-<version>-linux-x86_64` |
| Windows | `certutil -hashfile nexus-<version>-windows-x86_64.zip SHA256` |

哈希一致，说明你拿到的是 CI 从公开源码构建出的那份产物，未经篡改。想再深一层，`git checkout <校验表中的 commit>` 后按下方源码构建流程自行编译（同样带 `--locked` 锁定依赖版本），再比对哈希——产物一致即完全可复现。依赖安全由 CI 中的 `cargo audit`（rustsec 漏洞库扫描）持续把关，任一依赖出现已知 CVE 都会让 CI 变红。

### 从源码构建

```bash
# 前置条件：Rust 1.80+、protoc 27.2+
git clone https://github.com/Alele496/Nexus.git
cd Nexus/nexus
cargo build -p nexus-bin --release --locked
```

> Windows 用户注意：需要 Windows SDK。中文用户名可能引发路径编码问题，执行前设置 `TMP=C:/tmp` 即可。

---

## 与同类工具对比

2026 年的 AI 编程工具分化成三派：**云上编码助手**（Claude Code、Codex）单 Agent 能力强但闭源、数据在云上；**办公工作台**（WorkBuddy）界面优美但面向职场人；**记忆型长驻 Agent**（Hermes Agent）持久记忆、自学习但 Windows 支持弱。Nexus 的战场在它们的交叉空白。

| | Nexus | Claude Code | Codex | Hermes Agent | WorkBuddy |
|------|-------|-------------|-------|--------------|-----------|
| **定位** | 开发中台 | 编码助手 | 云端编码 | 记忆型长驻 Agent | 办公工作台 |
| **开源** | Apache-2.0 | — | — | MIT | — |
| **本地私有** | ✓ 单二进制 | — | — | ✓ | — |
| **中文原生 / 国内直连** | ✓ DeepSeek 零变通 | — | — | 部分 | ✓ |
| **多 Agent 编排** | 六角色 + 五种模式 | 子 Agent | 子任务 | ✓ | 并行执行 |
| **Agent 拓扑 / 信箱** | ✓ | — | — | 部分 | — |
| **持久记忆** | Markdown + 混合检索 | ✓ | — | 多级记忆 | ✓ |
| **跨仓库 Fleet** | ✓ | — | — | — | — |
| **MCP 生态** | ✓ 完整客户端 | ✓ | ✓ | ✓ | ✓ |
| **UI 形态** | TUI + Web 面板 | TUI | TUI | TUI + 桌面 | 桌面工作台 |

---

## 常用命令

```
/new              新建会话，Agent 团队就位
/model            切换模型（DeepSeek / OpenAI / Anthropic / 自定义）
/effort           调整推理深度（high → xhigh）
/review           触发代码审查（安全 / 性能 / 可读性）
/coordinator      启动调度，拆分并分配大任务
/approve-scope    设置 Agent 审批作用域
/resume           恢复历史会话，上下文完整保留
/compact          压缩上下文，释放 token 窗口
/theme            切换主题，实时预览
/dashboard        打开团队看板
/agent-graph      打开 Agent 关系拓扑图
/help             命令和快捷键一览
```

## 快捷键

```
F2      设置面板
Ctrl+G  Agent 拓扑图
Ctrl+L  新建会话
```

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
│ nexus-shell  │  nexus-agent     │  Agent 信箱/调度器*      │
│ Agent 运行时  │  提示词模板       │  Agent 异步消息          │
│ 会话管理      │  Agent 注册      │  Coordinator ↔ Worker  │
│ 工具调度      │  模型适配        │  定时任务 · 后台自治      │
├──────────────┴──────────────────┴───────────────────────┤
│    * 实现于 nexus-tools 的 SageBuild 工具集（send_message、check_mailbox、scheduler）│
├─────────────────────────────────────────────────────────┤
│                    nexus-config                          │
│            配置文件加载 · 合并 · 热更新 · 验证                │
├─────────────────────────────────────────────────────────┤
│   nexus-tools · nexus-mcp · nexus-memory · nexus-hooks   │
│   内置工具  ·  外部协议  ·  记忆系统  ·  生命周期钩子          │
├─────────────────────────────────────────────────────────┤
│    nexus-workflow (工作流引擎)    nexus-project (单仓库)    │
│    DAG 管道 · 步骤编排          项目上下文 · 依赖分析         │
├─────────────────────────────────────────────────────────┤
│              nexus-fleet-types (舰队管理)                  │
│         多仓库注册 · 健康巡检 · 批量操作                      │
└─────────────────────────────────────────────────────────┘
```

---

## 未来路线

Nexus 的演进方向详见 [ROADMAP](nexus/ROADMAP.md)，核心主线：

- **P1 · Fleet 跨仓库自治** —— 跨仓库影响分析、原子事务、批量分支、跨仓库 PR 工作流
- **P2 · Agent 长驻化与信箱深化** —— 信箱已就位，下一步：信箱可视化面板（`/mailbox`）、Leader 新消息推送、Agent 离线继续工作、跨进程/跨主机消息
- **P3 · 体验增强** —— 记忆系统完善（对标成长型 Agent）、**Web 协作面板**（拓扑图/看板浏览器访问）、后台自治深化、国产模型适配

---

## 致谢

Nexus 基于 [xAI grok-build](https://github.com/xai/grok-build)（Apache-2.0）构建，xAI 团队的开源工作让这一切成为可能。

本项目在此之上实现了：

- **模型中台**：多提供商统一接口，首次运行向导，按任务切换模型
- **Agent 中台**：六角色矩阵，五种编排模式，Agent 拓扑图
- **审查中台**：三维并行审查、审批作用域、冲突检测
- **Agent 信箱**：跨会话异步消息、标签寻址、消息持久化
- **中文原生**：系统提示词、命令描述、TUI 界面全面汉化
- **CI/CD**：跨平台自动构建，推送 tag 自动发布制品

## License

Apache-2.0
