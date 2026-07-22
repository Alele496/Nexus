//! Tree layout algorithm for the agent graph.
//!
//! Uses a recursive layered layout:
//! - Root nodes are at the top, children below
//! - Siblings are spaced evenly, parent centered above children
//! - Sub-trees are packed left-to-right with padding

use super::state::AgentGraph;
use ratatui::layout::Rect;

/// Minimum width of a node box in characters.
const MIN_NODE_WIDTH: u16 = 20;
/// Maximum width of a node box in characters.
const MAX_NODE_WIDTH: u16 = 40;
/// Node height (label + sub_label lines + 2 borders).
const NODE_HEIGHT: u16 = 5;
/// Horizontal gap between sibling nodes.
const SIBLING_GAP: u16 = 2;
/// Vertical gap between parent and child levels.
pub(crate) const LEVEL_GAP: u16 = 2;
/// Left margin for the graph area.
const MARGIN_LEFT: u16 = 2;
/// Top margin for the graph area.
const MARGIN_TOP: u16 = 1;

/// Computed position for a single node.
#[derive(Debug, Clone, Copy)]
pub(crate) struct NodeLayout {
    /// Bounding rectangle in terminal coordinates.
    pub rect: Rect,
    /// Center X of the node (for edge routing).
    pub center_x: u16,
    /// Top-center Y (for edge routing from parent).
    pub top_y: u16,
    /// Bottom-center Y (for edge routing to children).
    pub bottom_y: u16,
}

/// Complete layout for the graph.
#[derive(Debug, Clone)]
pub(crate) struct GraphLayout {
    /// Per-node layout entries (same indexing as AgentGraph::nodes).
    pub nodes: Vec<NodeLayout>,
    /// Total bounding area of the graph.
    #[allow(dead_code)]
    pub total_area: Rect,
}

/// Compute a node's rendered width based on its label length.
fn node_width(label: &str, sub_label: &str) -> u16 {
    let max_chars = label
        .chars()
        .count()
        .max(sub_label.chars().count())
        .max(4) as u16;
    (max_chars + 4).clamp(MIN_NODE_WIDTH, MAX_NODE_WIDTH) // +4 for border + padding
}

/// Recursively compute widths and positions for a subtree.
///
/// Returns the total width needed for this subtree (including children).
fn layout_subtree(
    graph: &AgentGraph,
    node_idx: usize,
    depth: u16,
    layouts: &mut [Option<NodeLayout>],
) -> u16 {
    let node = &graph.nodes[node_idx];
    let width = node_width(&node.label, &node.sub_label);

    // Layout children first to know their total widths.
    let mut child_total_width: u16 = 0;
    let mut child_widths: Vec<u16> = Vec::with_capacity(node.children.len());

    if !node.collapsed {
        for &child_idx in &node.children {
            let cw = layout_subtree(graph, child_idx, depth + 1, layouts);
            child_widths.push(cw);
            child_total_width += cw;
        }
        if node.children.len() > 1 {
            child_total_width += SIBLING_GAP * (node.children.len() as u16 - 1);
        }
    }

    // This node's total width is max of its own width and children's total.
    let total_width = width.max(child_total_width);

    layouts[node_idx] = Some(NodeLayout {
        rect: Rect::new(0, 0, width, NODE_HEIGHT), // x, y filled in later
        center_x: 0,
        top_y: 0,
        bottom_y: 0,
    });

    total_width
}

