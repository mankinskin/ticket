// ticket-vscode-core
//
// Rust/WASM core for the ticket-vscode VS Code extension.
// Compiled to wasm32-unknown-unknown via wasm-pack.
//
// Design contract (frozen in spec ticket-vscode/rust-wasm-port):
// - No `vscode` or Node/browser APIs are imported here.
// - All host interaction goes through capability-object arguments passed in
//   from the JS/TS host shell at activation time.
// - #[wasm_bindgen] annotations are gated behind the "wasm" feature flag so
//   the crate builds and tests natively without wasm-pack.

#[cfg(feature = "wasm")]
use wasm_bindgen::prelude::*;

// ── Version ───────────────────────────────────────────────────────────────────

/// Returns the core library version string.
/// Used by both hosts to confirm the WASM module loaded successfully.
#[cfg_attr(feature = "wasm", wasm_bindgen)]
pub fn core_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

// ── Domain types ──────────────────────────────────────────────────────────────
//
// These mirror the shapes in `src/api.ts` and are the input feed from the JS
// host shell after fetching from the ticket-viewer HTTP API.

/// Minimal ticket summary — mirrors `TicketSummary` in `src/api.ts`.
#[cfg_attr(feature = "wasm", wasm_bindgen)]
#[derive(Debug, Clone)]
pub struct TicketSummary {
    id: String,
    ticket_type: String,
    title: String,
    state: String,
}

#[cfg_attr(feature = "wasm", wasm_bindgen)]
impl TicketSummary {
    /// Construct a TicketSummary from the JS host.
    /// `title` and `state` are empty strings when the API returns null.
    #[cfg_attr(feature = "wasm", wasm_bindgen(constructor))]
    pub fn new(id: String, ticket_type: String, title: String, state: String) -> Self {
        Self {
            id,
            ticket_type,
            title,
            state,
        }
    }

    #[cfg_attr(feature = "wasm", wasm_bindgen(getter))]
    pub fn id(&self) -> String {
        self.id.clone()
    }

    #[cfg_attr(feature = "wasm", wasm_bindgen(getter))]
    pub fn ticket_type(&self) -> String {
        self.ticket_type.clone()
    }

    #[cfg_attr(feature = "wasm", wasm_bindgen(getter))]
    pub fn title(&self) -> String {
        self.title.clone()
    }

    #[cfg_attr(feature = "wasm", wasm_bindgen(getter))]
    pub fn state(&self) -> String {
        self.state.clone()
    }
}

/// A directed edge between two tickets — mirrors `EdgeRecord` in `src/api.ts`.
#[cfg_attr(feature = "wasm", wasm_bindgen)]
#[derive(Debug, Clone)]
pub struct EdgeRecord {
    from: String,
    to: String,
    kind: String,
}

#[cfg_attr(feature = "wasm", wasm_bindgen)]
impl EdgeRecord {
    #[cfg_attr(feature = "wasm", wasm_bindgen(constructor))]
    pub fn new(from: String, to: String, kind: String) -> Self {
        Self { from, to, kind }
    }

    #[cfg_attr(feature = "wasm", wasm_bindgen(getter))]
    pub fn from_id(&self) -> String {
        self.from.clone()
    }

    #[cfg_attr(feature = "wasm", wasm_bindgen(getter))]
    pub fn to_id(&self) -> String {
        self.to.clone()
    }

    #[cfg_attr(feature = "wasm", wasm_bindgen(getter))]
    pub fn kind(&self) -> String {
        self.kind.clone()
    }
}

// ── Host-kind detection ───────────────────────────────────────────────────────

/// Host kind reported by `HostDetectionCapability` in the JS shell.
#[cfg_attr(feature = "wasm", wasm_bindgen)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HostKind {
    DesktopNode,
    RemoteWorkspace,
    BrowserWeb,
    Virtual,
}

/// Returns `true` when server-control features (startServer, binary spawn)
/// should be available for the given host kind.
#[cfg_attr(feature = "wasm", wasm_bindgen)]
pub fn supports_server_control(host: HostKind) -> bool {
    matches!(host, HostKind::DesktopNode | HostKind::RemoteWorkspace)
}

/// Returns `true` when browser-bridge features (bridge* commands) should be
/// available for the given host kind.
#[cfg_attr(feature = "wasm", wasm_bindgen)]
pub fn supports_browser_bridge(host: HostKind) -> bool {
    matches!(host, HostKind::DesktopNode)
}

/// Returns `true` when on-disk file browsing is available.
/// Desktop and remote workspace hosts have a real filesystem accessible via
/// `vscode.workspace.fs`; browser/virtual hosts treat this as best-effort.
#[cfg_attr(feature = "wasm", wasm_bindgen)]
pub fn supports_file_browsing(host: HostKind) -> bool {
    matches!(host, HostKind::DesktopNode | HostKind::RemoteWorkspace)
}

// ── Filtering ─────────────────────────────────────────────────────────────────

/// Returns `true` when a ticket matches both state and query filters.
/// Pure function — no I/O, no VS Code APIs.
#[cfg_attr(feature = "wasm", wasm_bindgen)]
pub fn ticket_matches(ticket: &TicketSummary, state_filter: &str, query: &str) -> bool {
    if !state_filter.is_empty() && ticket.state != state_filter {
        return false;
    }
    if !query.is_empty() {
        let q = query.to_lowercase();
        if !ticket.title.to_lowercase().contains(&q) && !ticket.id.to_lowercase().contains(&q) {
            return false;
        }
    }
    true
}

