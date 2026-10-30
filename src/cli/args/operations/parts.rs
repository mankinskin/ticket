#[allow(unused_imports)]
use super::*;

#[derive(Debug, Args)]
pub struct ListPartsArgs {
    /// Ticket UUID or 8+ character hex prefix.
    pub id: String,
    /// Include each part's full markdown content in the output.
    #[arg(long, default_value_t = false)]
    pub with_content: bool,
}

#[derive(Debug, Args)]
pub struct GetPartArgs {
    /// Ticket UUID or 8+ character hex prefix.
    pub id: String,
    /// Opaque part id (UUID) to fetch.
    #[arg(long = "part-id")]
    pub part_id: Uuid,
}

#[derive(Debug, Args)]
pub struct WritePartArgs {
    /// Ticket UUID or 8+ character hex prefix.
    pub id: String,
    /// Opaque part id (UUID) to update. Omit to create a new part.
    #[arg(long = "part-id")]
    pub part_id: Option<Uuid>,
    /// Part kind (e.g. objective, requirements, review, or any free-form
    /// attachment kind). Used when creating a new part; ignored when
    /// updating an existing part (kind is assigned once at creation).
    #[arg(long)]
    pub kind: String,
    /// New markdown content for the part.
    #[arg(long, conflicts_with = "content_file")]
    pub content: Option<String>,
    /// Read new markdown content for the part from this file.
    #[arg(long = "content-file", conflicts_with = "content")]
    pub content_file: Option<PathBuf>,
    /// Author/user identity to record in the history revision (overrides TICKET_AUTHOR env var).
    #[arg(long)]
    pub author: Option<String>,
}

#[derive(Debug, Args)]
pub struct WriteAmendmentArgs {
    /// Ticket UUID or 8+ character hex prefix.
    pub id: String,
    /// Opaque part id (UUID) of the frozen (or any) part this amendment corrects.
    #[arg(long)]
    pub supersedes: Uuid,
    /// Opaque part id (UUID) for the new amendment part. Omit to generate one.
    #[arg(long = "part-id")]
    pub part_id: Option<Uuid>,
    /// Markdown content of the amendment.
    #[arg(long, conflicts_with = "content_file")]
    pub content: Option<String>,
    /// Read markdown content of the amendment from this file.
    #[arg(long = "content-file", conflicts_with = "content")]
    pub content_file: Option<PathBuf>,
    /// Author/user identity to record in the history revision (overrides TICKET_AUTHOR env var).
    #[arg(long)]
    pub author: Option<String>,
}

#[derive(Debug, Args)]
pub struct UndoPartArgs {
    /// Ticket UUID or 8+ character hex prefix.
    pub id: String,
    /// Opaque part id (UUID) to restore to its content prior to its most recent write.
    #[arg(long = "part-id")]
    pub part_id: Uuid,
    /// Author/user identity to record in the history revision (overrides TICKET_AUTHOR env var).
    #[arg(long)]
    pub author: Option<String>,
}
