#[allow(unused_imports)]
use super::*;

#[derive(Debug, Args)]
pub struct CloseArgs {
    /// Ticket UUID or 8+ character hex prefix.
    pub id: String,
    /// Target state to fast-forward to (default: done).
    #[arg(long = "to-state", default_value = "done")]
    pub to_state: String,
    /// Author/user identity to record in history revisions (overrides TICKET_AUTHOR env var).
    #[arg(long)]
    pub author: Option<String>,
}

#[derive(Debug, Args)]
pub struct CancelArgs {
    /// Ticket UUID or 8+ character hex prefix.
    pub id: String,
    /// Author/user identity to record in history revisions (overrides TICKET_AUTHOR env var).
    #[arg(long)]
    pub author: Option<String>,
}

#[derive(Debug, Args)]
pub struct AttachArgs {
    /// Ticket UUID or 8+ character hex prefix.
    pub id: String,
    /// Path to the file to attach.
    pub path: PathBuf,
    /// Optional name for the asset (defaults to source filename).
    #[arg(long = "as")]
    pub asset_name: Option<String>,
}

#[derive(Debug, Args)]
pub struct MoveArgs {
    /// Ticket UUID or 8+ character hex prefix (required for plan/execute mode).
    pub id: Option<String>,
    /// Destination workspace root (normalized to its canonical `.ticket` store).
    #[arg(long = "to-workspace-root")]
    pub to_workspace_root: Option<PathBuf>,
    /// Resume an interrupted move by journal UUID.
    #[arg(long)]
    pub resume: Option<String>,
    /// Roll back a move by journal UUID.
    #[arg(long)]
    pub rollback: Option<String>,
    /// Preview planning output without mutating storage.
    #[arg(long, default_value_t = false)]
    pub dry_run: bool,
}

#[derive(Debug, Args)]
pub struct UpdateArgsCli {
    /// Ticket UUID or 8+ character hex prefix.
    pub id: String,
    #[arg(long = "transition-state")]
    pub transition_states: Vec<String>,
    #[arg(long = "to-state")]
    pub to_state: Option<String>,
    /// Opt out of auto-walking multi-hop transitions. When set, a `--to-state`
    /// that would skip a required waypoint is rejected with recovery guidance
    /// instead of traversing the intermediate states.
    #[arg(long = "single-hop", visible_alias = "strict", default_value_t = false)]
    pub single_hop: bool,
    #[arg(long = "field")]
    pub fields: Vec<String>,
    /// Revert to the previous history revision (undo the last change).
    #[arg(long)]
    pub undo: bool,
    /// Markdown description to write/overwrite as description.md.
    #[arg(long)]
    pub description: Option<String>,
    /// How to apply `--description`: `replace` (overwrites) or `append`
    /// (preserves existing content, concatenating onto it). Required when
    /// setting a description; there is no default.
    #[arg(long = "description-mode", value_enum)]
    pub description_mode: Option<DescriptionMode>,
    /// Author/user identity to record in the history revision (overrides TICKET_AUTHOR env var).
    #[arg(long)]
    pub author: Option<String>,
    /// After a successful update, also check the agent in to the board.
    #[arg(long, default_value_t = false)]
    pub board_check_in: bool,
    /// Agent identity to use for --board-check-in.
    #[arg(long)]
    pub board_agent: Option<String>,
    /// Work intent description to use for --board-check-in.
    #[arg(long)]
    pub board_intent: Option<String>,
    /// Files to claim ownership of during --board-check-in.
    #[arg(long = "board-file")]
    pub board_files: Vec<String>,
    /// Heartbeat TTL in seconds for --board-check-in (default: 3600).
    #[arg(long)]
    pub board_ttl_secs: Option<u64>,
}

/// The real `update` command input, converted from [`UpdateArgsCli`] once at
/// the CLI boundary. There is no separate `description_mode` field here: a
/// `description` supplied without a mode cannot be represented past
/// [`UpdateArgsCli::try_into`] (AC5 of ticket 3d952036) — the raw two-flag
/// clap struct is a boundary decoder only, never used past parsing.
#[derive(Debug)]
pub struct UpdateArgs {
    pub id: String,
    pub transition_states: Vec<String>,
    pub to_state: Option<String>,
    pub single_hop: bool,
    pub fields: Vec<String>,
    pub undo: bool,
    pub description_update: ticket_api::storage::DescriptionUpdate,
    pub author: Option<String>,
    pub board_check_in: bool,
    pub board_agent: Option<String>,
    pub board_intent: Option<String>,
    pub board_files: Vec<String>,
    pub board_ttl_secs: Option<u64>,
}

impl TryFrom<UpdateArgsCli> for UpdateArgs {
    type Error = String;

    fn try_from(cli: UpdateArgsCli) -> Result<Self, Self::Error> {
        let description_mode_str = cli.description_mode.map(|mode| match mode {
            DescriptionMode::Replace => "replace",
            DescriptionMode::Append => "append",
        });
        let description_update =
            ticket_api::storage::DescriptionUpdate::decode(cli.description, description_mode_str)?;
        Ok(UpdateArgs {
            id: cli.id,
            transition_states: cli.transition_states,
            to_state: cli.to_state,
            single_hop: cli.single_hop,
            fields: cli.fields,
            undo: cli.undo,
            description_update,
            author: cli.author,
            board_check_in: cli.board_check_in,
            board_agent: cli.board_agent,
            board_intent: cli.board_intent,
            board_files: cli.board_files,
            board_ttl_secs: cli.board_ttl_secs,
        })
    }
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum ReproOutcome {
    Reproduced,
    NotReproduced,
    Intermittent,
    Fixed,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum DescriptionMode {
    Replace,
    Append,
}

impl ReproOutcome {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Reproduced => "reproduced",
            Self::NotReproduced => "not_reproduced",
            Self::Intermittent => "intermittent",
            Self::Fixed => "fixed",
        }
    }
}

#[derive(Debug, Args)]
pub struct ReproArgs {
    /// Ticket UUID or 8+ character hex prefix.
    pub id: String,
    /// Reproduction outcome.
    #[arg(long, value_enum, default_value_t = ReproOutcome::Reproduced)]
    pub outcome: ReproOutcome,
    /// Commit SHA where reproduction was attempted (defaults to git HEAD if available).
    #[arg(long)]
    pub commit: Option<String>,
    /// Optional reproduction command used.
    #[arg(long)]
    pub command: Option<String>,
    /// Optional short note.
    #[arg(long)]
    pub note: Option<String>,
    /// Optional RFC3339 timestamp (defaults to now/UTC).
    #[arg(long)]
    pub timestamp: Option<String>,
}