/// Assign absolute (x, y) positions to all nodes.
fn assign_positions(
    graph: &AgentGraph,
    node_idx: usize,
    x_offset: u16,
    y_offset: u16,
    layouts: &mut [Option<NodeLayout>],
) {
    let node = &graph.nodes[node_idx];
    let width = node_width(&node.label, &node.sub_label);

    // Center this node over its children, or use x_offset directly for leaves.
    let child_total_width: u16 = if node.collapsed || node.children.is_empty() {
        0
    } else {
        node.children
            .iter()
            .map(|&c| {
                layouts[c]
                    .as_ref()
                    .map(|l| l.rect.width)
                    .unwrap_or(MIN_NODE_WIDTH)
            })
            .sum::<u16>()
            + SIBLING_GAP * (node.children.len().saturating_sub(1) as u16)
    };

    let x = if child_total_width > width {
        // Children are wider, center this node over them.
        x_offset + (child_total_width - width) / 2
    } else {
        // This node is wider than children, centering within parent allocation.
        x_offset
    };

    let y = y_offset;

    if let Some(layout) = &mut layouts[node_idx] {
        layout.rect.x = x;
        layout.rect.y = y;
        layout.center_x = x + width / 2;
        layout.top_y = y;
        layout.bottom_y = y + NODE_HEIGHT;
    }

    // Layout children.
    if !node.collapsed && !node.children.is_empty() {
        let children_y = y + NODE_HEIGHT + LEVEL_GAP;

        // Compute starting x for children (center them under parent).
        let parent_center = x + width / 2;
        let children_start_x = if child_total_width > 0 {
            parent_center.saturating_sub(child_total_width / 2)
        } else {
            x
        };

        let mut child_x = children_start_x;
        for &child_idx in &node.children {
            assign_positions(graph, child_idx, child_x, children_y, layouts);
            let child_width = layouts[child_idx]
                .as_ref()
                .map(|l| l.rect.width)
                .unwrap_or(MIN_NODE_WIDTH);
            child_x += child_width + SIBLING_GAP;
        }
    }
}

/// Compute the complete layout for a graph.
pub(crate) fn compute_layout(graph: &AgentGraph, available_area: Rect) -> GraphLayout {
    let n = graph.nodes.len();
    let mut layouts: Vec<Option<NodeLayout>> = vec![None; n];

    // Phase 1: Compute subtree widths (bottom-up).
    let mut root_widths: Vec<u16> = Vec::with_capacity(graph.roots.len());
    for &root_idx in &graph.roots {
        let w = layout_subtree(graph, root_idx, 0, &mut layouts);
        root_widths.push(w);
    }

    // Phase 2: Position roots horizontally, centered in available area.
    let total_root_width: u16 = root_widths.iter().sum::<u16>()
        + if root_widths.len() > 1 {
            SIBLING_GAP * (root_widths.len() as u16 - 1)
        } else {
            0
        };

    let start_x = available_area.x
        + MARGIN_LEFT
        + if total_root_width < available_area.width.saturating_sub(MARGIN_LEFT * 2) {
            (available_area.width.saturating_sub(MARGIN_LEFT * 2) - total_root_width) / 2
        } else {
            0
        };

    let start_y = available_area.y + MARGIN_TOP;

    let mut x_offset = start_x;
    let mut max_subtree_bottom: u16 = start_y;
    let mut min_x: u16 = u16::MAX;
    let mut max_x: u16 = 0;

    for (i, &root_idx) in graph.roots.iter().enumerate() {
        assign_positions(graph, root_idx, x_offset, start_y, &mut layouts);
        let width = root_widths[i];
        x_offset += width + SIBLING_GAP;

        // Track total bounds.
        if let Some(layout) = &layouts[root_idx] {
            min_x = min_x.min(layout.rect.x);
            max_x = max_x.max(layout.rect.x + layout.rect.width);
            max_subtree_bottom = max_subtree_bottom.max(compute_subtree_bottom(
                graph,
                root_idx,
                &layouts,
            ));
        }
    }

    let nodes: Vec<NodeLayout> = layouts.into_iter().map(|l| l.unwrap_or(NodeLayout {
        rect: Rect::default(),
        center_x: 0,
        top_y: 0,
        bottom_y: 0,
    })).collect();

    let total_area = Rect::new(
        min_x,
        available_area.y,
        (max_x - min_x).max(1),
        (max_subtree_bottom - available_area.y + 1).max(1),
    );

    GraphLayout { nodes, total_area }
}

/// Compute the bottom edge of a subtree (recursive).
fn compute_subtree_bottom(
    graph: &AgentGraph,
    node_idx: usize,
    layouts: &[Option<NodeLayout>],
) -> u16 {
    let node_bottom = layouts[node_idx]
        .as_ref()
        .map(|l| l.bottom_y)
        .unwrap_or(0);

    let children_bottom = graph.nodes[node_idx]
        .children
        .iter()
        .map(|&c| compute_subtree_bottom(graph, c, layouts))
        .max()
        .unwrap_or(0);

    node_bottom.max(children_bottom)
}
