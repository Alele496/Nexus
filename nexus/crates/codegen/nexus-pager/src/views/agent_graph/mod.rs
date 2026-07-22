//! Agent Graph visualization view.
//!
//! Full-screen view showing agent relationships as a node-edge graph.
//! Nodes are colored by status. Keyboard/mouse navigation lets users
//! select nodes and drill into individual agent sessions.
//!
//! Entry points:
//! - `render_agent_graph()` — draw the graph to a ratatui Buffer
//! - `handle_agent_graph_input()` — process keyboard/mouse events
//! - `AgentGraphState` — mutable view state (selection, scroll, detail panel)

#![allow(deprecated)] // ratatui Buffer::get_mut → cell_mut migration WIP

pub(crate) mod input;
pub(crate) mod layout;
pub(crate) mod state;

use layout::{compute_layout, GraphLayout, NodeLayout};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::{Block, Borders, Widget};
use state::{AgentGraph, GraphNode, NodeKind, NodeStatus};

/// View-local mutable state for the agent graph.
#[derive(Debug, Clone)]
pub(crate) struct AgentGraphState {
    /// Currently selected node index.
    pub selected: Option<usize>,
    /// Whether the detail side panel is open.
    pub detail_open: bool,
    /// Horizontal scroll offset.
    pub scroll_x: u16,
    /// Vertical scroll offset.
    pub scroll_y: u16,
    /// Last computed layout (cached).
    pub layout: Option<GraphLayout>,
    /// Whether a refresh is needed.
    pub dirty: bool,
}

impl AgentGraphState {
    pub fn new() -> Self {
        Self {
            selected: None,
            detail_open: false,
            scroll_x: 0,
            scroll_y: 0,
            layout: None,
            dirty: true,
        }
    }

    /// Move selection to the next node (depth-first).
    pub fn select_next(&mut self, graph: &AgentGraph) {
        let n = graph.nodes.len();
        if n == 0 {
            self.selected = None;
            return;
        }
        let next = match self.selected {
            Some(idx) if idx + 1 < n => idx + 1,
            _ => 0,
        };
        self.selected = Some(next);
    }

    /// Move selection to the previous node (reverse depth-first).
    pub fn select_prev(&mut self, graph: &AgentGraph) {
        let n = graph.nodes.len();
        if n == 0 {
            self.selected = None;
            return;
        }
        let prev = match self.selected {
            Some(idx) if idx > 0 => idx - 1,
            _ => n.saturating_sub(1),
        };
        self.selected = Some(prev);
    }

    /// Move selection to the parent of the current node.
    pub fn select_parent(&mut self, graph: &AgentGraph) {
        if let Some(idx) = self.selected {
            if let Some(parent) = graph.nodes[idx].parent {
                self.selected = Some(parent);
            }
        }
    }

    /// Move selection to the first child of the current node.
    pub fn select_child(&mut self, graph: &AgentGraph) {
        if let Some(idx) = self.selected {
            if let Some(&first_child) = graph.nodes[idx].children.first() {
                self.selected = Some(first_child);
            }
        }
    }

    /// Toggle collapse/expand for the selected node.
    pub fn toggle_collapse(&mut self, graph: &mut AgentGraph) {
        if let Some(idx) = self.selected {
            graph.nodes[idx].collapsed = !graph.nodes[idx].collapsed;
            self.dirty = true;
        }
    }

    /// Hit-test: return the node index at the given viewport coordinates.
    /// Coordinates are relative to the graph area (not screen).
    pub fn find_node_at(&self, x: u16, y: u16) -> Option<usize> {
        let layout = self.layout.as_ref()?;
        for (idx, node_layout) in layout.nodes.iter().enumerate() {
            let r = node_layout.rect;
            if x >= r.x && x < r.right() && y >= r.y && y < r.bottom() {
                return Some(idx);
            }
        }
        None
    }
}

