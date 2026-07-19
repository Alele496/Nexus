# Fleet Management — 多仓库舰队管理

基于 grok-build 的多项目管理方案。由于 grok-build 是单会话工具，舰队管理通过以下方式实现：

## 架构

```
fleet/                        ← 舰队 HQ 配置目录
├── README.md                 ← 本文件
├── fleet-registry.json       ← 项目注册表（所有被管理项目的元信息）
├── docs/
│   ├── fleet-policy.md       ← 跨项目架构原则
│   └── spawn-protocol.md     ← Agent spawn 行为规范
└── scripts/                  ← 舰队管理脚本（待开发）
    ├── health-check.sh       ← 健康巡检脚本
    └── cron-runner.sh        ← Cron 调度器
```

## 与 CCB 舰队模式的关键区别

| CCB Fleet | Agent-SYS Fleet |
|---|---|
| Steward 在 CCB 内路由 | 无中心 Steward——每个项目独立运行 grok |
| Cron 通过 CCB 内置 | 通过系统 crontab + grok headless mode |
| 路由表在 CLAUDE.md | 路由信息在 fleet-registry.json |
| Steward 只输出文本指令 | 用户可以自己判断去哪 |

## 使用方式

### 1. 注册项目

编辑 `fleet-registry.json`，添加你的项目信息。

### 2. 部署 Agent 团队

在每个项目下复制 Agent-SYS 的 `.grok/` 和 `AGENTS.md`。

### 3. 健康巡检

```bash
# 手动巡检
cd fleet/ && ./scripts/health-check.sh

# 定时巡检（系统 crontab）
# 每 4 小时: 0 */4 * * * cd /path/to/fleet && ./scripts/health-check.sh
# 每天 00:00: 0 0 * * * cd /path/to/fleet && ./scripts/health-check.sh --full
```

### 4. 路由决策

当需要操作某个项目时：
1. 查 `fleet-registry.json` 确认项目路径
2. `cd <项目路径> && grok` 启动该项目的独立会话
3. 该项目的 AGENTS.md 自动加载为 Project Lead

## 未来计划

- [ ] `grok agent headless` 模式下的集中式舰队管理服务
- [ ] ACP 协议实现跨项目 Agent 通信
- [ ] Web Dashboard 查看舰队状态
- [ ] 模板同步机制（agent-crew 模板更新 → 下游项目自动提示）
