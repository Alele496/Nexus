//! `/agent-graph` — open the Agent Graph visualization.
//!
//! Displays all active agent sessions and their subagents as a visual
//! node-edge graph. Nodes are colored by status (running, completed,
//! failed, idle). Navigate with arrow keys, press Enter to open an
//! agent, Esc to exit back to the previous view.

use crate::app::actions::Action;
use crate::slash::command::{AppCtx, CommandExecCtx, CommandResult, SlashCommand};

/// Open the Agent Graph visualization view.
pub struct AgentGraphCommand;

impl SlashCommand for AgentGraphCommand {
    fn name(&self) -> &str {
        "agent-graph"
    }

    fn aliases(&self) -> &[&str] {
        &["graph"]
    }

    fn description(&self) -> &str {
        "打开Agent关系图 — 可视化所有Agent及其子Agent的节点关系图"
    }

    fn usage(&self) -> &str {
        "/agent-graph"
    }

    fn takes_args(&self) -> bool {
        false
    }

    fn available_in_minimal(&self) -> bool {
        false
    }

    fn visible(&self, ctx: &AppCtx) -> bool {
        !ctx.screen_mode.is_minimal()
    }

    fn run(&self, _ctx: &mut CommandExecCtx, _args: &str) -> CommandResult {
        CommandResult::Action(Action::OpenAgentGraph)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::acp::model_state::ModelState;
    use crate::app::bundle::BundleState;
    use crate::slash::command::{AppCtx, CommandExecCtx, CommandResult};

    #[test]
    fn run_returns_open_agent_graph_action() {
        let models = ModelState::default();
        let bundle = BundleState::default();
        let mut ctx = CommandExecCtx {
            models: &models,
            session_id: None,
            bundle_state: &bundle,
            screen_mode: crate::app::ScreenMode::Inline,
            pager_state: crate::settings::PagerLocalSnapshot {
                multiline_mode: false,
                yolo_mode: false,
                ..crate::settings::PagerLocalSnapshot::default()
            },
        };
        let cmd = AgentGraphCommand;
        assert!(matches!(
            cmd.run(&mut ctx, ""),
            CommandResult::Action(Action::OpenAgentGraph)
        ));
    }

    #[test]
    fn does_not_take_args() {
        assert!(!AgentGraphCommand.takes_args());
    }

    #[test]
    fn name_is_agent_graph() {
        assert_eq!(AgentGraphCommand.name(), "agent-graph");
    }

    #[test]
    fn aliases_include_graph() {
        assert_eq!(AgentGraphCommand.aliases(), &["graph"]);
    }

    #[test]
    fn not_available_in_minimal() {
        assert!(!AgentGraphCommand.available_in_minimal());
    }

    #[test]
    fn visible_except_minimal() {
        let models = ModelState::default();
        let cmd = AgentGraphCommand;
        let ctx = |screen_mode| AppCtx {
            models: &models,
            cwd: std::path::Path::new("."),
            has_session_announcements: false,
            screen_mode,
        };
        assert!(cmd.visible(&ctx(crate::app::ScreenMode::Fullscreen)));
        assert!(cmd.visible(&ctx(crate::app::ScreenMode::Inline)));
        assert!(!cmd.visible(&ctx(crate::app::ScreenMode::Minimal)));
    }
}