impl Default for AgentGraphState {
    fn default() -> Self {
        Self::new()
    }
}

/// Render the agent graph into the given buffer.
pub(crate) fn render_agent_graph(
    area: Rect,
    buf: &mut Buffer,
    graph: &AgentGraph,
    state: &mut AgentGraphState,
) {
    // Clear the area.
    for y in area.top()..area.bottom() {
        for x in area.left()..area.right() {
            buf[(x, y)].reset();
        }
    }

    if graph.nodes.is_empty() {
        render_empty_state(area, buf);
        return;
    }

    // Recompute layout if needed.
    if state.dirty || state.layout.is_none() {
        let layout_area = Rect::new(
            area.x,
            area.y,
            area.width,
            area.height.saturating_sub(2), // reserve bottom bar
        );
        state.layout = Some(compute_layout(graph, layout_area));
        state.dirty = false;
    }

    let layout = state.layout.as_ref().unwrap();

    // Clamp scroll to content bounds so users can't scroll past empty space.
    if let Some(layout) = &state.layout {
        let max_scroll_x = layout
            .total_area
            .width
            .saturating_sub(area.width)
            .saturating_add(4); // small padding
        let max_scroll_y = layout
            .total_area
            .height
            .saturating_sub(area.height.saturating_sub(2));
        state.scroll_x = state.scroll_x.min(max_scroll_x);
        state.scroll_y = state.scroll_y.min(max_scroll_y);
    }

    // Offset area by scroll.
    let scroll_area = Rect::new(
        area.x.saturating_add(state.scroll_x),
        area.y.saturating_add(state.scroll_y),
        area.width,
        area.height.saturating_sub(1),
    );

    // Draw edges first (behind nodes).
    render_edges(graph, layout, buf, scroll_area);

    // Draw nodes.
    for (idx, node) in graph.nodes.iter().enumerate() {
        let node_layout = &layout.nodes[idx];
        let is_selected = state.selected == Some(idx);
        render_node(node, node_layout, is_selected, buf);
    }

    // Draw bottom help bar.
    let help_y = area.bottom().saturating_sub(1);
    let help_area = Rect::new(area.x, help_y, area.width, 1);
    render_help_bar(help_area, buf, state.detail_open);

    // Draw side detail panel if open.
    if state.detail_open {
        if let Some(idx) = state.selected {
            let detail_area = Rect::new(
                area.x + area.width.saturating_sub(40).min(area.width.saturating_sub(4)),
                area.y,
                40.min(area.width),
                area.height.saturating_sub(1),
            );
            render_detail_panel(detail_area, buf, &graph.nodes[idx]);
        }
    }
}

