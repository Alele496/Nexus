# Bug 报告：Agent 信箱（mailbox）跨进程投递不生效 + 发送状态永远 "pending"

> 状态：已修复（81483d1，2026-08-08 于 agent-dev 确认）｜报告日期：2026-08-03 ｜报告人：agent-dev 会话（技术主管侧）
> 复现环境：两个独立 nexus 进程（`E:\Git仓库\Agent-SYS` 会话 + `F:/Sage-home/worktrees/git-agent-sys/nexus-agent` worktree 会话），安装版 `nexus 0.2.105 (1e69ead)`

## 摘要

`send_message` 发送的信箱消息**只存在于发送方进程的内存中**，没有持久化，也无法被另一个 nexus 进程（另一个终端窗口的会话）通过 `check_mailbox` 读到。且 `send_message` 返回的投递状态 `status: "pending"` 是**硬编码**的，永远显示 pending，给使用者造成"已投递"的错觉。

源码 `types.rs` 的模块注释宣称 "Messages are persisted via the Resources system"、"Agents send messages to each other asynchronously, **even across sessions**"，但实际行为与文档不符——跨进程投递完全不通。

## 复现步骤

1. 打开两个独立 nexus 会话（两个终端窗口/不同工作目录），记为 A、B
2. 在 A 中调用 `send_message(to = "<B 的会话 ID>", subject = "test", body = "hello")` → 返回 `status: "pending"`
3. 在 B 中调用 `check_mailbox()` → 返回 **0 条消息**
4. 检查磁盘上两个会话的 `resources_state.json`（`F:\Sage-home\sessions\<项目>\<会话ID>\resources_state.json`）→ **均无 Mailbox 字段**，消息从未落盘

## 影响范围

- 跨进程的 Agent 间消息传递静默失效（本次场景：技术主管向 worktree 中的开发 Agent 传递 bug 报告，对方查收为空）
- `send_message` 返回的 `"pending"` 状态无任何真实信息，无法区分"已入队 / 已投递 / 已读"
- 与文档承诺（跨会话异步投递）不符

## 根因分析

**文件 1**：`nexus/crates/codegen/nexus-tools/src/implementations/nexus_build/mailbox/types.rs`

`MailboxState` 注册为资源 `crate::register_resource!("nexus_build", "Mailbox", MailboxState)`，注释声称经 Resources 系统持久化。但实际持久化快照**未包含** Mailbox。

**文件 2**：`.../mailbox/actor.rs`

`MailboxActor::handle_command` 的 `Send`/`Check` 都只操作内存中的 `State<MailboxState>`（`self.resources.lock().await` 后 `state.insert(...)` / `s.messages.filter(...)`），**没有任何写盘动作**。

**文件 3**：`.../mailbox/send.rs`（约第 198 行）

```rust
Ok(SendMessageOutput {
    message_id: sent.id,
    to: sent.to.display_name().to_string(),
    status: "pending".to_string(),   // ← 硬编码，永远 pending
})
```

**文件 4**：`.../mailbox/check.rs`

`check_mailbox` 通过 `SessionIdResource` 取当前会话 ID，向本进程的 MailboxActor 查询 `inbox_for(session_id)`——**只查本进程内存状态**。

**文件 5**：`nexus/crates/codegen/nexus-tools/src/persistence.rs`

`ResourcesPersistence` 把快照写到**每个会话自己的** `resources_state.json`（防抖 500ms + shutdown flush）。实测两份 `resources_state.json`（发送方 1236B、接收方 2336B）均无 Mailbox 字段——MailboxState 未进入持久化快照（可能 State 序列化未覆盖该资源，或从未触发 save）。

**结论**：信箱状态是**进程内单例**。两个独立 nexus 进程各有各的空 MailboxState，消息无法跨进程；即使同进程，若无持久化，进程退出即丢失。

## 修复建议

1. **共享持久化**：把 MailboxState 持久化到一个**全局共享文件**（如 `F:\Sage-home\mailbox.json` 或共享 resources 存储，而非每会话的 `resources_state.json`），所有进程启动时加载、变更时落盘。这样才能支撑"跨会话异步投递"的设计意图
2. **真实状态**：`send_message` 返回真实的投递状态（Pending / Delivered / Read），而非硬编码 `"pending"`；`MessageStatus` 已有该枚举（types.rs），只是输出端写死了
3. **投递确认**：参考 types.rs 架构注释 "Leader → pushes new-message notifications to active recipient sessions"，实现接收方在线时的主动通知，或至少在 send 时能区分"接收方离线，消息已持久化待取"
4. **回归测试**：两个独立 `MailboxActor` 实例（模拟两个进程）之间 send → check 应能读到；进程重启后消息仍在

## 已验证的临时替代方案（修复前可用）

- 跨进程交接信息：写文件到共享仓库目录（如 `docs/bugs/`），接收方在工作时直接读取
- 同进程内（dashboard 派生的子会话共享 SharedResources）：可能可用，但**未验证**，且进程退出即丢失

## 备注

- 与 `/cd` 输入 bug 报告（`docs/bugs/dashboard-r-a-key-interception.md`）为同一批次提交给开发 Agent 的问题
- 修复后请按审查闸门（Council 三维护）审查再提交
