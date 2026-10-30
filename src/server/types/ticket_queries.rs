#[allow(unused_imports)]
use super::*;

#[derive(Serialize)]
pub struct TicketSummary {
    pub id: String,
    #[serde(rename = "type")]
    pub type_id: String,
    pub title: Option<String>,
    pub state: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub effort: Option<u64>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Serialize)]
pub struct TicketDetail {
    pub id: String,
    pub path: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub fields: BTreeMap<String, Value>,
}

#[derive(Clone, Serialize)]
pub struct EdgeItem {
    pub from: String,
    pub to: String,
    pub kind: String,
}

#[derive(Serialize)]
pub struct NodeItem {
    pub id: String,
    pub title: Option<String>,
    pub state: Option<String>,
    pub depth: usize,
}

#[derive(Serialize)]
pub struct SubgraphResponse {
    pub workspace: String,
    pub nodes: Vec<NodeItem>,
    pub edges: Vec<EdgeItem>,
    pub truncated: bool,
    pub stats: SubgraphStats,
}

#[derive(Serialize)]
pub struct SubgraphStats {
    pub nodes_returned: usize,
    pub edges_returned: usize,
    pub max_depth_reached: usize,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ListTicketsInput {
    pub workspace: String,
    #[serde(default)]
    pub state: Option<String>,
    #[serde(default, rename = "type")]
    pub type_id: Option<String>,
    #[serde(default)]
    pub query: Option<String>,
    #[serde(default)]
    pub limit: Option<usize>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct TicketRefInput {
    #[serde(default, deserialize_with = "blank_as_none")]
    pub workspace: Option<String>,
    pub id: String,
    /// Named read-projection view profile: summary, plan, review, or full.
    /// Mutually exclusive with `parts`. Defaults to `summary` when neither
    /// is supplied. Ignored by tools other than `get_ticket`.
    #[serde(default, deserialize_with = "blank_as_none")]
    pub view: Option<String>,
    /// Explicit comma-separated part-kind list to project (e.g.
    /// "objective,acceptance_criteria"). Mutually exclusive with `view`.
    /// Ignored by tools other than `get_ticket`.
    #[serde(default, deserialize_with = "blank_as_none")]
    pub parts: Option<String>,
}

/// Treat an omitted, null, or blank string as "not supplied". LLM callers
/// routinely fill every optional string field with `""` as a placeholder;
/// without this, `view: ""` plus `parts: ""` trips the mutual-exclusion
/// check and `workspace: ""` bypasses the `default` workspace fallback.
pub(super) fn blank_as_none<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = Option::<String>::deserialize(deserializer)?;
    Ok(value.filter(|s| !s.trim().is_empty()))
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ListEdgesInput {
    pub workspace: String,
    #[serde(default)]
    pub kind: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SubgraphInput {
    pub workspace: String,
    pub root: String,
    #[serde(default)]
    pub direction: Option<String>,
    #[serde(default)]
    pub edge_kind: Option<String>,
    #[serde(default)]
    pub depth: Option<usize>,
    #[serde(default)]
    pub limit_nodes: Option<usize>,
    #[serde(default)]
    pub limit_edges: Option<usize>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct TopgraphInput {
    pub workspace: String,
    pub root: String,
    #[serde(default)]
    pub direction: Option<String>,
    #[serde(default)]
    pub edge_kind: Option<String>,
    #[serde(default)]
    pub depth: Option<usize>,
    #[serde(default)]
    pub limit_nodes: Option<usize>,
    #[serde(default)]
    pub limit_edges: Option<usize>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct HealthCheckInput {
    pub workspace: String,
    #[serde(default)]
    pub root: Option<String>,
    #[serde(default)]
    pub all: bool,
    #[serde(default)]
    pub ids: Vec<String>,
    #[serde(default)]
    pub depth: Option<usize>,
    #[serde(default)]
    pub direction: Option<String>,
    #[serde(default)]
    pub r#where: Vec<String>,
}
