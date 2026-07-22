//! Input handling for the Agent Graph view.
//!
//! Keyboard navigation (arrows, j/k, enter, tab, esc, space, c, d)
//! and mouse click support.

use super::state::{AgentGraph, NodeId};
use super::AgentGraphState;
use crate::app::actions::Action;
use crate::app::app_view::InputOutcome;
use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, MouseButton, MouseEventKind};

/// Handle keyboard/mouse input for the agent graph view.
///
/// Returns `InputOutcome` — `Action(...)` to dispatch, or `Changed`/`Unchanged`
/// for visual-only updates.
pub(crate) fn handle_agent_graph_input(
    ev: &Event,
    graph: &mut AgentGraph,
    state: &mut AgentGraphState,
) -> InputOutcome {
    match ev {
        Event::Key(key) if key.kind != KeyEventKind::Release => handle_key(key, graph, state),
        Event::Mouse(mouse) => handle_mouse(mouse, graph, state),
        _ => InputOutcome::Unchanged,
    }
}

fn handle_key(
    key: &KeyEvent,
    graph: &mut AgentGraph,
    state: &mut AgentGraphState,
) -> InputOutcome {
    match key.code {
        // Navigation
        KeyCode::Up | KeyCode::Char('k') => {
            state.select_prev(graph);
            InputOutcome::Changed
        }
        KeyCode::Down | KeyCode::Char('j') => {
            state.select_next(graph);
            InputOutcome::Changed
        }
        KeyCode::Left | KeyCode::Char('h') => {
            state.select_parent(graph);
            InputOutcome::Changed
        }
        KeyCode::Right | KeyCode::Char('l') => {
            state.select_child(graph);
            InputOutcome::Changed
        }
        KeyCode::Tab => {
            state.select_next(graph);
            InputOutcome::Changed
        }
        // Back-tab: previous node
        KeyCode::BackTab => {
            state.select_prev(graph);
            InputOutcome::Changed
        }

        // Action keys
        KeyCode::Enter => {
            // Open/attach to the selected agent.
            if let Some(idx) = state.selected {
                let action = match &graph.nodes[idx].id {
                    NodeId::Session(agent_id) => {
                        Action::GraphOpenAgent(crate::app::agent::AgentId(*agent_id))
                    }
                    NodeId::Subagent(child_sid) => {
                        // Find the parent agent that owns this subagent.
                        let parent_agent = graph.nodes[idx].parent.and_then(|p_idx| {
                            match &graph.nodes[p_idx].id {
                                NodeId::Session(id) => Some(crate::app::agent::AgentId(*id)),
                                _ => None,
                            }
                        });
                        let sid: String = child_sid.as_ref().to_string();
                        if let Some(parent_id) = parent_agent {
                            Action::GraphOpenSubagent {
                                child_session_id: sid,
                                parent_agent: parent_id,
                            }
                        } else {
                            return InputOutcome::Unchanged;
                        }
                    }
                };
                return InputOutcome::Action(action);
            }
            InputOutcome::Unchanged
        }
        KeyCode::Char(' ') => {
            // Toggle expand/collapse.
            state.toggle_collapse(graph);
            InputOutcome::Changed
        }
        KeyCode::Char('c') => {
            state.toggle_collapse(graph);
            InputOutcome::Changed
        }
        KeyCode::Char('d') => {
            state.detail_open = !state.detail_open;
            InputOutcome::Changed
        }
        KeyCode::Char('r') => {
            // Mark dirty so layout is recomputed on next render.
            // The graph data itself is refreshed in AppView::tick().
            state.dirty = true;
            InputOutcome::Changed
        }

        // Exit
        KeyCode::Esc => InputOutcome::Action(Action::ExitAgentGraph),

        _ => InputOutcome::Unchanged,
    }
}

fn handle_mouse(
    mouse: &crossterm::event::MouseEvent,
    graph: &mut AgentGraph,
    state: &mut AgentGraphState,
) -> InputOutcome {
    match mouse.kind {
        MouseEventKind::Down(MouseButton::Left) => {
            // Hit-test using cached layout positions.
            if let Some(idx) = state.find_node_at(mouse.column, mouse.row) {
                if state.selected == Some(idx) {
                    // Double-click: open the agent.
                    let action = match &graph.nodes[idx].id {
                        NodeId::Session(agent_id) => {
                            Action::GraphOpenAgent(crate::app::agent::AgentId(*agent_id))
                        }
                        NodeId::Subagent(child_sid) => {
                            let parent_agent = graph.nodes[idx].parent.and_then(|p_idx| {
                                match &graph.nodes[p_idx].id {
                                    NodeId::Session(id) => {
                                        Some(crate::app::agent::AgentId(*id))
                                    }
                                    _ => None,
                                }
                            });
                            let sid: String = child_sid.as_ref().to_string();
                            if let Some(parent_id) = parent_agent {
                                Action::GraphOpenSubagent {
                                    child_session_id: sid,
                                    parent_agent: parent_id,
                                }
                            } else {
                                return InputOutcome::Unchanged;
                            }
                        }
                    };
                    return InputOutcome::Action(action);
                }
                state.selected = Some(idx);
                InputOutcome::Changed
            } else {
                InputOutcome::Unchanged
            }
        }
        MouseEventKind::ScrollDown => {
            state.select_next(graph);
            InputOutcome::Changed
        }
        MouseEventKind::ScrollUp => {
            state.select_prev(graph);
            InputOutcome::Changed
        }
        _ => InputOutcome::Unchanged,
    }
}
