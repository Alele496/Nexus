# Agent-SYS 项目记忆

> 最后更新：2026-07-19
> 维护者：Agent-SYS Lead

## 项目背景

### 愿景
基于开源 grok-build 框架构建完整的多 Agent 编排系统，替代 CCB 闭源方案。让任何人都能自由使用、自由定制。

### 为什么做
CCB（Claude Code Best）是闭源工具，最终解释权在 CC 那边。安全分类器（deepseek-v4-pro）经常阻塞多 Agent 的正常工作流。需要一个开源、可控、可二次开发的替代方案。

### 核心理念
- **开源优先**：所有配置和定义由用户掌控
- **grok-build 为基**：基于 xAI 的 grok-build 框架（Apache-2.0），不重新发明轮子
- **可二次开发**：grok-build 源码在 grok-build-main/，可 fork 修改
- **向下兼容**：Skills、Hooks 保持与 Claude Code 生态的兼容性

## 当前状态

- **阶段**：Step 1 — 项目骨架搭建 + Agent 定义迁移
- **grok-build 版本**：main 分支（2026-07-19 snapshot）
- **agent-crew 参考版本**：v2.0.0 (HEAD: 57193df)

## 架构对应

| agent-crew (CCB) | Agent-SYS (grok-build) |
|---|---|
| `.claude/agents/*.md` | `.grok/agents/*.md` |
| `.claude/workflows/*.js` | `.grok/skills/*/SKILL.md` + Plan Mode |
| `.claude/settings.json` | `.grok/config.toml` + `.grok/hooks/*.json` |
| `project-memory.md` | 本文件 + grok-build Memory 系统 |
| `examples/fleet/` | `fleet/` 目录 |
| CCB Cron | 待设计（系统 crontab + headless mode） |

## 已完成

- [x] 项目目录骨架创建
- [x] 5 个 Agent 定义迁移（developer/reviewer/operator/advisor + AGENTS.md 作为 lead）
- [x] 3 个 Skill 创建（dispatch/ship/council-review）
- [x] 3 个 Persona 定义（security/perf/readability reviewer）
- [x] Hooks 安全检查体系
- [x] 项目约定文档（rules/）

## 待处理

- [ ] grok-build 实际安装和验证
- [ ] 舰队管理方案设计和实现
- [ ] 下游项目同步机制
- [ ] grok-build 二次开发需求评估

## 设计决策

1. **AGENTS.md 即 Lead**：不再单独创建 lead Agent 定义。grok-build 的主会话就是 lead，身份由 AGENTS.md 定义。
2. **Skills 替代 Workflow**：grok-build 没有 JS workflow 脚本系统，用 Skills + 主 Agent 推理 + Plan Mode 替代。
3. **Personas 实现审查维度**：agent-crew 在 prompt 中指定审查维度，grok-build 用 Personas 更干净地分离关注点。
4. **深度限制接受**：grok-build 子 Agent 深度为 1 是设计选择，不是缺陷。避免了 agent-crew 的 TeamCreate 嵌套失控问题。
