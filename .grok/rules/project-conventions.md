# Agent-SYS 项目约定

## 代码风格

- 遵循项目已有的代码风格，不引入新风格
- 命名清晰、有意义，避免缩写（除非是广泛接受的如 `ctx`、`req`、`res`）
- 文件和目录名使用 kebab-case

## Git 提交规范

遵循 Conventional Commits：
```
type(scope): description

Co-Authored-By: 台阁 <armada@bsv-agent>
```

类型：feat / fix / refactor / docs / chore / test

## 安全规则

- 不在代码中硬编码密钥、token、密码
- 敏感配置通过环境变量注入
- 文件路径操作必须防路径遍历
- 用户输入必须做基本校验
- npm/node 命令执行前确认工作目录正确

## 多 Agent 协作规范

- 子 Agent 通过 spawn_subagent 调度，禁止在 Agent 定义中递归自 spawn
- 审查闸门是硬规则，不可跳过
- 推送前必须有 reviewer 的 PASS 结论
- operator 推送公开仓库前必须向用户二次确认
