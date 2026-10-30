#[allow(unused_imports)]
use super::*;

#[derive(Deserialize)]
pub struct WorkflowNextQuery {
    pub workspace: String,
    pub root: Option<Uuid>,
    pub filter: Option<String>,
    #[serde(default)]
    pub limit: Option<usize>,
}

#[derive(Deserialize)]
pub struct WorkflowTreeQuery {
    pub workspace: String,
    pub root: Uuid,
}

#[derive(Clone)]
pub(super) struct NextScope {
    pub(super) root: WorkflowRootSummary,
    pub(super) reachable_dependencies: usize,
    pub(super) blocked_dependencies: usize,
    pub(super) remaining_blockers: HashSet<Uuid>,
    pub(super) blocker_tree: WorkflowTreeItem,
}

#[derive(Clone, Serialize)]
pub struct WorkflowRootSummary {
    pub id: String,
    pub ticket_ref: TicketRef,
    pub title: Option<String>,
    pub state: Option<String>,
}

#[derive(Serialize)]
pub struct WorkflowCandidateItem {
    pub rank: usize,
    pub id: String,
    pub ticket_ref: TicketRef,
    pub title: Option<String>,
    pub state: Option<String>,
    #[serde(rename = "type")]
    pub ticket_type: String,
    pub priority: String,
    pub effort: Option<u64>,
    pub dependency_count: usize,
    pub remaining_blocker_count: usize,
    pub dependee_count: usize,
    pub transitive_reverse_dependents: usize,
    pub affected_reverse_dependent_reach: usize,
    pub max_affected_dependent_state: Option<String>,
    pub dependency_state_gap: usize,
    pub became_actionable_at: Option<String>,
    pub last_blocker_progress_at: Option<String>,
    pub created_at: String,
}

#[derive(Clone, Serialize)]
pub struct WorkflowTreeItem {
    pub id: String,
    pub ticket_ref: TicketRef,
    pub title: Option<String>,
    pub state: Option<String>,
    #[serde(rename = "type")]
    pub ticket_type: String,
    pub priority: String,
    pub remaining_blocker_count: usize,
    pub unresolved_frontier_leaf_count: usize,
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
    pub children: Vec<WorkflowTreeItem>,
}

#[derive(Serialize)]
pub struct ScopeMetadata {
    pub workspace: String,
    pub active_index_root: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filter: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub root: Option<String>,
}

#[derive(Serialize)]
pub struct WorkflowNextResponse {
    pub request_id: String,
    pub active_workspace: String,
    pub workspace: String,
    pub scope: ScopeMetadata,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub root: Option<WorkflowRootSummary>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reachable_dependencies: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub blocked_dependencies: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remaining_blocker_count: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub blocker_tree: Option<WorkflowTreeItem>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub frontier_count: Option<usize>,
    pub count: usize,
    pub items: Vec<WorkflowCandidateItem>,
    pub excluded_by_board: Vec<BoardExcludedCandidate>,
    pub warnings: Vec<String>,
}

#[derive(Serialize)]
pub struct WorkflowTreeResponse {
    pub request_id: String,
    pub active_workspace: String,
    pub workspace: String,
    pub kind: String,
    pub root: WorkflowTreeItem,
    pub frontier_count: usize,
    pub frontier_items: Vec<WorkflowCandidateItem>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reachable_dependents: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub blocked_dependents: Option<usize>,
}

#[derive(Clone, Copy)]
pub(super) enum TreeKind {
    Blockers,
    UnblockedBy,
}

pub(super) struct TreePayload<'a> {
    pub(super) tree: WorkflowTreeNode,
    pub(super) frontier_ids: Vec<Uuid>,
    pub(super) reachable_dependents: Option<usize>,
    pub(super) blocked_dependents: Option<usize>,
    pub(super) kind_label: &'a str,
    pub(super) satisfied_ids: Option<HashSet<Uuid>>,
}
