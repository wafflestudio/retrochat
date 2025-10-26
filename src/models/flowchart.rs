use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A reference to a message or part of a message that belongs to this node
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MessageRef {
    /// Message UUID
    pub message_id: String,
    /// Sequence number in the session
    pub sequence_number: u32,
    /// If this node only covers part of the message content, describe which portion
    pub portion: Option<String>,
}

/// Type of flowchart node
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum NodeType {
    /// A context or action step
    #[serde(rename = "context")]
    Context,
    /// A decision point
    #[serde(rename = "decision")]
    Decision,
    /// An action performed
    #[serde(rename = "action")]
    Action,
    /// Start of the flow
    #[serde(rename = "start")]
    Start,
    /// End of the flow
    #[serde(rename = "end")]
    End,
    /// Tool usage
    #[serde(rename = "tool_use")]
    ToolUse,
    /// An event that occurred
    #[serde(rename = "event")]
    Event,
}

impl std::fmt::Display for NodeType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NodeType::Context => write!(f, "context"),
            NodeType::Decision => write!(f, "decision"),
            NodeType::Action => write!(f, "action"),
            NodeType::Start => write!(f, "start"),
            NodeType::End => write!(f, "end"),
            NodeType::ToolUse => write!(f, "tool_use"),
            NodeType::Event => write!(f, "event"),
        }
    }
}

/// A node in the flowchart representing a single clear step or context
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FlowchartNode {
    /// Unique node ID (within this flowchart)
    pub id: String,
    /// Clear, concise label for this step (e.g., "Create todo-list")
    pub label: String,
    /// Messages that contribute to this node
    pub message_refs: Vec<MessageRef>,
    /// Type of node
    pub node_type: NodeType,
    /// Optional detailed description
    pub description: Option<String>,
}

/// Type of edge connecting nodes
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum EdgeType {
    /// Normal sequential flow
    Sequential,
    /// Multiple flows merging into one (2+ inputs → 1 output)
    Merge,
    /// Flow branching into multiple paths (1 input → 2+ outputs)
    Branch,
    /// Flow returning to a previous node (loop)
    Loop,
}

impl std::fmt::Display for EdgeType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EdgeType::Sequential => write!(f, "sequential"),
            EdgeType::Merge => write!(f, "merge"),
            EdgeType::Branch => write!(f, "branch"),
            EdgeType::Loop => write!(f, "loop"),
        }
    }
}

/// An edge connecting two nodes in the flowchart
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FlowchartEdge {
    /// Source node ID
    pub from_node: String,
    /// Destination node ID
    pub to_node: String,
    /// Type of edge
    pub edge_type: EdgeType,
    /// Optional label for the edge
    pub label: Option<String>,
}

/// A complete flowchart for a session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Flowchart {
    /// Unique flowchart ID
    pub id: String,
    /// Session this flowchart belongs to
    pub session_id: String,
    /// All nodes in the flowchart
    pub nodes: Vec<FlowchartNode>,
    /// All edges in the flowchart
    pub edges: Vec<FlowchartEdge>,
    /// When this flowchart was created
    pub created_at: DateTime<Utc>,
    /// Optional token usage for generation
    pub token_usage: Option<u32>,
}

