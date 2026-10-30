#[allow(unused_imports)]
use super::*;

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
        #[serde(default, skip_serializing_if = "std::collections::BTreeMap::is_empty")]
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
