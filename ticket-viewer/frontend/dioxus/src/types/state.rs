#[allow(unused_imports)]
use super::*;

// ── State colours ─────────────────────────────────────────────────────────────

/// Returns `(background, foreground)` CSS colour pair for a ticket state.
///
/// Used by badges, cards, and search results throughout the UI.
pub fn state_colors(state: &str) -> (&'static str, &'static str) {
    match state {
        "open" => ("#2d2d4a", "#a0a0c8"),
        "planned" => ("#1a3d28", "#86efac"),
        "in-implementation" => ("#3d2e1a", "#fbbf24"),
        "in-review" => ("#361a4a", "#c084fc"),
        "done" => ("#1a3d28", "#4ade80"),
        "cancelled" => ("#3d1a1a", "#f87171"),
        _ => ("#2a2a3a", "#9ca3af"),
    }
}

/// Returns a single accent colour for an optional state string.
///
/// Convenience wrapper used by graph node borders and other simple indicators.
pub fn state_accent(state: Option<&str>) -> &'static str {
    match state {
        Some("open") => "#a0a0c8",
        Some("planned") => "#86efac",
        Some("in-implementation") => "#fbbf24",
        Some("in-review") => "#c084fc",
        Some("done") => "#4ade80",
        Some("cancelled") => "#f87171",
        _ => "#9ca3af",
    }
}
