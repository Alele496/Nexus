---
name: dispatch
description: 自适应任务调度。分析任务内容→自动选择最佳编排模式(Workflow/Council/Supervisor/Handoff)→调度子Agent执行。当你需要执行不确定类型的任务时使用此技能。
user-invocable: true
argument-hint: "任务描述"
when-to-use: "执行任务、实现功能、修复bug、代码审查、推送代码、分析趋势"
---

# Dispatch — 自适应任务调度

分析任务并自动选择最佳编排模式执行。

## 模式选择决策树

```
分析任务
  ├── 单步骤、确定性 → Workflow（直接执行或 spawn operator）
  ├── 需要多视角评估 → Council（3 个 reviewer 并行，background=true）
  ├── 需要写代码/修bug → Supervisor（动态 spawn developer + reviewer）
  ├── 分类后移交 → Handoff（判断领域→spawn 对应专家）
  └── 多阶段完整交付 → Hybrid（引导用户使用 /ship）
```

## 关键词判断

- **Workflow 触发词**：push, commit, 推送, 发布, tag, version, lint, format
- **Council 触发词**：review, 审查, 检查, verify, 验证, 分析, 评估, 看看
- **Supervisor 触发词**：fix, 修, 实现, 写, 改, feature, refactor, implement, add, build
- **Handoff 触发词**：bsv, bluespec, verilog, 硬件（专业领域问题）
- **Hybrid 触发词**：完整, 全流程, 端到端, 从零, ship, 交付

## Workflow 模式

直接 spawn operator 执行：
```
spawn_subagent(subagent_type="operator", description="执行操作", prompt="<任务描述>")
```

## Council 模式

并行 spawn 3 个 reviewer（background=true），汇总结果：
```
spawn_subagent(subagent_type="reviewer", description="安全审查", prompt="从安全角度审查...", persona="security-reviewer", background=true)
spawn_subagent(subagent_type="reviewer", description="性能审查", prompt="从性能角度审查...", persona="perf-reviewer", background=true)
spawn_subagent(subagent_type="reviewer", description="可读性审查", prompt="从可读性角度审查...", persona="readability-reviewer", background=true)
```

汇总三维结论 → PASS/Council 审查报告。

注意：grok-build 中 persona 通过 subagent 解析时自动应用，不是 spawn_subagent 的直接参数。实际上，在 grok-build 中 persona 通过 role resolution 自动应用。要使用不同 persona 的 reviewer，可以通过在 prompt 中明确指定审查维度来实现。

## Supervisor 模式

动态调度 developer → reviewer 循环：
1. spawn_subagent(subagent_type="developer", ...) 实现
2. spawn_subagent(subagent_type="reviewer", ...) 审查
3. FAIL → 反馈 developer 修复（循环至 PASS）
4. PASS → 完成

## Handoff 模式

先用 advisor 分类问题领域，再移交给对应专家。

## 注意事项

- 子 Agent 深度限制为 1，不能嵌套 spawn 子 Agent
- Council 模式利用 background=true 实现真正的并行
- 审查闸门不可跳过，这是硬规则
