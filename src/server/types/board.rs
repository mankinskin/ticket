#[allow(unused_imports)]
use super::*;

#[derive(Debug, Deserialize, JsonSchema)]
pub struct BoardShowInput {
    pub workspace: String,
    #[serde(default)]
    pub agent_id: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct BoardHistoryInput {
    pub workspace: String,
    #[serde(default)]
    pub agent_id: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct BoardCheckInInput {
    pub workspace: String,
    pub ticket_id: String,
    pub agent_id: String,
    #[serde(default)]
    pub intent: Option<String>,
    #[serde(default)]
    pub files: Vec<String>,
    #[serde(default)]
    pub ttl_secs: Option<u64>,
    #[serde(default)]
    pub session_id: Option<String>,
    #[serde(default)]
    pub worktree_path: Option<String>,
    #[serde(default)]
    pub branch: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct BoardWorktreesInput {
    pub workspace: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct BoardCheckOutInput {
    pub workspace: String,
    pub ticket_id: String,
    #[serde(default)]
    pub agent_id: Option<String>,
    #[serde(default)]
    pub reason: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct BoardReleaseLeaseInput {
    pub workspace: String,
    pub ticket_id: String,
    pub requester: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct BoardHeartbeatInput {
    pub workspace: String,
    pub entry_id: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct BoardConfigureInput {
    pub workspace: String,
    #[serde(default)]
    pub max_wip: Option<u32>,
    #[serde(default)]
    pub stale_after_secs: Option<u64>,
    #[serde(default)]
    pub completed_audit_window_secs: Option<u64>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct BoardCleanPreviewInput {
    pub workspace: String,
    #[serde(default)]
    pub include_stale: Option<bool>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct BoardCleanApplyInput {
    pub workspace: String,
    pub token: String,
    #[serde(default)]
    pub include_stale: Option<bool>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct BoardUpdateFilesInput {
    pub workspace: String,
    pub ticket_id: String,
    pub agent_id: String,
    #[serde(default)]
    pub add: Vec<String>,
    #[serde(default)]
    pub remove: Vec<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct BoardRenameFileInput {
    pub workspace: String,
    pub ticket_id: String,
    pub agent_id: String,
    pub old_path: String,
    pub new_path: String,
}