/// Render a single node as a bordered box.
fn render_node(node: &GraphNode, layout: &NodeLayout, selected: bool, buf: &mut Buffer) {
    let rect = layout.rect;
    if rect.width < 4 || rect.height < 3 {
        return;
    }

    let base_style = if selected {
        Style::default()
            .fg(Color::White)
            .bg(Color::DarkGray)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(node.status.fg_color())
    };

    let border_style = if selected {
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(node.status.fg_color())
    };

    // Draw border.
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(border_style);

    let inner = block.inner(rect);

    // Render the block (border lines).
    block.render(rect, buf);

    // Draw the kind icon in the top-left corner.
    let kind_icon = match node.kind {
        NodeKind::Root => "H",
        NodeKind::Coordinator => "C",
        NodeKind::Worker => "W",
        NodeKind::Subagent => "S",
    };
    let kind_area = Rect::new(rect.x + 2, rect.y, 1, 1);
    if kind_area.right() <= rect.right() {
        buf.get_mut(kind_area.x, kind_area.y)
            .set_char(kind_icon.chars().next().unwrap_or('?'))
            .set_style(Style::default().fg(node.status.fg_color()).add_modifier(Modifier::BOLD));
    }

    // Status dot.
    let status_icon = node.status.icon();
    let status_area = Rect::new(rect.right().saturating_sub(3), rect.y, 2, 1);
    if status_area.x < rect.right() {
        for (i, ch) in status_icon.chars().enumerate() {
            let sx = status_area.x + i as u16;
            if sx < rect.right() {
                buf.get_mut(sx, status_area.y)
                    .set_char(ch)
                    .set_style(Style::default().fg(node.status.fg_color()).add_modifier(Modifier::BOLD));
            }
        }
    }

    // Render label (first line of inner area, centered/left-aligned).
    if inner.height > 0 && inner.width > 2 {
        let label_y = inner.y;
        let label_x = inner.x + 1;
        let max_width = inner.width.saturating_sub(2) as usize;

        let truncated: String = node.label.chars().take(max_width).collect();
        for (i, ch) in truncated.chars().enumerate() {
            let cx = label_x + i as u16;
            if cx < inner.right() {
                buf.get_mut(cx, label_y).set_char(ch).set_style(base_style);
            }
        }
    }

    // Render sub-label (second line of inner area).
    if inner.height > 1 && inner.width > 2 {
        let sub_y = inner.y + 1;
        let sub_x = inner.x + 1;
        let max_width = inner.width.saturating_sub(2) as usize;

        let truncated: String = node.sub_label.chars().take(max_width).collect();
        let sub_style = if selected {
            base_style
        } else {
            Style::default().fg(Color::Gray)
        };
        for (i, ch) in truncated.chars().enumerate() {
            let cx = sub_x + i as u16;
            if cx < inner.right() {
                buf.get_mut(cx, sub_y).set_char(ch).set_style(sub_style);
            }
        }
    }
}

/// Draw edges between parent and child nodes using box-drawing characters.
fn render_edges(
    graph: &AgentGraph,
    layout: &GraphLayout,
    buf: &mut Buffer,
    clip_area: Rect,
) {
    for (idx, node) in graph.nodes.iter().enumerate() {
        if node.children.is_empty() || node.collapsed {
            continue;
        }

        let parent = &layout.nodes[idx];
        let parent_bottom = parent.bottom_y;
        let parent_center = parent.center_x;

        // Draw vertical line from parent bottom to elbow point.
        let elbow_y = parent_bottom + (layout::LEVEL_GAP / 2);
        for y in parent_bottom..=elbow_y {
            if y < clip_area.top() || y >= clip_area.bottom() {
                continue;
            }
            if parent_center >= clip_area.left() && parent_center < clip_area.right() {
                buf.get_mut(parent_center, y)
                    .set_char('│')
                    .set_style(Style::default().fg(Color::DarkGray));
            }
        }

        // If multiple children, draw horizontal connector.
        if node.children.len() > 1 {
            let first = &layout.nodes[node.children[0]];
            let last = &layout.nodes[node.children[node.children.len() - 1]];
            let h_start = first.center_x.min(last.center_x);
            let h_end = first.center_x.max(last.center_x);
            for x in h_start..=h_end {
                if elbow_y < clip_area.top() || elbow_y >= clip_area.bottom() {
                    continue;
                }
                if x >= clip_area.left() && x < clip_area.right() {
                    let ch = if x == parent_center {
                        '┼'
                    } else if x == h_start {
                        '┌'
                    } else if x == h_end {
                        '┐'
                    } else {
                        '─'
                    };
                    buf.get_mut(x, elbow_y)
                        .set_char(ch)
                        .set_style(Style::default().fg(Color::DarkGray));
                }
            }
        }

        // Draw vertical lines from elbow to each child top.
        for &child_idx in &node.children {
            let child = &layout.nodes[child_idx];
            let child_center = child.center_x;
            let child_top = child.top_y;

            // Vertical from elbow to child top.
            for y in elbow_y..=child_top {
                if y < clip_area.top() || y >= clip_area.bottom() {
                    continue;
                }
                if child_center >= clip_area.left() && child_center < clip_area.right() {
                    let ch = if y == child_top { '╰' } else { '│' };
                    buf.get_mut(child_center, y)
                        .set_char(ch)
                        .set_style(Style::default().fg(Color::DarkGray));
                }
            }
        }
    }
}

