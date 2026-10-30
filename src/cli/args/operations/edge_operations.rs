#[allow(unused_imports)]
use super::*;

#[derive(Debug, Args)]
pub struct LinkArgs {
    /// UUID or 8+ character hex prefix of the source ticket.
    #[arg(long)]
    pub from: String,
    /// UUID or 8+ character hex prefix of the target ticket.
    #[arg(long)]
    pub to: String,
    /// Edge kind (e.g. depends_on, linked).
    #[arg(long)]
    pub kind: String,
    /// Human-readable reason for this edge (optional, stored in response only).
    #[arg(long)]
    pub reason: Option<String>,
}

#[derive(Debug, Args)]
pub struct UnlinkArgs {
    /// UUID or 8+ character hex prefix of the source ticket.
    #[arg(long)]
    pub from: String,
    /// UUID or 8+ character hex prefix of the target ticket.
    #[arg(long)]
    pub to: String,
    /// Edge kind (e.g. depends_on, linked).
    #[arg(long)]
    pub kind: String,
    /// Human-readable reason for this removal (optional, stored in response only).
    #[arg(long)]
    pub reason: Option<String>,
}

#[derive(Debug, Args)]
pub struct SubgraphArgs {
    /// Root ticket UUID or 8+ character hex prefix.
    pub root: String,
    /// Maximum traversal depth (default: 4, max: 8).
    #[arg(long, default_value = "4")]
    pub depth: usize,
    /// Edge direction to follow: out, in, or both.
    #[arg(long, default_value = "out")]
    pub direction: String,
    /// Filter edges by kind (default: all).
    #[arg(long = "edge-kind", default_value = "all")]
    pub edge_kind: String,
}

#[derive(Debug, Args)]
pub struct TopgraphArgs {
    /// Root ticket UUID or 8+ character hex prefix.
    pub root: String,
    /// Maximum traversal depth (default: 4, max: 8).
    #[arg(long, default_value = "4")]
    pub depth: usize,
    /// Edge direction to follow: out, in, or both.
    #[arg(long, default_value = "in")]
    pub direction: String,
    /// Filter edges by kind (default: all).
    #[arg(long = "edge-kind", default_value = "all")]
    pub edge_kind: String,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum DanglingStrategy {
    /// Remove each dangling edge directly.
    Unlink,
    /// Reconcile only: report candidates without mutation.
    ReconcileOnly,
}

impl DanglingStrategy {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Unlink => "unlink",
            Self::ReconcileOnly => "reconcile_only",
        }
    }

    pub fn mutates(self) -> bool {
        matches!(self, Self::Unlink)
    }
}

#[derive(Debug, Args)]
pub struct PruneDanglingArgs {
    /// Source ticket UUID or 8+ character hex prefix.
    /// Omit with --all to inspect all source tickets.
    #[arg(required_unless_present = "all")]
    pub root: Option<String>,
    /// Check all source tickets instead of a single root ticket.
    #[arg(long, default_value_t = false)]
    pub all: bool,
    /// Edge kind to inspect (default: depends_on).
    #[arg(long, default_value = "depends_on")]
    pub kind: String,
    /// Cleanup strategy.
    #[arg(long, value_enum, default_value_t = DanglingStrategy::Unlink)]
    pub strategy: DanglingStrategy,
    /// Optional reason recorded in response output.
    #[arg(long)]
    pub reason: Option<String>,
}
