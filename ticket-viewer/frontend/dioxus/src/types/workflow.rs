#[allow(unused_imports)]
use super::*;

// ── Workflow ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct WorkflowRootSummary {
    pub id: String,
    #[serde(default)]
    pub ticket_ref: TicketRef,
    pub title: Option<String>,
    pub state: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct WorkflowCandidateItem {
    pub rank: usize,
    pub id: String,
    #[serde(default)]
    pub ticket_ref: TicketRef,
    pub title: Option<String>,
    pub state: Option<String>,
    #[serde(rename = "type", default)]
    pub ticket_type: String,
    #[serde(default)]
    pub priority: String,
    pub dependency_count: usize,
    pub remaining_blocker_count: usize,
    #[serde(default, alias = "dependees")]
    pub dependee_count: usize,
    pub transitive_reverse_dependents: usize,
    pub affected_reverse_dependent_reach: usize,
    pub max_affected_dependent_state: Option<String>,
    pub dependency_state_gap: usize,
    pub became_actionable_at: Option<String>,
    pub last_blocker_progress_at: Option<String>,
    pub created_at: String,
}

impl WorkflowCandidateItem {
    #[allow(dead_code)]
    pub fn resolved_ticket_ref(&self, active_workspace: &str) -> TicketRef {
        if !self.ticket_ref.workspace.is_empty() && !self.ticket_ref.id.is_empty() {
            return self.ticket_ref.clone();
        }

        TicketRef::new(active_workspace, self.id.clone())
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct WorkflowTreeItem {
    pub id: String,
    #[serde(default)]
    pub ticket_ref: TicketRef,
    pub title: Option<String>,
    pub state: Option<String>,
    #[serde(rename = "type", default)]
    pub ticket_type: String,
    #[serde(default)]
    pub priority: String,
    pub remaining_blocker_count: usize,
    pub unresolved_frontier_leaf_count: usize,
    #[serde(default)]
    pub frontier_leaf_ids: Vec<String>,
    pub blocker_distance: usize,
    pub is_frontier: bool,
    pub dependency_count: usize,
    pub immediate_dependees: usize,
    pub transitive_reverse_dependents: usize,
    pub affected_reverse_dependent_reach: usize,
    pub dependency_state_gap: usize,
    pub became_actionable_at: Option<String>,
    pub last_blocker_progress_at: Option<String>,
    #[serde(default)]
    pub children: Vec<WorkflowTreeItem>,
}

impl WorkflowTreeItem {
    #[allow(dead_code)]
    pub fn resolved_ticket_ref(&self, active_workspace: &str) -> TicketRef {
        if !self.ticket_ref.workspace.is_empty() && !self.ticket_ref.id.is_empty() {
            return self.ticket_ref.clone();
        }

        TicketRef::new(active_workspace, self.id.clone())
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct WorkflowNextResponse {
    pub request_id: String,
    #[serde(default)]
    pub active_workspace: String,
    pub workspace: String,
    #[serde(default)]
    pub root: Option<WorkflowRootSummary>,
    #[serde(default)]
    pub reachable_dependents: Option<usize>,
    #[serde(default)]
    pub blocked_dependents: Option<usize>,
    #[serde(default)]
    pub remaining_blocker_count: Option<usize>,
    pub count: usize,
    #[serde(default)]
    pub items: Vec<WorkflowCandidateItem>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct WorkflowTreeResponse {
    pub request_id: String,
    #[serde(default)]
    pub active_workspace: String,
    pub workspace: String,
    pub kind: String,
    pub root: WorkflowTreeItem,
    pub frontier_count: usize,
    #[serde(default)]
    pub frontier_items: Vec<WorkflowCandidateItem>,
    #[serde(default)]
    pub reachable_dependents: Option<usize>,
    #[serde(default)]
    pub blocked_dependents: Option<usize>,
}
