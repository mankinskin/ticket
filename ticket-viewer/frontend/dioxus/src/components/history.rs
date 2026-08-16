//! HistoryPanel — vertical revision timeline with field diffs and a snapshot
//! drawer.
//!
//! Props:
//! - `workspace`  — workspace the ticket belongs to.
//! - `ticket_id`  — ticket identifier used for the revert API call.
//! - `entries`    — history entries pre-fetched by the parent (TicketContent).
//! - `on_refresh` — called after a successful revert so the parent can
//!                  re-fetch the history list and update any dependents.

use dioxus::prelude::*;

use crate::{
    api::{
        HttpTicketBackend,
        TicketBackend,
    },
    types::HistoryEntry,
};

// ── Field-diff helpers ────────────────────────────────────────────────────────

#[derive(Clone, PartialEq)]
enum DiffKind {
    Added,
    Changed,
    Removed,
}

#[derive(Clone, PartialEq)]
struct FieldDiff {
    key: String,
    kind: DiffKind,
    old_val: Option<String>,
    new_val: Option<String>,
}

fn json_display(v: &serde_json::Value) -> String {
    match v {
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Null => String::new(),
        other => other.to_string(),
    }
}

fn compute_diff(
    prev: &serde_json::Value,
    curr: &serde_json::Value,
) -> Vec<FieldDiff> {
    let empty = serde_json::Map::new();
    let prev_map = prev.as_object().unwrap_or(&empty);
    let curr_map = curr.as_object().unwrap_or(&empty);
    let mut diffs = Vec::new();

    for (key, new_val) in curr_map {
        match prev_map.get(key) {
            None => diffs.push(FieldDiff {
                key: key.clone(),
                kind: DiffKind::Added,
                old_val: None,
                new_val: Some(json_display(new_val)),
            }),
            Some(old_val) if old_val != new_val => diffs.push(FieldDiff {
                key: key.clone(),
                kind: DiffKind::Changed,
                old_val: Some(json_display(old_val)),
                new_val: Some(json_display(new_val)),
            }),
            _ => {},
        }
    }
    for (key, old_val) in prev_map {
        if !curr_map.contains_key(key) {
            diffs.push(FieldDiff {
                key: key.clone(),
                kind: DiffKind::Removed,
                old_val: Some(json_display(old_val)),
                new_val: None,
            });
        }
    }
    diffs.sort_by(|a, b| a.key.cmp(&b.key));
    diffs
}

fn format_ts(ts: &str) -> String {
    let s = ts.trim_end_matches('Z');
    let s = if let Some(dot) = s.find('.') {
        &s[..dot]
    } else {
        s
    };
    format!("{} UTC", s.replace('T', " "))
}

// ── Pre-computed per-entry data ───────────────────────────────────────────────

/// Pre-computed display data for one timeline entry.
/// Computed outside `rsx!` to avoid `let` statements inside `for` loop bodies.
#[derive(Clone)]
struct EntryView {
    rev: u64,
    ts_display: String,
    author_display: String,
    diffs: Vec<FieldDiff>,
    is_last: bool,
    /// Field rows `(key, display_value)` for the snapshot drawer.
    field_rows: Vec<(String, String)>,
}

// ── Component ─────────────────────────────────────────────────────────────────

