# Nexus 发展路线

## 定位

**中国人自己的开源 AI 终端编程助手。**

> 当前国内没有好用的终端 AI 编程工具：Claude Code 不支持中国地区，Cursor 需要代理，
> Codex CLI 需要 OpenAI API。Nexus 填补这个空白——Rust 原生，中文优先，开源免费。

## 目标用户（渐进式发展）

```
个人开发者 ──────────→ 小团队 ──────────→ 企业
（当下）              （未来）            （远期）
```

| 阶段 | 用户 | 痛点 | 核心卖点 |
|------|------|------|---------|
| **当下** | 个人开发者 | 国内没有好用的 AI 终端工具 | 免费、中文、DeepSeek 直连、Windows 原生 |
| **未来** | 5-20 人团队 | 微服务多了，跨仓库改字段漏改 | Fleet 跨仓库协同 |
| **远期** | 企业 | 内部框架统一升级、安全漏洞批量修复 | 私有化部署 + 国产模型 + 合规 |

## 开源策略

**纯开源（MIT/Apache 2.0）。**

不设托管服务、不设企业版。社区驱动，人人可用。等有团队和资金后再考虑商业路径。

## 核心护城河

当前所有 AI 编程助手（Claude Code、Cursor、Codex CLI、opencode）的架构限制：

| 局限 | 竞争对手 | Nexus |
|------|---------|-------|
| 单仓库 | 天然无法跨仓库工作 | Fleet 多仓库管理 + Coordinator 并行分派 |
| 单会话 | 子 Agent 是黑盒，看不到也聊不了 | Agent Graph 可视化，点开任意 Agent 独立聊天 |
| 单窗口 | 多个会话要多开终端 | Leader 后台常驻，TUI 内多窗格 |
| 无 worktree | 人工 git worktree | Rust 原生 CoW 克隆 + 沙箱隔离 |

**Nexus 不是"更好的聊天工具"，而是"你的代码库管家"——管理整个系统、多个仓库、多个 Agent 在同时工作。**

---

## 功能路线

### P0 — 基础体验 + 第一波传播（当下）

| 功能 | 说明 | 状态 |
|------|------|------|
| CI 全绿 | Windows + Linux 零失败 | ✅ 已完成 |
| 模型无关化 | 支持所有 OpenAI 兼容 API，用户自填 Key + Base URL | ✅ 已完成 |
| **Agent Graph 可视化** | `/agent-graph` 命令，可视化所有 Agent 关系图，点击独立聊天 | ✅ 已完成 |
| Windows 安装体验 | 单 exe 免安装，winget / scoop 包 | ✅ 已完成 |
| 中文文档 | 完整中文 README + 使用指南 | ✅ 已完成 |
| **MCP 生态兼容** | `nexus-mcp`：stdio/HTTP 客户端、OAuth、凭据；项目级 `.mcp.json` 热重载 | ✅ 已完成 |
| **记忆系统** | `nexus-memory`：Markdown 存储 + FTS5/向量混合检索 + dream 整合 + 文件 watcher | 🟡 实验性（`--experimental-memory`） |

**为什么 Agent Graph 放在 P0：** 对个人开发者来说，可视化多 Agent 并行工作是最直观的差异化体验——"这工具能同时跑 5 个 Agent 还能点开聊天？"——比功能列表更有传播力。

---

### Agent Graph 可视化详情

`/agent-graph` 命令打开 Agent 关系图：

```
┌─────────────────────────────────────────────────────┐
│  Agent Graph                              [拖拽] [缩放]│
│                                                       │
│              ┌──────────┐                             │
│              │Coordinator│ ◄── 总指挥                  │
│              │  🟢 空闲   │                             │
│              └─────┬────┘                             │
│         ┌──────────┼──────────┐                       │
│         ▼          ▼          ▼                       │
│  ┌──────────┐ ┌──────────┐ ┌──────────┐              │
│  │ user-svc │ │ order-svc│ │ gateway  │              │
│  │ 🟡 运行中 │ │ 🟢 完成  │ │ 🔴 失败  │ ← 点击打开     │
│  │"改字段中" │ │"测试通过" │ │"找不到引 │              │
│  └──────────┘ └──────────┘ │ 用..."   │              │
│                             └─────┬────┘              │
│   ┌──────────────────────────────┼──────┐            │
│   │ 与 gateway Agent 的对话       │      │  ← 侧边窗格 │
│   │                              │      │            │
│   │ Agent: 找不到 User.name 字段  │      │            │
│   │ 你:   User 表在 user-svc 里   │      │            │
│   │ Agent: 明白了，重新定位中...   │      │            │
│   └──────────────────────────────┴──────┘            │
│                                                       │
│  [F1 图] [F2 列表] [F3 日志] [Tab 下一个] [Esc 关闭]   │
└─────────────────────────────────────────────────────┘
```

