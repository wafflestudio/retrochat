use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
};
use std::collections::{HashMap, HashSet};

use crate::models::{EdgeType, Flowchart, FlowchartNode};

/// Renders a flowchart as ASCII art with Unicode box characters
pub struct FlowchartRenderer {
    max_width: usize,
}

impl FlowchartRenderer {
    pub fn new(max_width: usize) -> Self {
        Self { max_width }
    }

    /// Render flowchart to lines for TUI display
    pub fn render(&self, flowchart: &Flowchart) -> Vec<Line<'static>> {
        let mut lines = Vec::new();

        if flowchart.nodes.is_empty() {
            lines.push(Line::from(vec![Span::styled(
                "Empty flowchart",
                Style::default().fg(Color::Gray),
            )]));
            return lines;
        }

        // Build adjacency lists
        let mut outgoing: HashMap<&str, Vec<&str>> = HashMap::new();
        let mut incoming: HashMap<&str, Vec<&str>> = HashMap::new();
        let mut edge_types: HashMap<(&str, &str), EdgeType> = HashMap::new();

        for edge in &flowchart.edges {
            outgoing
                .entry(&edge.from_node)
                .or_default()
                .push(&edge.to_node);
            incoming
                .entry(&edge.to_node)
                .or_default()
                .push(&edge.from_node);
            edge_types.insert((&edge.from_node, &edge.to_node), edge.edge_type.clone());
        }

        // Topological sort to get rendering order
        let order = self.topological_sort(&flowchart.nodes, &outgoing);

        // Render each node in order
        for (idx, node_id) in order.iter().enumerate() {
            if let Some(node) = flowchart.get_node(node_id) {
                // Render the node
                self.render_node(&mut lines, node, idx + 1);

                // Render connections to next nodes based on edge types
                if let Some(targets) = outgoing.get(node_id.as_str()) {
                    if targets.len() == 1 {
                        // Check if this is a loop
                        let target = targets[0];
                        if let Some(edge_type) = edge_types.get(&(node_id.as_str(), target)) {
                            match edge_type {
                                EdgeType::Loop => {
                                    self.render_loop(&mut lines);
                                }
                                _ => {
                                    self.render_sequential_arrow(&mut lines);
                                }
                            }
                        } else {
                            self.render_sequential_arrow(&mut lines);
                        }
                    } else if targets.len() > 1 {
                        // Branch
                        self.render_branch(&mut lines, targets.len());
                    }
                }

                // Check if this is a merge point
                if let Some(sources) = incoming.get(node_id.as_str()) {
                    if sources.len() > 1 && idx > 0 {
                        // This was a merge, add visual indicator before the node
                        // (We'll handle this by detecting it before rendering the node)
                    }
                }
            }
        }