/// Render the bottom help bar.
fn render_help_bar(area: Rect, buf: &mut Buffer, detail_open: bool) {
    // Fill background.
    for x in area.left()..area.right() {
        buf.get_mut(x, area.y)
            .set_style(Style::default().bg(Color::Rgb(30, 30, 40)));
    }

    let help_text = " ↑↓/j,k: move  h/l: parent/child  Enter: view  Space: toggle  c: collapse  d: detail  r: refresh  Esc: exit ";
    let detail_hint = if detail_open { " [d] close detail" } else { "" };
    let combined = format!("{}{}", help_text, detail_hint);

    let start_x = area.x + 1;
    for (i, ch) in combined.chars().enumerate() {
        let cx = start_x + i as u16;
        if cx >= area.right() {
            break;
        }
        buf.get_mut(cx, area.y)
            .set_char(ch)
            .set_style(Style::default().fg(Color::Gray).bg(Color::Rgb(30, 30, 40)));
    }
}

/// Render the detail side panel for a selected node.
fn render_detail_panel(area: Rect, buf: &mut Buffer, node: &GraphNode) {
    // Semi-transparent background.
    for y in area.top()..area.bottom() {
        for x in area.left()..area.right() {
            buf.get_mut(x, y)
                .set_style(Style::default().bg(Color::Rgb(20, 20, 30)));
        }
    }

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan))
        .title(format!(" {}", node.label));

    let inner = block.inner(area);
    block.render(area, buf);

    // Render details inside the panel.
    let kind_str = match node.kind {
        NodeKind::Root => "Root Session",
        NodeKind::Coordinator => "Coordinator",
        NodeKind::Worker => "Worker Agent",
        NodeKind::Subagent => "Subagent",
    };

    let status_str = match node.status {
        NodeStatus::Idle => "Idle",
        NodeStatus::Running => "Running",
        NodeStatus::Completed => "Completed",
        NodeStatus::Failed => "Failed",
        NodeStatus::Pending => "Pending",
        NodeStatus::Cancelled => "Cancelled",
    };

    let lines = vec![
        format!(" Kind:   {}", kind_str),
        format!(" Status: {}", status_str),
        format!(" Label:  {}", node.sub_label),
        String::new(),
        format!(" Children: {}", node.children.len()),
        format!(
            " Collapsed: {}",
            if node.collapsed { "yes" } else { "no" }
        ),
        String::new(),
        " [Enter] Open agent".to_string(),
        " [c] Toggle collapse".to_string(),
        " [d] Close panel".to_string(),
    ];

    let mut y = inner.y;
    for line in &lines {
        if y >= inner.bottom() {
            break;
        }
        let max_w = inner.width.saturating_sub(2) as usize;
        let display: String = line.chars().take(max_w).collect();
        for (i, ch) in display.chars().enumerate() {
            let cx = inner.x + 1 + i as u16;
            if cx < inner.right() {
                buf.get_mut(cx, y)
                    .set_char(ch)
                    .set_style(Style::default().fg(Color::White));
            }
        }
        y += 1;
    }
}

/// Render when there are no agents.
fn render_empty_state(area: Rect, buf: &mut Buffer) {
    let msg = "No active agents. Start a conversation to see the agent graph.";
    let x = area.x + area.width.saturating_sub(msg.len() as u16) / 2;
    let y = area.y + area.height / 2;

    for (i, ch) in msg.chars().enumerate() {
        let cx = x + i as u16;
        if cx < area.right() {
            buf.get_mut(cx, y)
                .set_char(ch)
                .set_style(Style::default().fg(Color::Gray));
        }
    }
}
