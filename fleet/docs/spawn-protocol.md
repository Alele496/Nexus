# Agent Spawn 行为规范

> Agent 被 spawn 到项目时的标准行为约定。与 agent-crew 的 spawn-protocol.md 保持兼容。

---

## Layer 0: Agent 启动 SOP

### 1. 定位自己

Agent 被 spawn 时，第一步是理解自己在哪个项目工作：

```
1. Read {cwd}/AGENTS.md         ← 项目专属 Agent 手册
2. Read {cwd}/project-memory.md ← 项目记忆
3. Read {cwd}/README.md         ← 人类文档（补充上下文）
```

如果 `cwd` 是一个工作子目录，追溯上级目录找到项目根，然后读根目录的文件。

### 2. 理解舰队上下文

如果需要跨项目信息（依赖、端口、其他项目状态），查询 `fleet-registry.json`：
- 路径：`<fleet-hq>/fleet-registry.json`
- 包含：所有项目的路径、端口、依赖关系、环境变量

### 3. 干活

- 该读的文件读了 → 开始执行任务
- 遇到需要知道的 → 先搜索本项目文档，再查 fleet-registry

---

## 主 Agent spawn 子 Agent 时的标准 brief

主 Agent 在 prompt 中只需包含：
1. **任务描述**（做什么）
2. **约束条件**（不做什么）
3. **验证标准**（怎么算做完）

不需要包含：
- 项目路径（子 Agent 从 AGENTS.md 或 fleet-registry 中获取）
- 构建命令（子 Agent 从项目 AGENTS.md 中获取）
- 环境变量（子 Agent 从 fleet-registry 中获取）

### 正确示范

```
在项目中实现 X 功能。
先读 AGENTS.md 了解项目，然后写代码。
完成后跑测试验证，全部通过才算完成。
```

### 错误示范

```
src/server.mjs 的 629 行有个 bug，
httpServer.listen Promise 没处理 reject。改成...
改完跑 test。
```

→ 问题：主 Agent 写了具体行号和代码。子 Agent 应该自己找到正确的位置，而不是被 micro-manage。

---

## 跨项目 spawn 的路径约定

| 项目 | cwd | AGENTS.md 路径 |
|------|-----|---------------|
| 各项目 | 项目根目录 | `{cwd}/AGENTS.md` |
| 舰队级 | fleet/ 目录 | fleet/README.md |

---

## 执行纪律

1. **先读后写** — 不做假设，不凭记忆写代码
2. **验证后报告** — 跑完测试再报告完成，不猜
3. **询问优于猜测** — 不确定时确认
4. **不微观管理** — 主 Agent 给方向不给具体实现，子 Agent 有自主权

## Nexus 特有约束

5. **深度限制** — 子 Agent 不能再 spawn 孙 Agent（深度限制为 1）
6. **工具边界** — 遵守 capability_mode 限制（read-only / execute / read-write / all）
7. **Plan Mode 感知** — 主 Agent 在 Plan Mode 时，子 Agent 不受影响（各自独立的 plan mode tracker）