        lines
    }

    fn render_node(&self, lines: &mut Vec<Line<'static>>, node: &FlowchartNode, number: usize) {
        let label = self.truncate_label(&node.label, self.max_width.saturating_sub(8));
        
        // Calculate proper box width based on content
        let content = format!("{}. {}", number, label);
        let box_width = content.chars().count() + 4; // 2 spaces on each side

        // Top border
        lines.push(Line::from(vec![Span::styled(
            format!("┌{}┐", "─".repeat(box_width.saturating_sub(2))),
            Style::default().fg(Color::Cyan),
        )]));

        // Content with proper padding
        lines.push(Line::from(vec![Span::styled(
            format!("│ {} │", content),
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        )]));

        // Bottom border
        lines.push(Line::from(vec![Span::styled(
            format!("└{}┘", "─".repeat(box_width.saturating_sub(2))),
            Style::default().fg(Color::Cyan),
        )]));
    }

    fn render_sequential_arrow(&self, lines: &mut Vec<Line<'static>>) {
        lines.push(Line::from(vec![Span::styled(
            "     │",
            Style::default().fg(Color::Gray),
        )]));
        lines.push(Line::from(vec![Span::styled(
            "     ▼",
            Style::default().fg(Color::Yellow),
        )]));
    }

    fn render_branch(&self, lines: &mut Vec<Line<'static>>, branch_count: usize) {
        lines.push(Line::from(vec![Span::styled(
            "     │",
            Style::default().fg(Color::Gray),
        )]));

        // Branch point
        if branch_count == 2 {
            lines.push(Line::from(vec![Span::styled(
                " ┌───┴───┐",
                Style::default().fg(Color::Magenta),
            )]));
            lines.push(Line::from(vec![Span::styled(
                " │       │",
                Style::default().fg(Color::Magenta),
            )]));
            lines.push(Line::from(vec![Span::styled(
                " ▼       ▼",
                Style::default().fg(Color::Yellow),
            )]));
        } else {
            // More than 2 branches - simplified representation
            let branch_line = format!(
                " ├{}┤ {} branches",
                "─".repeat(5),
                branch_count
            );
            lines.push(Line::from(vec![Span::styled(
                branch_line,
                Style::default().fg(Color::Magenta),
            )]));
        }
    }

    fn render_loop(&self, lines: &mut Vec<Line<'static>>) {
        lines.push(Line::from(vec![Span::styled(
            "     │",
            Style::default().fg(Color::Gray),
        )]));
        lines.push(Line::from(vec![Span::styled(
            "     ▼",
            Style::default().fg(Color::Yellow),
        )]));
        lines.push(Line::from(vec![Span::styled(
            " ┌─────┐",
            Style::default().fg(Color::Red),
        )]));
        lines.push(Line::from(vec![Span::styled(
            " │ LOOP│",
            Style::default().fg(Color::Red),
        )]));
        lines.push(Line::from(vec![Span::styled(
            " └─────┘",
            Style::default().fg(Color::Red),
        )]));
    }

    fn truncate_label(&self, label: &str, max_len: usize) -> String {
        if label.len() <= max_len {
            label.to_string()
        } else {
            format!("{}...", &label[..max_len.saturating_sub(3)])
        }
    }

    /// Simple topological sort for rendering order
    fn topological_sort(
        &self,
        nodes: &[FlowchartNode],
        outgoing: &HashMap<&str, Vec<&str>>,
    ) -> Vec<String> {
        let mut visited = HashSet::new();
        let mut result = Vec::new();
        let mut in_degree: HashMap<&str, usize> = HashMap::new();

        // Calculate in-degrees
        for node in nodes {
            in_degree.entry(&node.id).or_insert(0);
        }

        for targets in outgoing.values() {
            for target in targets {
                *in_degree.entry(target).or_insert(0) += 1;
            }
        }

        // Find nodes with in-degree 0 (start nodes)
        let mut queue: Vec<&str> = in_degree
            .iter()
            .filter(|(_, &degree)| degree == 0)
            .map(|(&node, _)| node)
            .collect();

        // BFS
        while let Some(node_id) = queue.pop() {
            if visited.contains(node_id) {
                continue;
            }

            visited.insert(node_id);
            result.push(node_id.to_string());

            if let Some(targets) = outgoing.get(node_id) {
                for &target in targets {
                    if let Some(degree) = in_degree.get_mut(target) {
                        *degree = degree.saturating_sub(1);
                        if *degree == 0 {
                            queue.push(target);
                        }
                    }
                }
            }
        }

        // Add any remaining nodes (disconnected or cyclic)
        for node in nodes {
            if !visited.contains(node.id.as_str()) {
                result.push(node.id.clone());
            }
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{EdgeType, Flowchart, FlowchartEdge, FlowchartNode, MessageRef, NodeType};

    #[test]
    fn test_render_simple_flowchart() {
        let nodes = vec![
            FlowchartNode {
                id: "1".to_string(),
                label: "Start".to_string(),
                message_refs: vec![],
                node_type: NodeType::Start,
                description: None,
            },
            FlowchartNode {
                id: "2".to_string(),
                label: "Process data".to_string(),
                message_refs: vec![],
                node_type: NodeType::Action,
                description: None,
            },
            FlowchartNode {
                id: "3".to_string(),
                label: "End".to_string(),
                message_refs: vec![],
                node_type: NodeType::End,
                description: None,
            },
        ];

        let edges = vec![
            FlowchartEdge {
                from_node: "1".to_string(),
                to_node: "2".to_string(),
                edge_type: EdgeType::Sequential,
                label: None,
            },
            FlowchartEdge {
                from_node: "2".to_string(),
                to_node: "3".to_string(),
                edge_type: EdgeType::Sequential,
                label: None,
            },
        ];

        let flowchart = Flowchart::new("test-session".to_string(), nodes, edges);
        let renderer = FlowchartRenderer::new(40);
        let lines = renderer.render(&flowchart);

        assert!(!lines.is_empty());
        // Should have boxes for 3 nodes + arrows
        assert!(lines.len() > 9); // At least 3 nodes * 3 lines each
    }

    #[test]
    fn test_render_branch() {
        let nodes = vec![
            FlowchartNode {
                id: "1".to_string(),
                label: "Start".to_string(),
                message_refs: vec![],
                node_type: NodeType::Start,
                description: None,
            },
            FlowchartNode {
                id: "2".to_string(),
                label: "Branch A".to_string(),
                message_refs: vec![],
                node_type: NodeType::Action,
                description: None,
            },
            FlowchartNode {
                id: "3".to_string(),
                label: "Branch B".to_string(),
                message_refs: vec![],
                node_type: NodeType::Action,
                description: None,
            },
        ];

        let edges = vec![
            FlowchartEdge {
                from_node: "1".to_string(),
                to_node: "2".to_string(),
                edge_type: EdgeType::Branch,
                label: None,
            },
            FlowchartEdge {
                from_node: "1".to_string(),
                to_node: "3".to_string(),
                edge_type: EdgeType::Branch,
                label: None,
            },
        ];

        let flowchart = Flowchart::new("test-session".to_string(), nodes, edges);
        let renderer = FlowchartRenderer::new(40);
        let lines = renderer.render(&flowchart);

        assert!(!lines.is_empty());
    }
}
