#[allow(unused_imports)]
use super::*;

#[derive(Debug, Deserialize, JsonSchema)]
pub struct MovePreflightInput {
    /// Explicit workspace root or .ticket store path required; omitted/empty/default/.. rejected; . means server cwd.
    pub workspace: String,
    pub id: String,
    pub to_workspace_root: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct MoveApplyInput {
    /// Explicit workspace root or .ticket store path required; omitted/empty/default/.. rejected; . means server cwd.
    pub workspace: String,
    pub id: String,
    pub to_workspace_root: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct MoveJournalInput {
    /// Explicit workspace root or .ticket store path required; omitted/empty/default/.. rejected; . means server cwd.
    pub workspace: String,
    pub id: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ListPartsInput {
    /// Explicit workspace root or .ticket store path required; omitted/empty/default/.. rejected; . means server cwd.
    pub workspace: String,
    pub id: String,
    #[serde(default)]
    pub with_content: bool,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetPartInput {
    /// Explicit workspace root or .ticket store path required; omitted/empty/default/.. rejected; . means server cwd.
    pub workspace: String,
    pub id: String,
    pub part_id: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct WritePartInput {
    /// Explicit workspace root or .ticket store path required; omitted/empty/default/.. rejected; . means server cwd.
    pub workspace: String,
    pub id: String,
    /// Opaque part id (UUID) to update. Omit to create a new part.
    #[serde(default)]
    pub part_id: Option<String>,
    /// Part kind (e.g. objective, requirements, review, or any free-form
    /// attachment kind). Used when creating a new part; ignored when
    /// updating an existing part.
    pub kind: String,
    pub content: String,
    #[serde(default)]
    pub author: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct WriteAmendmentInput {
    /// Explicit workspace root or .ticket store path required; omitted/empty/default/.. rejected; . means server cwd.
    pub workspace: String,
    pub id: String,
    /// Opaque part id (UUID) of the part this amendment corrects.
    pub supersedes: String,
    /// Opaque part id (UUID) for the new amendment part. Omit to generate one.
    #[serde(default)]
    pub part_id: Option<String>,
    pub content: String,
    #[serde(default)]
    pub author: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct UndoPartInput {
    /// Explicit workspace root or .ticket store path required; omitted/empty/default/.. rejected; . means server cwd.
    pub workspace: String,
    pub id: String,
    pub part_id: String,
    #[serde(default)]
    pub author: Option<String>,
}
