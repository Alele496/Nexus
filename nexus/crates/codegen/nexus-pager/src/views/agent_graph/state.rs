//! Agent Graph state types.
//!
//! Data structures for the agent graph visualization:
//! nodes represent agents (sessions), edges represent parent-child relationships.

use crate::app::subagent::SubagentInfo;
use std::sync::Arc;

/// A node in the agent graph, representing one agent session.
#[derive(Debug, Clone)]
pub(crate) struct GraphNode {
    /// Display label (agent name/type).
    pub label: String,
    /// Sub-label (description or status).
    pub sub_label: String,
    /// Unique identifier for this node.
    pub id: NodeId,
    /// Node kind.
    pub kind: NodeKind,
    /// Current status.
    pub status: NodeStatus,
    /// Indices of child nodes in the graph's `nodes` vec.
    pub children: Vec<usize>,
    /// Index of parent node (None for root).
    pub parent: Option<usize>,
    /// Whether this node's children are collapsed (hidden).
    pub collapsed: bool,
}

/// Unique identifier for a graph node.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) enum NodeId {
    /// A top-level agent session.
    Session(usize),
    /// A subagent (child_session_id).
    Subagent(Arc<str>),
}

/// What kind of agent this node represents.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum NodeKind {
    /// Top-level user session.
    Root,
    /// Coordinator (orchestrates workers).
    Coordinator,
    /// Worker agent (spawned by coordinator or parent).
    Worker,
    /// General subagent (reserved for future non-coordinator subagent types).
    #[allow(dead_code)]
    Subagent,
}

/// Visual status of a node.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum NodeStatus {
    /// Idle / waiting for input.
    Idle,
    /// Currently running (turn in progress).
    Running,
    /// Completed successfully.
    Completed,
    /// Failed with error.
    Failed,
    /// Spawned but not yet active (reserved for future pending state tracking).
    #[allow(dead_code)]
    Pending,
    /// Killed / cancelled.
    Cancelled,
}

impl NodeStatus {
    /// Ratatui foreground color for this status.
    pub fn fg_color(&self) -> ratatui::style::Color {
        match self {
            Self::Idle => ratatui::style::Color::Gray,
            Self::Running => ratatui::style::Color::Yellow,
            Self::Completed => ratatui::style::Color::Green,
            Self::Failed => ratatui::style::Color::Red,
            Self::Pending => ratatui::style::Color::Cyan,
            Self::Cancelled => ratatui::style::Color::DarkGray,
        }
    }

    /// Single-character indicator for this status.
    pub fn icon(&self) -> &str {
        match self {
            Self::Idle => "-",
            Self::Running => "R",
            Self::Completed => "OK",
            Self::Failed => "X",
            Self::Pending => ".",
            Self::Cancelled => "K",
        }
    }
}

/// The complete graph snapshot for rendering.
#[derive(Debug, Clone)]
pub(crate) struct AgentGraph {
    /// All nodes in the graph.
    pub nodes: Vec<GraphNode>,
    /// Root node indices (top-level sessions).
    pub roots: Vec<usize>,
    /// Currently selected/highlighted node index.
    pub selected: Option<usize>,
}

impl AgentGraph {
    pub fn new() -> Self {
        Self {
            nodes: Vec::new(),
            roots: Vec::new(),
            selected: None,
        }
    }

    /// Build the graph from pager state.
    ///
    /// `agents` is the `IndexMap<AgentId, AgentView>` from `AppView`.
    /// For each agent, we get its subagent_sessions to build the tree.
    pub fn build_from_sessions(
        agents: &indexmap::IndexMap<crate::app::agent::AgentId, crate::app::agent_view::AgentView>,
    ) -> Self {
        let mut graph = Self::new();

        for (agent_id, agent_view) in agents {
            // Determine status from session state.
            let status = if agent_view.session.state.is_turn_running()
                || agent_view.session.state.is_cancelling()
            {
                NodeStatus::Running
            } else {
                NodeStatus::Idle
            };

            let label = agent_name_label(agent_view);
            let sub_label = agent_status_detail(agent_view);

            let node = GraphNode {
                label,
                sub_label,
                id: NodeId::Session(agent_id.0),
                kind: NodeKind::Root,
                status,
                children: Vec::new(),
                parent: None,
                collapsed: false,
            };

            let node_idx = graph.nodes.len();
            graph.nodes.push(node);
            graph.roots.push(node_idx);

            // Collect subagents for this session.
            // Separate coordinator-type subagents from workers/regular.
            let subagent_entries: Vec<(&String, &SubagentInfo)> =
                agent_view.subagent_sessions.iter().collect();

            for (child_sid, info) in &subagent_entries {
                let sub_status = if info.finished {
                    match info.status.as_deref() {
                        Some("completed") => NodeStatus::Completed,
                        Some("failed") => NodeStatus::Failed,
                        Some("cancelled") => NodeStatus::Cancelled,
                        _ => NodeStatus::Completed,
                    }
                } else {
                    NodeStatus::Running
                };

                let kind = if info.subagent_type.as_ref() == "coordinator" {
                    NodeKind::Coordinator
                } else {
                    NodeKind::Worker
                };

                let (sub_label_text, sub_sub_label) =
                    crate::app::subagent::format_subagent_label(info);

                let child_node = GraphNode {
                    label: sub_label_text,
                    sub_label: sub_sub_label,
                    id: NodeId::Subagent(child_sid.as_str().into()),
                    kind,
                    status: sub_status,
                    children: Vec::new(),
                    parent: Some(node_idx),
                    collapsed: false,
                };

                let child_idx = graph.nodes.len();
                graph.nodes.push(child_node);
                graph.nodes[node_idx].children.push(child_idx);
            }
        }

        // Select first node if any exist.
        if !graph.nodes.is_empty() {
            graph.selected = Some(0);
        }

        graph
    }
}

impl Default for AgentGraph {
    fn default() -> Self {
        Self::new()
    }
}

/// Build a display label for an agent session.
fn agent_name_label(agent_view: &crate::app::agent_view::AgentView) -> String {
    // Use the model name + cwd base name.
    let model = agent_view
        .session
        .models
        .current_model_name()
        .unwrap_or_else(|| "agent".to_string());

    let cwd_name = agent_view
        .session
        .cwd
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| agent_view.session.cwd.to_string_lossy().to_string());

    format!("{} @ {}", model, cwd_name)
}

/// Build a status detail line for an agent session.
fn agent_status_detail(agent_view: &crate::app::agent_view::AgentView) -> String {
    let state = &agent_view.session.state;
    if state.is_turn_running() {
        "running...".to_string()
    } else if state.is_cancelling() {
        "cancelling...".to_string()
    } else {
        let sub_count = agent_view.subagent_sessions.len();
        let running = agent_view
            .subagent_sessions
            .values()
            .filter(|i| !i.finished)
            .count();
        if sub_count > 0 {
            format!("{} subagents ({} running)", sub_count, running)
        } else {
            "idle".to_string()
        }
    }
}
