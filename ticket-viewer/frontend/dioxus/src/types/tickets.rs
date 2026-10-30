#[allow(unused_imports)]
use super::*;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct TicketRef {
    #[serde(default)]
    pub workspace: String,
    #[serde(default)]
    pub id: String,
}

impl TicketRef {
    pub fn new(workspace: impl Into<String>, id: impl Into<String>) -> Self {
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
    pub fn label_for_name(&self, workspace_name: &str) -> Option<String> {
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
    pub fn resolved_ticket_ref(&self, active_workspace: &str) -> TicketRef {
        if !self.ticket_ref.workspace.is_empty() && !self.ticket_ref.id.is_empty() {
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
    pub fn resolved_ticket_ref(&self, active_workspace: &str) -> TicketRef {
        if !self.ticket_ref.workspace.is_empty() && !self.ticket_ref.id.is_empty() {
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