fn filter_indices(tickets: &[TicketSummary], state_filter: &str, query: &str) -> Vec<usize> {
    tickets
        .iter()
        .enumerate()
        .filter(|(_, t)| ticket_matches(t, state_filter, query))
        .map(|(i, _)| i)
        .collect()
}

// ── Dependency maps ───────────────────────────────────────────────────────────

/// Pre-computed bidirectional lookup tables built from a flat edge list.
///
/// Equivalent to `_depsOf` / `_parentOf` / `_hasParent` in `TicketTreeProvider`.
pub struct DependencyMaps {
    /// ticket id → ids of tickets it depends_on (children in the tree)
    pub deps_of: std::collections::HashMap<String, Vec<String>>,
    /// ticket id → ids of its parents (reverse of deps_of)
    pub parent_of: std::collections::HashMap<String, Vec<String>>,
}

impl DependencyMaps {
    pub fn build(tickets: &[TicketSummary], edges: &[EdgeRecord]) -> Self {
        let known: std::collections::HashSet<&str> =
            tickets.iter().map(|t| t.id.as_str()).collect();

        let mut deps_of: std::collections::HashMap<String, Vec<String>> =
            std::collections::HashMap::new();
        let mut parent_of: std::collections::HashMap<String, Vec<String>> =
            std::collections::HashMap::new();

        for edge in edges {
            if edge.kind != "depends_on" {
                continue;
            }
            if !known.contains(edge.from.as_str()) || !known.contains(edge.to.as_str()) {
                continue;
            }
            deps_of
                .entry(edge.from.clone())
                .or_default()
                .push(edge.to.clone());
            parent_of
                .entry(edge.to.clone())
                .or_default()
                .push(edge.from.clone());
        }

        Self { deps_of, parent_of }
    }
}

// ── State grouping + root detection ──────────────────────────────────────────

/// A group of tickets that share the same state, as displayed in the sidebar.
///
/// Mirrors `StateGroupItem` / `buildStateGroups` from `ticketProvider.ts`.
#[derive(Debug, Clone)]
pub struct StateGroup {
    /// The state value (e.g. "planned", "in-implementation").
    pub state: String,
    /// Total tickets in this state bucket.
    pub total: usize,
    /// Ids of root tickets — those with no same-state parent.
    pub root_ids: Vec<String>,
}

/// Build state groups from tickets, edges, schema order, and active filters.
///
/// `state_order` is the schema-defined state list (empty = alphabetical).
/// `state_filter` and `query` are the current active filters.
pub fn build_state_groups(
    tickets: &[TicketSummary],
    edges: &[EdgeRecord],
    state_order: &[String],
    state_filter: &str,
    query: &str,
) -> Vec<StateGroup> {
    let maps = DependencyMaps::build(tickets, edges);
    let visible_indices = filter_indices(tickets, state_filter, query);
    let visible: Vec<&TicketSummary> = visible_indices.iter().map(|&i| &tickets[i]).collect();

    let mut grouped: std::collections::HashMap<&str, Vec<&TicketSummary>> =
        std::collections::HashMap::new();
    for t in &visible {
        grouped.entry(t.state.as_str()).or_default().push(t);
    }

    let make_group = |state: &str, bucket: &[&TicketSummary]| -> StateGroup {
        let state_ids: std::collections::HashSet<&str> =
            bucket.iter().map(|t| t.id.as_str()).collect();
        let root_ids: Vec<String> = bucket
            .iter()
            .filter(|t| {
                !maps.parent_of.get(t.id.as_str()).map_or(false, |ps| {
                    ps.iter().any(|p| state_ids.contains(p.as_str()))
                })
            })
            .map(|t| t.id.clone())
            .collect();
        StateGroup {
            state: state.to_string(),
            total: bucket.len(),
            root_ids,
        }
    };

    let mut result: Vec<StateGroup> = Vec::new();
    let mut remaining = grouped.clone();

    for s in state_order {
        if let Some(bucket) = remaining.remove(s.as_str()) {
            if !bucket.is_empty() {
                result.push(make_group(s.as_str(), &bucket));
            }
        }
    }
    let mut extra: Vec<(&str, Vec<&TicketSummary>)> = remaining.into_iter().collect();
    extra.sort_by_key(|(s, _)| *s);
    for (s, bucket) in extra {
        if !bucket.is_empty() {
            result.push(make_group(s, &bucket));
        }
    }

    result
}

// ── URL / command intent derivation ──────────────────────────────────────────

/// Returns the URL for opening a ticket in the ticket-viewer SPA.
#[cfg_attr(feature = "wasm", wasm_bindgen)]
pub fn ticket_viewer_url(base_url: &str, workspace: &str, ticket_id: &str) -> String {
    format!("{base_url}/?workspace={workspace}&ticket={ticket_id}")
}

/// Returns the short display label for a ticket (first 8 chars of id if no title).
#[cfg_attr(feature = "wasm", wasm_bindgen)]
pub fn ticket_display_label(id: &str, title: &str) -> String {
    if title.is_empty() {
        format!("({})", &id[..id.len().min(8)])
    } else {
        title.to_string()
    }
}

// ── WASM-specific wrappers ─────────────────────────────────────────────────────
//
// These types and functions expose the domain logic to JavaScript.
// Bulk inputs (ticket lists, edge lists) are passed as parallel `js_sys::Array`
// instances of strings rather than arrays of Rust structs, because wasm-bindgen
// does not support extracting custom struct types from JsValue arrays.
// Individual TicketSummary objects CAN be passed directly to ticket_matches
// (wasm-bindgen handles &T parameters via RefFromWasmAbi).

#[cfg(feature = "wasm")]
mod wasm_api;

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests;
