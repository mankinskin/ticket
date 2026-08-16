//! API request and response types for the ticket viewer.
//!
//! Kept separate from the HTTP client so components can import types without
//! pulling in the transport layer.

use serde::{
    Deserialize,
    Serialize,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct TicketRef {
    #[serde(default)]
    pub workspace: String,
    #[serde(default)]
    pub id: String,
}

impl TicketRef {
    pub fn new(
        workspace: impl Into<String>,
        id: impl Into<String>,
    ) -> Self {
        Self {
            workspace: workspace.into(),
            id: id.into(),
        }
    }

    pub fn key(&self) -> String {
        format!("{}:{}", self.workspace, self.id)
    }
}

// ── Workspace ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Deserialize)]
pub struct WorkspaceInfo {
    pub name: String,
    #[serde(default)]
    pub label: String,
}

impl WorkspaceInfo {
    pub fn display_label(&self) -> String {
        if self.label.trim().is_empty() {
            return self.name.clone();
        }

        self.label.clone()
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct WorkspacesResponse {
    #[serde(default)]
    pub active_workspace: String,
    pub workspaces: Vec<WorkspaceInfo>,
}

impl WorkspacesResponse {
    pub fn label_for_name(
        &self,
        workspace_name: &str,
    ) -> Option<String> {
        self.workspaces
            .iter()
            .find(|workspace| workspace.name == workspace_name)
            .map(WorkspaceInfo::display_label)
    }
}

// ── Ticket ────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct TicketSummary {
    pub id: String,
    #[serde(default)]
    pub ticket_ref: TicketRef,
    pub title: Option<String>,
    pub state: Option<String>,
    #[serde(rename = "type", default)]
    pub ticket_type: Option<String>,
    pub created_at: String,
    #[serde(default)]
    pub updated_at: String,
    #[serde(default)]
    pub fields: serde_json::Value,
}

impl TicketSummary {
    pub fn resolved_ticket_ref(
        &self,
        active_workspace: &str,
    ) -> TicketRef {
        if !self.ticket_ref.workspace.is_empty()
            && !self.ticket_ref.id.is_empty()
        {
            return self.ticket_ref.clone();
        }

        TicketRef::new(active_workspace, self.id.clone())
    }
}

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct TicketsResponse {
    #[serde(default)]
    pub active_workspace: String,
    pub workspace: String,
    pub items: Vec<TicketSummary>,
    pub next_cursor: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TicketDetail {
    pub id: String,
    #[serde(default)]
    pub ticket_ref: TicketRef,
    pub created_at: String,
    pub fields: serde_json::Value,
}

impl TicketDetail {
    pub fn resolved_ticket_ref(
        &self,
        active_workspace: &str,
    ) -> TicketRef {
        if !self.ticket_ref.workspace.is_empty()
            && !self.ticket_ref.id.is_empty()
        {
            return self.ticket_ref.clone();
        }

        TicketRef::new(active_workspace, self.id.clone())
    }
}

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct TicketDetailResponse {
    #[serde(default)]
    pub active_workspace: String,
    pub workspace: String,
    pub ticket: TicketDetail,
}

// ── Parts / refs (structured ticket read, `view=full` projection) ────────────

/// A typed reference from a ticket to a non-ticket entity (spec, log, file,
/// etc.). `kind` is a plain string so a foreign/future kind round-trips.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct ProjectedRef {
    pub kind: String,
    pub urn: String,
    #[serde(default)]
    pub note: Option<String>,
}

/// A single projected ticket part. `amendments` carries any parts that
/// supersede this one, inlined directly beneath it by the backend
/// projection (oldest first).
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct ProjectedPart {
    pub id: String,
    pub kind: String,
    #[serde(default)]
    pub frozen: bool,
    pub created_at: String,
    #[serde(default)]
    pub supersedes: Option<String>,
    #[serde(default)]
    pub content: String,
    #[serde(default)]
    pub implicit: bool,
    #[serde(default)]
    pub amendments: Vec<ProjectedPart>,
}

/// The schema-validated core part kinds understood by projections (spec
/// 24b3d22b). Anything else is a free-form/untyped attachment.
pub const CORE_PART_KINDS: &[&str] = &[
    "objective",
    "requirements",
    "design",
    "examples",
    "acceptance_criteria",
    "review",
    "validation",
    "notes",
    "amendment",
];

impl ProjectedPart {
    pub fn is_untyped(&self) -> bool {
        !CORE_PART_KINDS.contains(&self.kind.as_str())
    }
}

/// The `ticket` payload of a `view=full` projected read
/// (`GET /api/tickets/{id}?view=full`).
#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct TicketProjection {
    pub id: String,
    pub created_at: String,
    #[serde(default)]
    pub fields: serde_json::Value,
    #[serde(default)]
    pub profile: Option<String>,
    #[serde(default)]
    pub parts: Vec<ProjectedPart>,
    #[serde(default)]
    pub refs: Option<Vec<ProjectedRef>>,
}

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct TicketFullResponse {
    #[serde(default)]
    pub active_workspace: String,
    pub workspace: String,
    pub ticket: TicketProjection,
}

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct TicketDescriptionResponse {
    pub id: String,
    #[serde(default)]
    pub active_workspace: String,
    pub workspace: String,
    #[serde(default)]
    pub ticket_ref: TicketRef,
    pub description: Option<String>,
}

// ── Ticket file tree ──────────────────────────────────────────────────────────

/// A single user-visible file belonging to a ticket.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct TicketFileEntry {
    /// Relative path within the ticket folder, e.g. `"description.md"` or
    /// `"assets/design/plan.md"`.
    pub path: String,
    /// Display name (just the filename), e.g. `"plan.md"`.
    pub name: String,
}

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct TicketFilesResponse {
    pub id: String,
    #[serde(default)]
    pub active_workspace: String,
    pub workspace: String,
    #[serde(default)]
    pub ticket_ref: TicketRef,
    pub files: Vec<TicketFileEntry>,
}

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct TicketAssetResponse {
    pub id: String,
    #[serde(default)]
    pub active_workspace: String,
    pub workspace: String,
    #[serde(default)]
    pub ticket_ref: TicketRef,
    pub path: String,
    pub content: String,
}

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
    pub fn resolved_ticket_ref(
        &self,
        active_workspace: &str,
    ) -> TicketRef {
        if !self.ticket_ref.workspace.is_empty()
            && !self.ticket_ref.id.is_empty()
        {
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
    pub fn resolved_from_ref(
        &self,
        active_workspace: &str,
    ) -> TicketRef {
        if !self.from_ref.workspace.is_empty() && !self.from_ref.id.is_empty() {
            return self.from_ref.clone();
        }

        TicketRef::new(active_workspace, self.from.clone())
    }

    pub fn resolved_to_ref(
        &self,
        active_workspace: &str,
    ) -> TicketRef {
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
    pub fn resolved_ticket_ref(
        &self,
        active_workspace: &str,
    ) -> TicketRef {
        if !self.ticket_ref.workspace.is_empty()
            && !self.ticket_ref.id.is_empty()
        {
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
    pub fn resolved_ticket_ref(
        &self,
        active_workspace: &str,
    ) -> TicketRef {
        if !self.ticket_ref.workspace.is_empty()
            && !self.ticket_ref.id.is_empty()
        {
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

// ── Schema ────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct FieldDef {
    pub field_type: String,
    pub required: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TransitionDef {
    pub from: String,
    pub to: String,
}

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct EdgeRuleDef {
    pub directed: bool,
    pub acyclic_enforced: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct TypeSchema {
    pub type_id: String,
    pub states: Vec<String>,
    pub transitions: Vec<TransitionDef>,
    pub fields: std::collections::BTreeMap<String, FieldDef>,
    pub edge_rules: std::collections::BTreeMap<String, EdgeRuleDef>,
    pub required_states: Vec<String>,
    pub terminal_states: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct SchemaListResponse {
    #[serde(default)]
    pub active_workspace: Option<String>,
    pub workspace: String,
    pub types: Vec<TypeSchema>,
}

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct SchemaDetailResponse {
    #[serde(default)]
    pub active_workspace: Option<String>,
    pub workspace: String,
    pub schema: TypeSchema,
}

// ── Mutation requests ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Default)]
pub struct TicketPatch {
    pub workspace: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub state: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fields: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CreateTicketRequest {
    #[serde(rename = "type")]
    pub type_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fields: Option<std::collections::BTreeMap<String, serde_json::Value>>,
}

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct CreateTicketResponse {
    #[serde(default)]
    pub active_workspace: String,
    pub workspace: String,
    pub ticket: TicketDetail,
}

#[derive(Debug, Clone, Serialize)]
pub struct EdgeMutationBody {
    pub from_id: String,
    pub to_id: String,
    pub kind: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

// ── History ───────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct HistoryEntry {
    pub rev: u64,
    pub ts: String,
    pub author: Option<String>,
    pub fields: serde_json::Value,
}

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct TicketHistoryResponse {
    pub id: String,
    #[serde(default)]
    pub active_workspace: String,
    pub workspace: String,
    #[serde(default)]
    pub ticket_ref: TicketRef,
    pub count: u64,
    pub entries: Vec<HistoryEntry>,
}

// ── Batch operations ──────────────────────────────────────────────────────────

/// A single command within a batch request sent to `POST /api/batch`.
///
/// The `op` field is the discriminator; matches the server-side `BatchCommand`
/// serde representation.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum BatchCommand {
    Update {
        id: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        state: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        from_state: Option<String>,
        #[serde(
            default,
            skip_serializing_if = "std::collections::BTreeMap::is_empty"
        )]
        fields: std::collections::BTreeMap<String, serde_json::Value>,
    },
    Close {
        id: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        target_state: Option<String>,
    },
    Cancel {
        id: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        reason: Option<String>,
    },
}

/// Request body for `POST /api/batch`.
#[derive(Debug, Clone, Serialize)]
pub struct BatchRequest {
    pub workspace: String,
    pub commands: Vec<BatchCommand>,
}

/// Per-command result returned in the batch response.
#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct BatchCommandResult {
    /// The raw JSON result value for this command (ticket object, etc.).
    pub result: Option<serde_json::Value>,
    /// Error message if this command failed (non-transactional reporting).
    pub error: Option<String>,
}

/// Response body from `POST /api/batch`.
#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct BatchResponse {
    pub workspace: String,
    pub status: String,
    pub count: usize,
    pub results: Vec<serde_json::Value>,
}

// ── State colours ─────────────────────────────────────────────────────────────

/// Returns `(background, foreground)` CSS colour pair for a ticket state.
///
/// Used by badges, cards, and search results throughout the UI.
pub fn state_colors(state: &str) -> (&'static str, &'static str) {
    match state {
        "open" => ("#2d2d4a", "#a0a0c8"),
        "planned" => ("#1a3d28", "#86efac"),
        "in-implementation" => ("#3d2e1a", "#fbbf24"),
        "in-review" => ("#361a4a", "#c084fc"),
        "done" => ("#1a3d28", "#4ade80"),
        "cancelled" => ("#3d1a1a", "#f87171"),
        _ => ("#2a2a3a", "#9ca3af"),
    }
}

/// Returns a single accent colour for an optional state string.
///
/// Convenience wrapper used by graph node borders and other simple indicators.
pub fn state_accent(state: Option<&str>) -> &'static str {
    match state {
        Some("open") => "#a0a0c8",
        Some("planned") => "#86efac",
        Some("in-implementation") => "#fbbf24",
        Some("in-review") => "#c084fc",
        Some("done") => "#4ade80",
        Some("cancelled") => "#f87171",
        _ => "#9ca3af",
    }
}