impl Flowchart {
    pub fn new(session_id: String, nodes: Vec<FlowchartNode>, edges: Vec<FlowchartEdge>) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            session_id,
            nodes,
            edges,
            created_at: Utc::now(),
            token_usage: None,
        }
    }

    pub fn with_token_usage(mut self, token_usage: u32) -> Self {
        self.token_usage = Some(token_usage);
        self
    }

    /// Get a node by ID
    pub fn get_node(&self, node_id: &str) -> Option<&FlowchartNode> {
        self.nodes.iter().find(|n| n.id == node_id)
    }

    /// Get all edges originating from a node
    pub fn get_outgoing_edges(&self, node_id: &str) -> Vec<&FlowchartEdge> {
        self.edges
            .iter()
            .filter(|e| e.from_node == node_id)
            .collect()
    }

    /// Get all edges pointing to a node
    pub fn get_incoming_edges(&self, node_id: &str) -> Vec<&FlowchartEdge> {
        self.edges
            .iter()
            .filter(|e| e.to_node == node_id)
            .collect()
    }

    /// Check if this is a valid DAG (no cycles)
    pub fn is_valid_dag(&self) -> bool {
        let mut visited = std::collections::HashSet::new();
        let mut rec_stack = std::collections::HashSet::new();

        for node in &self.nodes {
            if !visited.contains(&node.id) {
                if self.has_cycle(&node.id, &mut visited, &mut rec_stack) {
                    return false;
                }
            }
        }

        true
    }

    fn has_cycle(
        &self,
        node_id: &str,
        visited: &mut std::collections::HashSet<String>,
        rec_stack: &mut std::collections::HashSet<String>,
    ) -> bool {
        visited.insert(node_id.to_string());
        rec_stack.insert(node_id.to_string());

        for edge in self.get_outgoing_edges(node_id) {
            if !visited.contains(&edge.to_node) {
                if self.has_cycle(&edge.to_node, visited, rec_stack) {
                    return true;
                }
            } else if rec_stack.contains(&edge.to_node) {
                return true;
            }
        }

        rec_stack.remove(node_id);
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_flowchart_creation() {
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
                label: "Create todo-list".to_string(),
                message_refs: vec![MessageRef {
                    message_id: Uuid::new_v4().to_string(),
                    sequence_number: 1,
                    portion: None,
                }],
                node_type: NodeType::Action,
                description: None,
            },
        ];

        let edges = vec![FlowchartEdge {
            from_node: "1".to_string(),
            to_node: "2".to_string(),
            edge_type: EdgeType::Sequential,
            label: None,
        }];

        let flowchart = Flowchart::new("session-123".to_string(), nodes, edges);

        assert_eq!(flowchart.session_id, "session-123");
        assert_eq!(flowchart.nodes.len(), 2);
        assert_eq!(flowchart.edges.len(), 1);
        assert!(flowchart.is_valid_dag());
    }

    #[test]
    fn test_dag_validation() {
        let nodes = vec![
            FlowchartNode {
                id: "1".to_string(),
                label: "A".to_string(),
                message_refs: vec![],
                node_type: NodeType::Context,
                description: None,
            },
            FlowchartNode {
                id: "2".to_string(),
                label: "B".to_string(),
                message_refs: vec![],
                node_type: NodeType::Context,
                description: None,
            },
        ];

        let edges = vec![FlowchartEdge {
            from_node: "1".to_string(),
            to_node: "2".to_string(),
            edge_type: EdgeType::Sequential,
            label: None,
        }];

        let flowchart = Flowchart::new("session-123".to_string(), nodes, edges);
        assert!(flowchart.is_valid_dag());

        let edges_with_cycle = vec![
            FlowchartEdge {
                from_node: "1".to_string(),
                to_node: "2".to_string(),
                edge_type: EdgeType::Sequential,
                label: None,
            },
            FlowchartEdge {
                from_node: "2".to_string(),
                to_node: "1".to_string(),
                edge_type: EdgeType::Sequential,
                label: None,
            },
        ];

        let nodes_clone = vec![
            FlowchartNode {
                id: "1".to_string(),
                label: "A".to_string(),
                message_refs: vec![],
                node_type: NodeType::Context,
                description: None,
            },
            FlowchartNode {
                id: "2".to_string(),
                label: "B".to_string(),
                message_refs: vec![],
                node_type: NodeType::Context,
                description: None,
            },
        ];

        let flowchart_with_cycle =
            Flowchart::new("session-123".to_string(), nodes_clone, edges_with_cycle);
        assert!(!flowchart_with_cycle.is_valid_dag());
    }
}
