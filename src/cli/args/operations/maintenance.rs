#[allow(unused_imports)]
use super::*;

#[derive(Debug, Args)]
pub struct HealthArgs {
    /// Root ticket UUID or 8+ character hex prefix. Checks the subgraph rooted here.
    #[arg(required_unless_present_any = ["all", "stdin", "ids"])]
    pub root: Option<String>,
    /// Check all tickets instead of a subgraph.
    #[arg(long, default_value_t = false)]
    pub all: bool,
    /// Read newline-delimited ticket UUIDs from stdin instead of traversing a subgraph.
    #[arg(long, default_value_t = false)]
    pub stdin: bool,
    /// Explicit ticket IDs (UUID or 8+ prefix). Can be repeated.
    #[arg(long = "id")]
    pub ids: Vec<String>,
    /// Maximum traversal depth when walking the subgraph (default: 0 = single ticket; max: 8).
    #[arg(long, default_value = "0")]
    pub depth: usize,
    /// Edge direction to follow for subgraph: out, in, or both.
    #[arg(long, default_value = "out")]
    pub direction: String,
    /// Filter by field values (key=value). Can be repeated.
    #[arg(long = "where")]
    pub where_clauses: Vec<String>,
}

#[derive(Debug, Args)]
pub struct StoreIndexArgs {
    /// Check-only mode: render the ticket catalog and exit non-zero on drift.
    #[arg(long, default_value_t = false)]
    pub check: bool,
}

#[derive(Debug, Args)]
pub struct FmtArgs {
    /// Report files needing reordering without writing any changes.
    ///
    /// When set, the command exits with `status = "needs_formatting"` and a
    /// positive `reformatted` count if any ticket.toml is out of canonical
    /// field order. Useful for CI gating.
    #[arg(long, default_value_t = false)]
    pub check: bool,
}

#[derive(Debug, Args)]
#[command(group(
    ArgGroup::new("part_layout_operation")
        .args(["apply", "verify", "undo"])
        .multiple(false)
))]
pub struct MigratePartLayoutArgs {
    /// Apply a previously saved dry-run plan.
    #[arg(long)]
    pub apply: bool,
    /// Verify the actual store state from a saved apply report.
    #[arg(long)]
    pub verify: bool,
    /// Undo only migration-owned changes from a saved apply report.
    #[arg(long)]
    pub undo: bool,
    /// Saved dry-run plan to apply.
    #[arg(long)]
    pub plan_file: Option<PathBuf>,
    /// Saved apply report to verify or undo.
    #[arg(long)]
    pub apply_report: Option<PathBuf>,
    /// Path for the machine-readable operation report.
    #[arg(long, required = true)]
    pub report_file: PathBuf,
}
