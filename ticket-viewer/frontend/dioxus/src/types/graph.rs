#[allow(unused_imports)]
use super::*;

// ── Graph / Subgraph ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, Deserialize)]
pub struct GraphNodeItem {
    pub id: String,
    #[serde(default)]
    pub ticket_ref: TicketRef,
    pub title: Option<String>,
    pub state: Option<String>,
    pub depth: usize,
    #[serde(rename = "type", default)]
    pub ticket_type: Option<String>,
    #[serde(default)]
    pub priority: Option<String>,
}

impl GraphNodeItem {
    pub fn resolved_ticket_ref(&self, active_workspace: &str) -> TicketRef {
        if !self.ticket_ref.workspace.is_empty() && !self.ticket_ref.id.is_empty() {
            return self.ticket_ref.clone();
        }

        TicketRef::new(active_workspace, self.id.clone())
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct GraphEdgeItem {
    pub from: String,
    pub to: String,
    #[serde(default)]
    pub from_ref: TicketRef,
    #[serde(default)]
    pub to_ref: TicketRef,
    pub kind: String,
}

impl GraphEdgeItem {
    pub fn resolved_from_ref(&self, active_workspace: &str) -> TicketRef {
        if !self.from_ref.workspace.is_empty() && !self.from_ref.id.is_empty() {
            return self.from_ref.clone();
        }

        TicketRef::new(active_workspace, self.from.clone())
    }

    pub fn resolved_to_ref(&self, active_workspace: &str) -> TicketRef {
        if !self.to_ref.workspace.is_empty() && !self.to_ref.id.is_empty() {
            return self.to_ref.clone();
        }

        TicketRef::new(active_workspace, self.to.clone())
    }
}

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct SubgraphStats {
    pub nodes_returned: usize,
    pub edges_returned: usize,
    pub max_depth_reached: usize,
}

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct GraphSubgraphResponse {
    #[serde(default)]
    pub active_workspace: String,
    pub workspace: String,
    pub nodes: Vec<GraphNodeItem>,
    pub edges: Vec<GraphEdgeItem>,
    pub truncated: bool,
    pub stats: SubgraphStats,
}