**功能点：**
- 力导向布局自动排列节点，支持鼠标拖拽
- 节点颜色表示状态（空闲/运行中/完成/失败）
- 点击节点打开侧边窗格，与目标 Agent 独立聊天
- 连线表示父子/依赖关系
- 支持缩放和平移
- Tab 在多个 Agent 聊天窗格间切换

**为什么别人抄不了：**
- 需要 Leader 常驻进程管理 Agent 注册表和状态推送 → 架构级依赖
- 需要 Fleet + Coordinator 记录 Agent 父子关系树 → 数据来源
- 需要每个 Agent 是独立会话（可单独聊天）→ 多会话架构
- Claude Code 等工具的子 Agent 是临时的、黑盒的，没有这些基础设施

---

### P1 — Fleet Agent 跨仓库自治

| 功能 | 说明 |
|------|------|
| 跨仓库影响分析 | 根据 proto/API 定义自动判断变更影响哪些服务 |
| 跨仓库原子事务 | 多 repo 的 commit/revert 要么全成功要么全回滚 |
| Fleet 级分支管理 | 一条命令为所有 repo 创建同名 worktree 分支 |
| 跨仓库 PR 工作流 | 一个指令自动在多个 repo 开 PR，关联追踪 |

典型场景：

```
用户: "把 user 表的 name 字段改成 display_name，所有受影响的服务一起改"

Nexus:
  1. Fleet 影响分析 → 找出所有依赖 user 表的服务
  2. Coordinator 分解 → 每个服务一个子任务
  3. 5 个 Worker 并行 → 同时改 gateway/user-service/order-service/notification/admin
  4. Workflow 编排 → 先改 proto，再改各服务，最后跑集成测试
  5. 任一失败 → 全部回滚
```

---

### P2 — Agent 信箱系统

让 Agent 之间像同事一样异步协作：

```
Agent A（网关服务）→ 消息 → Agent B（用户服务）→ 消息 → Agent C（订单服务）
         ↑                         ↑                         ↑
         └─────────────────────────┼─────────────────────────┘
                              消息总线（Leader 常驻进程）
```

- Agent 持久存在，有独立"地址"和待办队列
- 跨会话、跨时间异步通信
- 人离线时 Agent 继续工作，完成通知
- 建立数据护城河 — Agent 在你的系统里积累协作历史，切换工具意味着全部丢失

---

### P3 — 体验增强

| 功能 | 说明 |
|------|------|
| VS Code 扩展 | TUI + IDE 互补，Nexus 作为 AI 后端 |
| 后台自治 | /loop /schedule，Agent 离开终端运行 |
| 国产模型适配 | 通义千问、Moonshot、Qwen 等 |
| 记忆系统完善 | dream 定时整合、跨工作区记忆、嵌入模型选择 |
| 团队协作 | Fleet 共享、Agent 消息跨成员传递 |

---

## 开发基础设施：知识图谱

Nexus 的代码库自带两个知识图谱工具，作为项目自身的开发基础（也是 Agent 理解代码的第一手段）：

- **CodeGraph**（`nexus/.codegraph/`）：符号级 SQLite 索引。`codegraph explore "<符号或问题>"` 一次返回相关符号逐行源码 + 调用路径（含动态分发）。也提供 MCP server（`codegraph serve --mcp`）。
- **Graphify**（`graphify-out/` / `nexus/graphify-out/`）：文件级知识图谱，含 god nodes 与社区结构。`graphify query/path/explain` 返回作用域子图，比 grep 更小更聚焦。修改代码后 `graphify update .` 保持图最新（AST-only，零 API 成本）。

**理念**：Agent 探索代码时优先查图而不是翻源码——图检索一次拿到目标符号与调用链，grep/翻文件只作为兜底。

---

## 记忆系统详情

`nexus-memory` crate 实现，存储为人类可读 Markdown，索引自动维护：

- **存储**：`~/.nexus/memory/`（全局）+ `~/.nexus/memory/{project-slug}-{hash8}/`（工作区）
- **检索**：SQLite FTS5 全文 + 可选向量 KNN 混合检索（recency/source 加权），无嵌入 API 时优雅降级为 FTS-only
- **整合**：dream 会话把散落记忆整合归档（与 Claude Code `/dream` 同源）
- **感知**：文件 watcher 监视 `.md` 变更自动重建索引，外部编辑（含迁移来的记忆文件）即时可搜
- **启用**：`--experimental-memory` 或 `NEXUS_MEMORY=1`，配置在 `[memory]` section

> 迁移：Claude Code / CCB 的记忆是同一类 Markdown 文件，直接拷入 `~/.nexus/memory/` 即可被索引检索。