#[component]
pub fn HistoryPanel(
    workspace: String,
    ticket_id: String,
    entries: Vec<HistoryEntry>,
    on_refresh: EventHandler<()>,
) -> Element {
    let mut selected_rev: Signal<Option<u64>> = use_signal(|| None);
    let mut revert_loading: Signal<bool> = use_signal(|| false);
    let mut revert_error: Signal<Option<String>> = use_signal(|| None);

    // Sort newest-first.
    let mut sorted = entries.clone();
    sorted.sort_by(|a, b| b.rev.cmp(&a.rev));

    // Pre-compute all per-entry display data BEFORE entering rsx! so we can
    // use ordinary Rust `let` without hitting the RSX parser limitation.
    let entry_count = sorted.len();
    let entry_views: Vec<EntryView> = sorted
        .iter()
        .enumerate()
        .map(|(i, entry)| {
            let prev_fields = sorted.get(i + 1).map(|e| &e.fields);
            let diffs = if let Some(prev) = prev_fields {
                compute_diff(prev, &entry.fields)
            } else {
                compute_diff(
                    &serde_json::Value::Object(serde_json::Map::new()),
                    &entry.fields,
                )
            };
            let field_rows = entry
                .fields
                .as_object()
                .map(|m| {
                    m.iter()
                        .map(|(k, v)| (k.clone(), json_display(v)))
                        .collect()
                })
                .unwrap_or_default();
            EntryView {
                rev: entry.rev,
                ts_display: format_ts(&entry.ts),
                author_display: entry.author.clone().unwrap_or_default(),
                diffs,
                is_last: i + 1 == entry_count,
                field_rows,
            }
        })
        .collect();

    // Find the drawer entry for the currently-selected revision.
    let drawer_view: Option<EntryView> = selected_rev()
        .and_then(|rev| entry_views.iter().find(|ev| ev.rev == rev).cloned());

    // Snapshot drawer metadata (avoids repeated clones inside rsx!).
    let drawer_rev = drawer_view.as_ref().map(|ev| ev.rev);
    let drawer_field_rows: Vec<(String, String)> = drawer_view
        .as_ref()
        .map(|ev| ev.field_rows.clone())
        .unwrap_or_default();

    rsx! {
        div {
            "data-testid": "history-panel",
            style: "position: relative; display: flex; flex-direction: row; height: 100%; overflow: hidden;",

            // ── Timeline ──────────────────────────────────────────────
            div {
                "data-testid": "history-timeline",
                style: "flex: 1; overflow-y: auto; padding: 16px 20px; min-width: 0;",

                for ev in entry_views {
                    {
                        let rev = ev.rev;
                        let ts_display = ev.ts_display.clone();
                        let author = ev.author_display.clone();
                        let is_last = ev.is_last;
                        let diffs = ev.diffs.clone();
                        let is_selected = selected_rev() == Some(rev);

                        rsx! {
                            div {
                                key: "{rev}",
                                "data-testid": "history-entry-{rev}",
                                style: "display: flex; flex-direction: row; gap: 12px;",

                                // Dot + connector
                                div {
                                    style: "display: flex; flex-direction: column; align-items: center; flex-shrink: 0; width: 20px;",
                                    div {
                                        style: "width: 12px; height: 12px; border-radius: 50%; background: #6366f1; border: 2px solid #1a1a2e; flex-shrink: 0; margin-top: 4px;",
                                    }
                                    if !is_last {
                                        div {
                                            style: "width: 2px; flex: 1; min-height: 24px; background: #3a3a4a; margin: 4px 0;",
                                        }
                                    }
                                }

                                // Entry body
                                div {
                                    style: "flex: 1; padding-bottom: 20px; min-width: 0;",

                                    // Header row
                                    div {
                                        style: "display: flex; align-items: center; flex-wrap: wrap; gap: 8px; margin-bottom: 6px;",

                                        span {
                                            style: "background: #2d2d4a; color: #a5b4fc; border-radius: 4px; padding: 1px 7px; font-size: 11px; font-family: monospace; flex-shrink: 0;",
                                            "r{rev}"
                                        }
                                        span {
                                            style: "color: #9ca3af; font-size: 12px; flex-shrink: 0;",
                                            "{ts_display}"
                                        }
                                        if !author.is_empty() {
                                            span {
                                                style: "color: #6b7280; font-size: 11px;",
                                                "by {author}"
                                            }
                                        }
                                        div { style: "flex: 1;" }
                                        button {
                                            "aria-label": "View snapshot for revision {rev}",
                                            style: if is_selected {
                                                "background: #6366f1; color: #fff; border: none; border-radius: 4px; padding: 2px 10px; font-size: 11px; cursor: pointer;"
                                            } else {
                                                "background: transparent; color: #6366f1; border: 1px solid #6366f1; border-radius: 4px; padding: 2px 10px; font-size: 11px; cursor: pointer;"
                                            },
                                            onclick: move |_| {
                                                if selected_rev() == Some(rev) {
                                                    selected_rev.set(None);
                                                } else {
                                                    selected_rev.set(Some(rev));
                                                    revert_error.set(None);
                                                }
                                            },
                                            if is_selected { "Close" } else { "View" }
                                        }
                                    }

                                    // Field diffs
                                    if diffs.is_empty() {
                                        em {
                                            style: "color: #6b7280; font-size: 12px;",
                                            "No field changes."
                                        }
                                    } else {
                                        div {
                                            style: "display: flex; flex-direction: column; gap: 2px;",
                                            for diff in diffs {
                                                {
                                                    let (bg, fg, pfx, val) = match &diff.kind {
                                                        DiffKind::Added => ("#1a3d28", "#86efac", "+", diff.new_val.clone().unwrap_or_default()),
                                                        DiffKind::Removed => ("#3d1a1a", "#f87171", "-", diff.old_val.clone().unwrap_or_default()),
                                                        DiffKind::Changed => ("#3d2e1a", "#fbbf24", "~", diff.new_val.clone().unwrap_or_default()),
                                                    };
                                                    let dk = diff.key.clone();
                                                    rsx! {
                                                        div {
                                                            style: "background: {bg}; color: {fg}; border-radius: 3px; padding: 2px 8px; font-family: monospace; font-size: 12px; white-space: pre-wrap; word-break: break-all;",
                                                            "{pfx} {dk}: {val}"
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // ── Snapshot drawer ───────────────────────────────────────
            if let Some(rev) = drawer_rev {
                {
                    let ws = workspace.clone();
                    let tid = ticket_id.clone();
                    rsx! {
                        div {
                            "data-testid": "history-drawer",
                            style: "width: 320px; flex-shrink: 0; border-left: 1px solid #3a3a4a; display: flex; flex-direction: column; overflow: hidden; background: #141420;",

                            // Header
                            div {
                                style: "display: flex; align-items: center; padding: 12px 16px; border-bottom: 1px solid #3a3a4a; flex-shrink: 0;",
                                span {
                                    style: "flex: 1; font-size: 13px; color: #e0e0e8; font-weight: 600;",
                                    "Revision r{rev} snapshot"
                                }
                                button {
                                    "aria-label": "Close snapshot drawer",
                                    style: "background: transparent; border: none; color: #6b7280; cursor: pointer; font-size: 18px; line-height: 1; padding: 0 4px;",
                                    onclick: move |_| {
                                        selected_rev.set(None);
                                        revert_error.set(None);
                                    },
                                    "×"
                                }
                            }

                            // Field snapshot
                            div {
                                style: "flex: 1; overflow-y: auto; padding: 12px 16px;",
                                if drawer_field_rows.is_empty() {
                                    em {
                                        style: "color: #6b7280; font-size: 12px;",
                                        "No fields in this snapshot."
                                    }
                                } else {
                                    for (fk, fv) in drawer_field_rows.clone() {
                                        div {
                                            style: "margin-bottom: 10px;",
                                            div {
                                                style: "color: #9ca3af; font-size: 11px; margin-bottom: 2px;",
                                                "{fk}"
                                            }
                                            div {
                                                style: "color: #e0e0e8; font-size: 12px; font-family: monospace; background: #1e1e2e; border-radius: 3px; padding: 4px 8px; word-break: break-all; white-space: pre-wrap;",
                                                "{fv}"
                                            }
                                        }
                                    }
                                }
                            }

                            // Revert
                            div {
                                style: "padding: 12px 16px; border-top: 1px solid #3a3a4a; flex-shrink: 0;",
                                if let Some(err) = revert_error() {
                                    div {
                                        "data-testid": "revert-error",
                                        style: "color: #f87171; font-size: 12px; margin-bottom: 8px;",
                                        "{err}"
                                    }
                                }
                                button {
                                    "data-testid": "revert-button-{rev}",
                                    "aria-label": "Revert ticket to revision {rev}",
                                    disabled: revert_loading(),
                                    style: "width: 100%; padding: 7px 0; background: #7c3aed; color: #fff; border: none; border-radius: 5px; font-size: 13px; cursor: pointer;",
                                    onclick: move |_| {
                                        let ws2 = ws.clone();
                                        let tid2 = tid.clone();
                                        revert_loading.set(true);
                                        revert_error.set(None);
                                        spawn(async move {
                                            let backend = HttpTicketBackend::new(None);
                                            match backend.revert_ticket(&ws2, &tid2, rev).await {
                                                Ok(_) => {
                                                    revert_loading.set(false);
                                                    selected_rev.set(None);
                                                    on_refresh.call(());
                                                }
                                                Err(e) => {
                                                    revert_loading.set(false);
                                                    revert_error.set(Some(e));
                                                }
                                            }
                                        });
                                    },
                                    if revert_loading() { "Reverting…" } else { "Revert to this revision" }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
