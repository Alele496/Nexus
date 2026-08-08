# Bug 报告：Dashboard 输入框无法输入字母 `r` 和 `a`（审批快捷键拦截）

> 状态：已修复（359aaf8，2026-08-08 于 agent-dev 确认）｜报告日期：2026-08-03 ｜报告人：agent-dev 会话（技术主管侧）
> 原始来源：用户在 `nexus 0.2.105 (1e69ead)` 真实环境复现

## 摘要

在 Agent Dashboard 的 dispatch 输入框中，裸字母键 **`r` 和 `a` 无法输入**——按键被 dashboard 的"审批快捷键"逻辑无条件拦截，字符永远到不了输入框。中英文输入法均受影响（按键事件在到达 IME 之前即被应用层消费）。

若此时恰好选中一个带待处理权限的行，按 `r`/`a` 还会**真的触发拒绝/批准**该权限请求，属于有数据副作用的隐蔽 bug。

## 复现步骤

1. 打开 Agent Dashboard（`/dashboard`）
2. 在底部 dispatch 输入框（非搜索模式）输入任意包含 `r` 或 `a` 的路径/文本，例如 `F:\git-src\AI-Cognitive-Memory`
3. 观察：所有 `r` 和 `a` 字符被吞掉。实际输入 `AI-Cognitive-Memory` 会得到 `AI-Cognitive-Memoy`（`r` 丢失，后续字符前移补位）
4. 若此时 dashboard 列表中有选中行且该行有待处理权限，按 `r` 会直接执行 **拒绝** 操作

## 影响范围

- 输入框手打任何含 `r`/`a` 的内容都会丢字（路径、命令、搜索词等）
- 快捷键冲突：`r` = reject、`a` = approve 的快捷语义与文本输入冲突，无任何"输入框聚焦时放行"的保护
- 中文 IME：拼音输入需要 `r` 键（如 ren），同样被吃，无法组词
- 已确认版本：安装版 `nexus 0.2.105 (1e69ead)` 与 `agent-dev` 分支 HEAD 均存在此问题（`git show 1e69ead:` 验证两处代码一致）

## 根因分析

**文件**：`nexus/crates/codegen/nexus-pager/src/views/dashboard/state.rs`

**位置 1**：`handle_key`（起始行 3048）内，第 **3344–3359** 行，审批快捷键拦截块：

```rust
// Approval shortcuts: a = approve, r = reject, Shift+A = approve all.
// Works on the selected row when it's a NeedsInput agent.
// Skip during search/filter mode so typing 'a' or 'r' enters search text.
if !self.search_mode
    && (key.modifiers.is_empty() || key.modifiers.contains(KeyModifiers::SHIFT))
{
    match key.code {
        KeyCode::Char('a') => {
            return self.handle_dashboard_approval_key(key, registry);
        }
        KeyCode::Char('r') if key.modifiers.is_empty() => {
            return self.handle_dashboard_approval_key(key, registry);
        }
        _ => {}
    }
}
```

**关键问题**：该块只豁免了 `search_mode`，**没有检查 dispatch 输入框是否聚焦/是否有文本**。它在按键流中位于输入框 fall-through（第 **3584** 行 `self.dispatch.handle_key(key)`）之前，因此裸 `a`/`r` 永远到不了输入框。

**位置 2**：`handle_dashboard_approval_key`（第 3626–3655 行）：

```rust
if self.selected.is_some() {
    match key.code {
        KeyCode::Char('a') => return InputOutcome::Action(Action::DashboardApproveSelected),
        KeyCode::Char('r') => return InputOutcome::Action(Action::DashboardRejectSelected),
        _ => {}
    }
}
InputOutcome::Unchanged   // ← 无选中行时按键被静默吞掉（Unchanged = 已消费，不落到输入框）
```

无选中行时返回 `InputOutcome::Unchanged`（按键被消费且无效果）；有选中行时直接触发审批动作。

**对照**：同一函数内其他区块（section fold 第 3208 行、overflow 第 3234 行、open_row_detail 第 3261 行）都用 `prompt_empty || self.list_focused` 门控（`prompt_empty` 定义于第 3138 行 `self.dispatch.text().is_empty()`），唯独审批快捷键块漏了这个门控——属于明显遗漏。

## 修复建议

给审批拦截块加上与同函数其他区块一致的门控——**仅在未在输入框打字时生效**：

```rust
if !self.search_mode
    && (prompt_empty || self.list_focused)
    && (key.modifiers.is_empty() || key.modifiers.contains(KeyModifiers::SHIFT))
{
    match key.code {
        KeyCode::Char('a') => {
            return self.handle_dashboard_approval_key(key, registry);
        }
        KeyCode::Char('r') if key.modifiers.is_empty() => {
            return self.handle_dashboard_approval_key(key, registry);
        }
        _ => {}
    }
}
```

可选加固：即使满足门控，也仅在选中行确实存在 `NeedsInput` 待处理权限时才触发（避免空转消费按键）。

## 回归测试建议

- 输入框聚焦且有文本时，按 `a`/`r` → 字符应正常插入输入框（新增断言）
- 列表聚焦 + 选中行 + 有待处理权限时，按 `r` → 触发 `DashboardRejectSelected`（保留现有语义）
- 粘贴（Ctrl+V）通路不受影响（走第 3148 行 `handle_paste_key_deferred`，直读剪贴板插入）

## 已验证的绕过方案（修复前可用）

1. **位置选择器**：`/cd` 不带参数打开位置选择器——其输入框完全接管输入（`handle_input_with_paste_provenance` 第 2065 行 gate → `handle_location_picker_input` 第 4058 行 → 独立 `picker` 组件），**不经过**审批拦截，`r`/`a` 可正常输入
2. **Ctrl+V 粘贴**：粘贴走剪贴板直插通路（第 3148–3155 行），绕开按键拦截，完整路径可正常插入

## 备注

- 源码修复后需重新构建才能进入用户运行中的二进制（当前安装版 0.2.105 不受源码改动影响）
- 修复后请按审查闸门（Council 三维护）审查再提交
