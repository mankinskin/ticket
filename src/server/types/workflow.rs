#[allow(unused_imports)]
use super::*;

#[derive(Debug, Deserialize, JsonSchema)]
pub struct WorkflowInput {
    #[serde(default = "default_workflow_name")]
    pub name: WorkflowName,
    #[serde(default)]
    pub workspace: Option<String>,
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default)]
    pub query: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct NextTicketsInput {
    pub workspace: String,
    #[serde(default)]
    pub limit: Option<usize>,
    #[serde(default)]
    pub filter: Option<String>,
    /// Optional ticket UUID or 8+ character hex prefix.
    /// When set, scope results to actionable leaf blockers beneath this ticket.
    #[serde(default)]
    pub root: Option<String>,
}

pub(super) fn default_workflow_name() -> WorkflowName {
    WorkflowName::List
}
