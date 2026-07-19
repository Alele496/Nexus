---
name: ship
description: 完整交付流程。五阶段混合编排：Council分析→规格编写→用户确认→Supervisor实现→Council审查→用户确认→Workflow推送。用于完整的端到端功能交付。
user-invocable: true
argument-hint: "需求描述"
when-to-use: "完整交付、从零开发、端到端实现新功能、ship"
---

# Ship — 完整交付流程

五阶段混合编排，从需求到推送的完整流程。

## 阶段总览

| 阶段 | 名称 | 模式 | 说明 |
|------|------|------|------|
| 1 | Analyze | Council | 三维可行性分析（技术/风险/工作量） |
| 2 | Spec | Workflow | 基于分析生成技术规格 |
| 🔒 | Checkpoint | — | 暂停，等待用户确认规格 |
| 3 | Implement | Supervisor | 动态调度 developer 实现 |
| 4 | Review | Council | 三维并行审查（安全/性能/可读性） |
| 🔒 | Checkpoint | — | 暂停，等待用户确认推送 |
| 5 | Push | Workflow | 固定检查清单 → commit → push |

## 阶段 1: Analyze（Council 模式）

并行启动 3 个 advisor 做可行性分析：

**技术可行性：**
```
spawn_subagent(subagent_type="advisor", description="技术可行性",
  prompt="分析此需求的技术可行性：当前架构是否支持？技术难点？需要哪些依赖？输出: FEASIBLE/RISKY + 理由。需求：<用户需求>")
```

**风险评估：**
```
spawn_subagent(subagent_type="advisor", description="风险评估",
  prompt="评估此需求的风险：技术风险？对现有功能的影响？向后兼容性？安全风险？输出: LOW/MEDIUM/HIGH + 理由。需求：<用户需求>")
```

**工作量评估：**
```
spawn_subagent(subagent_type="advisor", description="工作量评估",
  prompt="评估此需求的工作量：涉及哪些文件？预估代码量？需要多少轮迭代？输出: SMALL/MEDIUM/LARGE + 理由。需求：<用户需求>")
```

三个分析都使用 `background=true` 实现并行。

汇总三维分析 → 综合判断：
- BLOCKED → 告知用户阻塞原因，需要调整方向
- FEASIBLE/RISKY → 继续进入规格阶段

## 阶段 2: Spec（Workflow 模式）

基于可行性分析生成技术规格：
```
spawn_subagent(subagent_type="advisor", description="生成技术规格",
  prompt="生成技术规格，包含：1.涉及文件清单(具体路径) 2.接口变更 3.实现步骤(按顺序) 4.风险点 5.测试要点。需求+分析摘要：<汇总内容>")
```

## 检查点 1：用户确认

向用户展示：
- 可行性分析摘要
- 技术规格摘要
- 询问："是否按此规格继续实现？"

用户确认后才进入阶段 3。

## 阶段 3: Implement（Supervisor 模式）

主 Agent 作为 Supervisor 动态调度 developer：
1. spawn_subagent(subagent_type="developer", ...) 按规格实现
2. developer 完成后检查结果
3. 如不满意 → 反馈修改意见 → 重新 spawn developer

## 阶段 4: Review（Council 模式）

并行启动 3 个 reviewer：
- reviewer (安全维度)
- reviewer (性能维度)  
- reviewer (可读性维度)

汇总三维结论：
- 全部 PASS → 进入检查点 2
- 任一 FAIL → 反馈 developer 修复，回到阶段 3
- ESCALATE → 告知用户根本性偏差，需人工干预

## 检查点 2：用户确认

展示审查结果，询问是否推送。

## 阶段 5: Push（Workflow 模式）

```
spawn_subagent(subagent_type="operator", description="推送代码",
  prompt="审查已通过。执行推送: 1.git status 2.确认无敏感文件 3.git add [具体文件] 4.git commit 5.git push")
```

## 错误处理

- 任何子 Agent 返回空结果：重试一次，仍失败则报告用户
- Council 分析全部失败：告知用户，建议手动评估
- 审查反复 FAIL：3 轮后建议用户介入
