#[allow(unused_imports)]
use super::*;

#[derive(Debug, Args)]
pub struct StatusArgs {
    /// Optional prefix filter — only include tickets whose title starts with this string.
    /// E.g. "[bootstrap]" to scope the view to the bootstrap track.
    #[arg(long)]
    pub filter: Option<String>,
    /// Include blocked tickets in the output (default: omitted for brevity).
    #[arg(long, default_value_t = false)]
    pub show_blocked: bool,
}

#[derive(Debug, Args)]
pub struct ReadyOverviewArgs {
    /// Optional prefix filter — only include tickets whose title starts with this string.
    #[arg(long)]
    pub filter: Option<String>,
    /// Optional scope label included in the JSON response.
    #[arg(long)]
    pub scope: Option<String>,
}

#[derive(Debug, Args)]
pub struct NextArgs {
    /// Optional ticket UUID or 8+ character hex prefix.
    /// When set, scope results to actionable leaf blockers beneath this ticket.
    pub root: Option<String>,
    /// Maximum number of tickets to return.
    #[arg(long)]
    pub limit: Option<usize>,
    /// Optional prefix filter — only include tickets whose title starts with this string.
    #[arg(long)]
    pub filter: Option<String>,
    /// Skip board-awareness: include tickets already tracked on the board in results.
    #[arg(long, default_value_t = false)]
    pub no_board: bool,
}

#[derive(Debug, Args)]
pub struct UnblockedByArgs {
    /// Ticket UUID or 8+ character hex prefix to treat as satisfied.
    pub id: String,
}

#[derive(Debug, Args)]
pub struct BlockersArgs {
    /// Ticket UUID or 8+ character hex prefix to inspect for unresolved prerequisites.
    pub id: String,
}

#[derive(Debug, Args)]
pub struct WatchArgs {
    /// Debounce time in milliseconds before triggering reconcile after an event.
    #[arg(long, default_value = "200")]
    pub debounce_ms: u64,
}

#[derive(Debug, Args)]
pub struct ServeCliArgs {
    /// TCP port to bind to.
    #[arg(long, default_value = "8080")]
    pub port: u16,
    /// Host address to bind to.
    #[arg(long, default_value = "127.0.0.1")]
    pub host: String,
    /// Serve a specific named workspace only (default: all registered).
    #[arg(long)]
    pub workspace: Option<String>,
}
