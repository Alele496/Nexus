//! `/review` — spawn a reviewer agent to audit another agent's work.
//!
//! # Syntax
//!
//! ```text
//! /review <agent-name>      # review a specific agent's work
//! /review --on              # enable auto-review on Worker completion
//! /review --off             # disable auto-review
//! /review --status          # show current review config
//! ```

use crate::app::actions::{Action, ReviewConfigSubcommand};
use crate::slash::command::{CommandExecCtx, CommandResult, SlashCommand};

pub struct ReviewCommand;

impl SlashCommand for ReviewCommand {
    fn name(&self) -> &str {
        "review"
    }

    fn description(&self) -> &str {
        "生成审查 Agent 审计指定 Agent 的代码（可选自动触发）"
    }

    fn usage(&self) -> &str {
        "/review <agent-name> | --on | --off | --status | --mode light|full | --model <name>"
    }

    fn takes_args(&self) -> bool {
        true
    }

    fn args_required(&self) -> bool {
        false
    }

    fn arg_placeholder(&self) -> Option<&str> {
        Some("<agent-name> | --on | --off | --status")
    }

    fn run(&self, _ctx: &mut CommandExecCtx, args: &str) -> CommandResult {
        let args = args.trim();

        match args {
            "--on" => CommandResult::Action(Action::ReviewConfig {
                subcommand: ReviewConfigSubcommand::On,
            }),
            "--off" => CommandResult::Action(Action::ReviewConfig {
                subcommand: ReviewConfigSubcommand::Off,
            }),
            "--status" => CommandResult::Action(Action::ReviewConfig {
                subcommand: ReviewConfigSubcommand::Status,
            }),
            "--mode full" => CommandResult::Action(Action::ReviewConfig {
                subcommand: ReviewConfigSubcommand::SetMode(
                    crate::app::review::ReviewMode::Full,
                ),
            }),
            "--mode light" => CommandResult::Action(Action::ReviewConfig {
                subcommand: ReviewConfigSubcommand::SetMode(
                    crate::app::review::ReviewMode::Light,
                ),
            }),
            "" => CommandResult::Message(
                "用法: /review <agent-name> | --on | --off | --status | --mode light|full | --model <name>".into(),
            ),
            name => {
                // Check for --model prefix
                if let Some(model) = name.strip_prefix("--model ") {
                    return CommandResult::Action(Action::ReviewConfig {
                        subcommand: ReviewConfigSubcommand::SetModel(model.trim().to_string()),
                    });
                }
                CommandResult::Action(Action::ReviewStart {
                    target_agent_name: name.to_string(),
                })
            }
        }
    }
}
