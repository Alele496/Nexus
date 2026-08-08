# Bug 报告：Mailbox 标签寻址投递不生效（+ /mailbox register 缺失待确认）

> 状态：已修复（84c3f77，2026-08-08 合并入 agent-dev，未发布）｜报告日期：2026-08-08 ｜报告人：agent-dev 会话（技术主管侧）
> 影响版本：0.2.105 (504be8e)（实测）；相关代码在 mailbox 模块

## 摘要

Mailbox 支持**按标签寻址**（`send_message` 的 `to` 接受标签，如 `agent-sys-lead`），标签注册表也正常工作，但**投递时标签不解析成会话 ID**——按标签发送/回复的消息永远无法被目标会话 `check_mailbox` 读到。会话 ID 寻址路径完整可用（已实测验证）。

## 复现步骤

1. 会话 A 发送消息时带 `senderLabel = "alice"` → 标签注册表写入 `{"alice": "<A 的会话ID>"}` ✅
2. 会话 B 回复时 `to = "alice"`（标签）→ 消息存储时 `to.session_id = "alice"`（标签字符串原样入库）⚠️
3. 会话 A `check_mailbox()` → **空**（`to.session_id` 是 `"alice"`，不等于 A 的真实会话 ID）❌

实测数据（mailbox.json）：
- 回信 `019fdd48f131`：`"to": { "session_id": "agent-sys-lead", "label": null }` —— 标签塞进了 session_id 字段
- 收件方（真实会话 `019fc7ea-...`）`check_mailbox` 返回 "No new messages"

## 根因分析

**文件**：`nexus/crates/codegen/nexus-tools/src/implementations/nexus_build/mailbox/`

1. `types.rs:195-196`：`register_label(session_id, label)` → `self.labels.insert(label, session_id)` —— 标签注册表存在且正确
2. `actor.rs:142`：发送时通过 `senderLabel` 注册发送方标签 —— 注册路径正常
3. **缺失环节**：send 路径处理 `to` 时，未将标签通过 `labels` 注册表解析为会话 ID（`send.rs` 的 `to` 直接存入 `session_id` 字段）
4. `types.rs` 的 `inbox_for(session_id)` 只按 `m.to.session_id == session_id` 精确过滤 —— 标签字符串永不匹配真实会话 ID

## 修复建议（二选一或都做）

1. **send 时解析**：`to` 若命中 `labels` 注册表（或格式上是标签），解析为对应 `session_id` 再存储。发信方可用标签，存储层落真实会话 ID
2. **check 时回查**：`inbox_for` 增加标签回查——`labels.iter().find(|(label, sid)| sid == session_id)` 收集该会话的所有标签，匹配 `to.session_id ∈ 标签 ∪ {session_id}`

建议同时支持"按标签收件箱"显示（check_mailbox 返回时展示发件人标签）。

## 修复落实（84c3f77，两条建议均已实现）

1. **send 时解析** ✅ `send.rs`：`to` 非 ID 形状时经 `MailboxCommand::ResolveLabel` 查注册表，命中则 `AgentAddress::with_label(session_id, label)`——存储层落真实会话 ID 且保留标签显示名；ID 形状收件人永不走注册表（防 legacy 恶意条目劫持）
2. **check 时回查** ✅ `types.rs` `inbox_for`：按标签地址入库的旧消息（`to.session_id` 存标签原文）经反向查找投递到标签所属会话；仅标签形状字符串参与反向查找，ID 寻址消息永不被标签 siphon
3. **注册表加固** ✅ `register_label` 拒绝空/超长/ID 形状/已被占用 label，上限 256 条；`ResolveLabel` 对 ID 形状直接返回 None

验证：`cargo test -p nexus-tools --lib -- mailbox` 16 通过（含新增 4 个：标签回查投递、ID 形状拒绝、siphon 防护、actor 层 ID 形状忽略）。

## 附带发现：`/mailbox register` 疑似未实现

- `send.rs:77` 文档写到"registered label (set with sender_label or **via /mailbox register**)"
- 但在 nexus-pager 源码中**未找到 `/mailbox` 命令的实现**（搜索 `'/mailbox'`、`MailboxRegister` 无命中）
- 待确认：命令是否在别的 crate，还是文档先行、命令未做。若未做，建议补上（用户需要手动给会话命名，而不是只有"发消息时自报家门"）

## 用户诉求（产品侧）

- 希望**手动给每个 Agent 会话命名**（如"build-agent"、"memory-agent"），之后按名字寻址/找人
- 对应能力：`/mailbox register <名字>`（手动注册标签）+ 标签投递解析（本报告主 bug）

## 验证方法

1. 修复后：A 注册标签 `alice`，B 回复 `to="alice"`，A `check_mailbox` 应能读到
2. `/mailbox register`（若补实现）：手动注册 → 立即生效 → 其他会话可按该名字寻址

## 备注

- 会话 ID 寻址（主路径）已验证可用：发送 delivered → 落盘共享 mailbox.json → 接收方 check 读到（read 状态）
- 跨进程共享存储修复（81483d1）本身有效；本报告是它之上的寻址层缺口
