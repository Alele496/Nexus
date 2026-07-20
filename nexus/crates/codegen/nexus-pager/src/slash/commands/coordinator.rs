//! `/coordinator` — multi-agent coordinator mode.
//!
//! Subcommands:
//! - `/coordinator start` — enter coordinator mode with multi-agent orchestration
//! - `/coordinator stop` — exit coordinator mode
//! - `/coordinator status` — show current coordinator status
//!
//! When coordinator mode is active, the system prompt instructs the model to
//! act as a coordinator that can decompose complex tasks and distribute work
//! to worker subagents via the `task` tool.

use crate::slash::command::{CommandExecCtx, CommandResult, SlashCommand};
use agent_client_protocol as acp;

pub struct CoordinatorCommand;

impl CoordinatorCommand {
    const COORDINATOR_PROMPT: &str = "\
你现在是 **Nexus Coordinator（协调者）** 模式。

## 你的角色

你是一个多 Agent 协调者。你有能力：
1. **分析任务** — 将复杂任务分解为多个独立的子任务
2. **分配工作** — 使用 `task` 工具将子任务分发给 Worker Agent 并行处理
3. **汇总结果** — 收集所有 Worker 的输出，整合为统一的报告

## 工作流程

1. 仔细分析用户的需求
2. 将需求拆分为可以并行执行的独立子任务
3. 使用 `task` 工具为每个子任务创建 Worker Agent（subagent_type: \"general-purpose\"）
4. 收集所有 Worker 的完成结果
5. 汇总输出，向用户报告最终结果

## 注意事项
- Worker Agent 不能进一步创建子 Agent（深度限制为 1）
- 尽量让子任务之间相互独立，以便并行执行
- 如果子任务之间有依赖，按顺序分阶段执行
- 对每个 Worker 给出清晰、具体的指令";

    const STOP_PROMPT: &str = "已退出 Coordinator 模式。你现在是普通的 Nexus 编程助手。";
}

impl SlashCommand for CoordinatorCommand {
    fn name(&self) -> &str {
        "coordinator"
    }

    fn description(&self) -> &str {
        "多 Agent 协调者模式：任务拆分 → Worker 并行执行 → 结果汇总"
    }

    fn usage(&self) -> &str {
        "/coordinator [start|stop|status]"
    }

    fn takes_args(&self) -> bool {
        true
    }

    fn arg_placeholder(&self) -> Option<&str> {
        Some("start|stop|status")
    }

    fn session_scoped(&self) -> bool {
        true
    }

    fn run(&self, _ctx: &mut CommandExecCtx, args: &str) -> CommandResult {
        match args.trim() {
            "" | "start" => {
                // Enter coordinator mode — inject coordinator system prompt.
                let block = acp::ContentBlock::Text(acp::TextContent::new(Self::COORDINATOR_PROMPT));
                CommandResult::InjectSkill {
                    display_text: "已进入 Coordinator 模式 — 你将作为多 Agent 协调者工作"
                        .to_string(),
                    prompt_blocks: vec![block],
                    display_as_skill: true,
                    scheduled_task_preview: None,
                }
            }
            "stop" => {
                let block = acp::ContentBlock::Text(acp::TextContent::new(Self::STOP_PROMPT));
                CommandResult::InjectSkill {
                    display_text: "已退出 Coordinator 模式".to_string(),
                    prompt_blocks: vec![block],
                    display_as_skill: true,
                    scheduled_task_preview: None,
                }
            }
            "status" => CommandResult::Message(
                "Coordinator 模式需要配合 /coordinator start 启动。\n\
                 启动后，Agent 会将复杂任务拆分为子任务并分发给 Worker 并行处理。"
                    .to_string(),
            ),
            other => CommandResult::Error(format!(
                "未知子命令: {other}。用法: /coordinator [start|stop|status]"
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::acp::model_state::ModelState;
    use crate::app::bundle::BundleState;
    use crate::settings::PagerLocalSnapshot;

    fn ctx<'a>(models: &'a ModelState, bundle: &'a BundleState) -> CommandExecCtx<'a> {
        CommandExecCtx {
            models,
            session_id: None,
            bundle_state: bundle,
            screen_mode: crate::app::ScreenMode::Inline,
            pager_state: PagerLocalSnapshot::default(),
        }
    }

    #[test]
    fn metadata() {
        let cmd = CoordinatorCommand;
        assert_eq!(cmd.name(), "coordinator");
        assert!(cmd.takes_args());
        assert!(!cmd.description().is_empty());
        assert!(!cmd.usage().is_empty());
    }

    #[test]
    fn start_injects_skill() {
        let (models, bundle) = (ModelState::default(), BundleState::default());
        let mut c = ctx(&models, &bundle);
        match CoordinatorCommand.run(&mut c, "") {
            CommandResult::InjectSkill {
                display_text,
                display_as_skill: true,
                ..
            } => assert!(display_text.contains("Coordinator")),
            other => panic!("expected InjectSkill, got {other:?}"),
        }
    }

    #[test]
    fn start_explicit_injects_skill() {
        let (models, bundle) = (ModelState::default(), BundleState::default());
        let mut c = ctx(&models, &bundle);
        match CoordinatorCommand.run(&mut c, "start") {
            CommandResult::InjectSkill {
                display_text,
                display_as_skill: true,
                ..
            } => assert!(display_text.contains("Coordinator")),
            other => panic!("expected InjectSkill, got {other:?}"),
        }
    }

    #[test]
    fn stop_injects_skill() {
        let (models, bundle) = (ModelState::default(), BundleState::default());
        let mut c = ctx(&models, &bundle);
        match CoordinatorCommand.run(&mut c, "stop") {
            CommandResult::InjectSkill {
                display_text,
                display_as_skill: true,
                ..
            } => assert!(display_text.contains("退出")),
            other => panic!("expected InjectSkill, got {other:?}"),
        }
    }

    #[test]
    fn status_returns_message() {
        let (models, bundle) = (ModelState::default(), BundleState::default());
        let mut c = ctx(&models, &bundle);
        match CoordinatorCommand.run(&mut c, "status") {
            CommandResult::Message(msg) => assert!(msg.contains("Coordinator")),
            other => panic!("expected Message, got {other:?}"),
        }
    }

    #[test]
    fn unknown_subcommand_errors() {
        let (models, bundle) = (ModelState::default(), BundleState::default());
        let mut c = ctx(&models, &bundle);
        match CoordinatorCommand.run(&mut c, "unknown") {
            CommandResult::Error(msg) => assert!(msg.contains("未知子命令")),
            other => panic!("expected Error, got {other:?}"),
        }
    }
}
