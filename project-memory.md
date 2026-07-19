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
- **可二次开发**：grok-build 源码在 grok-build-main/，已通过 cargo check 验证
- **向下兼容**：Skills、Hooks 保持与 Claude Code 生态的兼容性

## 当前状态

- **阶段**：Step 1 完成 — 项目骨架 + Agent 定义 + grok-build 编译通过
- **grok-build 版本**：v0.2.105 (commit f87b219)，85 个 crate 全部编译通过
- **git 仓库**：4 个 commit，本地仓库，未推 GitHub

## Git 历史

| Commit | 内容 |
|--------|------|
| `fe89604` | feat: initial Agent-SYS project skeleton (2797 files) |
| `8915a48` | fix: Windows compatibility patches for grok-build |
| `fe027f9` | fix: grok-build Windows compilation — all crates pass cargo check |

## Windows 编译要点

grok-build 在 Windows 下编译需要以下修复：
1. **protoc.exe** — 下载 Windows 版 (v29.3)，替换 Linux DotSlash wrapper
2. **PROTOC 环境变量** — 指向 `bin/protoc.exe`（`find_protoc()` 不搜 `.exe` 后缀）
3. **依赖追踪** — Windows 不支持 `--dependency_out=/dev/stdout`，已补丁跳过
4. **Unicode 路径** — protoc 不能处理含中文的路径，TEMP 必须重定向到纯 ASCII 路径（E:/cargo-tmp）

5. **MSVC link.exe PDB 限制** — `LNK1318: PDB 错误: LIMIT (12)`，257+ object files 超出 MSVC PDB 大小上限
   - 修复：`.cargo/config.toml` 添加 `linker = "rust-lld"`（LLVM 链接器，无此限制）

构建命令：
```bash
# cargo check (快速验证)
TMP="/e/cargo-tmp" TEMP="/e/cargo-tmp" cargo check -p xai-grok-pager-bin
# release build (生成二进制)
TMP="/e/cargo-tmp" TEMP="/e/cargo-tmp" cargo build -p xai-grok-pager-bin --release
# 二进制位置: target/release/xai-grok-pager.exe (123MB)
# 或使用封装脚本
bash scripts/build-grok.sh --release
```

## 已完成

- [x] 项目骨架（AGENTS.md + .grok/ + fleet/ + scripts/）
- [x] 4 个 Agent 定义（developer/reviewer/operator/advisor）
- [x] 3 个 Skills（dispatch/ship/council-review）+ 3 个 Personas + Hooks
- [x] 舰队管理文档（fleet-policy, fleet-registry, spawn-protocol）
- [x] 舰队健康巡检脚本（health-check.sh, cron-runner.sh）
- [x] 配置完整性验证通过（validate-config.sh）
- [x] 本地 git 仓库初始化 + 4 个 commit
- [x] grok-build 源码在 Windows 上编译通过（cargo check，85 crates，7m 46s）
- [x] 构建脚本封装（build-grok.sh, build-grok.ps1）
- [x] `cargo build --release` 成功生成 grok 二进制（v0.2.105, 123MB, 24m 13s）

## 待处理

- [ ] 端到端验证：grok 启动 → 加载 AGENTS.md → spawn developer → reviewer 审查
- [ ] grok-build 二次开发需求评估和技术方案
- [ ] Agent 中台架构设计
- [ ] 下游项目同步机制
- [ ] GitHub 仓库创建

## 设计决策

1. **AGENTS.md 即 Lead** — grok-build 主会话身份，不再有独立 lead agent 定义
2. **Skills 替代 Workflow** — JS 脚本变为 Markdown Skills，编排逻辑由主 Agent 推理执行
3. **Personas 分离审查维度** — security/perf/readability 作为独立 Persona
4. **深度限制 1 是特性** — 防止 agent-crew 的 TeamCreate 嵌套失控
5. **grok-build vendored 入仓库** — 72MB 源码直接纳入 git，方便二次开发
