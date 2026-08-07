# Bug 报告：DeepSeek responses 流式搜索项反序列化失败，整轮对话中止

> 状态：已修复并发布（v0.3.4，2026-08-07）｜报告日期：2026-08-06 ｜报告人：agent-dev 会话
> 复现环境：`F:\Sage-home` 安装版 `nexus 0.2.105 (504be8e)`，flash 模型走 responses 后端 + `supports_backend_search = true`

## 摘要

对话中途（模型决定发起联网搜索时）整轮采样以 `serialization error: missing field `action`` 中止，随后出现 `Turn failed in 1m29s: Internal error: {"message": "serialization error: missing field `action`", "promptUsage": {...}}`。

**不是回归**：git 历史证明 sampler 的 `client.rs` / `stream/responses.rs` 在 v0.3.2（775662d/3ed8e63）之后未改动，昨天信箱/帧率/dashboard 的提交（504be8e、b70bdfa、cd25ab7、81483d1、359aaf8）均不涉及此处。根因是 async-openai 0.33.1 的 `WebSearchToolCall` 模型与 DeepSeek 实际下发的 `web_search_call` 输出项 schema 不兼容。

## 复现

对 DeepSeek responses 端点（`POST https://api.deepseek.com/responses`）发起带 `{"type":"web_search"}` 工具的流式请求，实测 720KB / 3158 个 SSE 事件中，`web_search_call` 输出项有两种形状，async-openai 均解析不了：

1. **in_progress 项缺 `action`**（`response.output_item.added`，6 个/次搜索）：

```json
{"type":"response.output_item.added","item":{"type":"web_search_call","id":"call_00_...","status":"in_progress"},"output_index":1,"sequence_number":120}
```

async-openai 的 `WebSearchToolCall { action(必填), id, status }` 解析即报 `missing field `action``——这就是用户看到的错误。

2. **completed 项 `action.queries` 是数组**（`response.output_item.done`，6 个/次搜索）：

```json
{"type":"web_search_call","id":"call_00_...","status":"completed","action":{"type":"search","queries":["2026 Paris Olympics news",...]}}
```

async-openai 的 `WebSearchActionSearch { query: String, ... }` 只认单数 `query`，报 `missing field `query``。

## 根因分析

**文件**：`nexus/crates/codegen/nexus-sampler/src/client.rs` 的 `deserialize_response_event`（约 L99）

SSE 事件逐个 `serde_json::from_str::<rs::ResponseStreamEvent>` 反序列化。失败时的兜底只清理 `/response/tools` 数组（原为兼容 xAI 的 `x_search` 工具），**不处理** `output_item` 事件顶层的 `item` 字段，也不清理 `/response/output`。于是任意一个 action-less 的 `web_search_call` 输出项直接让整轮 SSE 流返回 `SamplingError::Serialization`，`stream_responses` 收到 Err 即 yield `Failed` 并 `return`——整轮对话中止。

链路：`deserialize_response_event` → `Err(SamplingError::Serialization)` → `stream/responses.rs` L150-159 的 `Err(err) => yield Failed; return`。

## 修复

`deserialize_response_event` 的兜底改为三层：

1. 保留原 `/response/tools` 清理；
2. 新增 `/response/output` 清理（全响应事件回显的坏输出项直接剔除）；
3. 新增对 `output_item.added/in_progress/done` 顶层 `item` 的处理：
   - `web_search_call` 且 `action.queries` 为数组时，把第一个 query 重映射为 `action.query`——completed 项因此仍能解析为 `WebSearchCall`，`ResponseOutputItemDone` 处理分支照常发出 `BackendToolCallCompleted`，搜索进度不丢；
   - 其余仍无法解析的项（含 action-less 的 in_progress 项）替换为中立 `message` 项——解码器忽略 message，单条坏项不再拖垮整轮。

**验证**：新增 3 个 `deserialize_response_event` 单元测试 + 1 个 `stream_responses` 集成测试（`deepseek_web_search_output_items_stream_on_and_complete`）；并用抓包的全部 3158 个真实 SSE 事件跑通新反序列化（0 失败）。`cargo test -p nexus-sampler --lib` 全绿（158 通过），`cargo check -p nexus-shell` 通过。

## 影响范围

- 仅影响走 **responses 后端 + 后端搜索**（`supports_backend_search = true`）的模型，且只在模型发起联网搜索时触发（所以是"聊一半才出现"）
- chat_completions 后端（本地 web_search 工具）不受影响
- 修复对任何下发 async-openai 无法建模输出项的后端都有益（未知工具类型不再中断流）

## 备注

- 用户配置无需修改：flash 的 `api_backend = "responses"` + 无 `/v1` 的 base_url + `supports_backend_search = true` 本身合法（实测 DeepSeek responses 端点可用），问题是代码容错不足
- 若后续 release，需重新构建并替换 `F:\Sage-home\bin\nexus.exe` 才生效
