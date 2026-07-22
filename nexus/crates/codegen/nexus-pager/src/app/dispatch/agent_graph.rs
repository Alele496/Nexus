//! Dispatch handlers for Agent Graph actions.

use crate::app::actions::Effect;
use crate::app::app_view::{ActiveView, AppView};

/// Open the Agent Graph view.
///
/// Builds the graph snapshot from current session state, then
/// switches `active_view` to `AgentGraph`. Records the return
/// target so `ExitAgentGraph` can restore the previous view.
pub(super) fn dispatch_open_agent_graph(app: &mut AppView) -> Vec<Effect> {
    // If already open, close it (idempotent toggle).
    if matches!(app.active_view, ActiveView::AgentGraph) {
        return dispatch_exit_agent_graph(app);
    }

    // Record return target.
    app.agent_graph_return = Some(app.active_view);

    // Build the graph data from current sessions.
    app.agent_graph_data = Some(crate::views::agent_graph::state::AgentGraph::build_from_sessions(
        &app.agents,
    ));

    // Initialize graph state if needed.
    let graph_data = app.agent_graph_data.as_mut().unwrap();
    let state = app
        .agent_graph
        .get_or_insert_with(crate::views::agent_graph::AgentGraphState::new);
    state.dirty = true;

    // Pre-select the first node if none selected.
    if state.selected.is_none() && !graph_data.nodes.is_empty() {
        state.selected = Some(0);
    }

    app.active_view = ActiveView::AgentGraph;
    vec![]
}

/// Exit the Agent Graph view, returning to the previous view.
pub(super) fn dispatch_exit_agent_graph(app: &mut AppView) -> Vec<Effect> {
    let return_view = app.agent_graph_return.take().unwrap_or(ActiveView::Welcome);
    // If the return agent is dead, fall back to welcome.
    let target = match return_view {
        ActiveView::Agent(id) if app.agents.contains_key(&id) => return_view,
        ActiveView::AgentDashboard => return_view,
        ActiveView::AgentGraph => ActiveView::Welcome, // safety: don't re-enter graph
        _ => return_view,
    };
    app.active_view = target;
    // Don't drop state so reopen preserves selection and scroll.
    vec![]
}

/// Open a specific agent session from the graph, switching to its agent view.
pub(super) fn dispatch_graph_open_agent(
    app: &mut AppView,
    agent_id: crate::app::agent::AgentId,
) -> Vec<Effect> {
    if !app.agents.contains_key(&agent_id) {
        return vec![];
    }
    if let Some(agent) = app.agents.get_mut(&agent_id) {
        agent.active_subagent = None;
    }
    app.active_view = ActiveView::Agent(agent_id);
    vec![]
}

/// Open a subagent in fullscreen from the graph.
pub(super) fn dispatch_graph_open_subagent(
    app: &mut AppView,
    child_session_id: String,
    parent_agent: crate::app::agent::AgentId,
) -> Vec<Effect> {
    let alive = app
        .agents
        .get(&parent_agent)
        .is_some_and(|a| a.subagent_sessions.contains_key(&child_session_id));
    if !alive {
        return vec![];
    }
    if let Some(agent) = app.agents.get_mut(&parent_agent) {
        crate::app::subagent::ensure_subagent_child_replayed(agent, &child_session_id);
        agent.active_subagent = Some(child_session_id);
    }
    app.active_view = ActiveView::Agent(parent_agent);
    vec![]
}
