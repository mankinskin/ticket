//! Renders a `view=full` ticket projection: typed parts as independently
//! collapsible sections in manifest order, frozen badges, inline amendments,
//! and typed refs. See spec 24b3d22b, ticket 89fa0c25.

use std::collections::HashSet;

use dioxus::prelude::*;
use percent_encoding::{
    utf8_percent_encode,
    NON_ALPHANUMERIC,
};
use viewer_api_dioxus::FileContentViewer;

use crate::types::{
    ProjectedPart,
    ProjectedRef,
};

fn kind_label(kind: &str) -> String {
    kind.replace('_', " ")
        .split(' ')
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                Some(first) => {
                    first.to_uppercase().collect::<String>() + chars.as_str()
                },
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// Renders the parts of a `view=full` projection plus a trailing typed-refs
/// list. `collapsed` holds the ids of parts currently collapsed; anything
/// not in the set is expanded (AC4: default-open, independent toggling).
pub(super) fn render_parts_panel(
    parts: Vec<ProjectedPart>,
    refs: Option<Vec<ProjectedRef>>,
    collapsed: Signal<HashSet<String>>,
) -> Element {
    if parts.is_empty() {
        return rsx! {
            div {
                "data-testid": "ticket-parts-empty",
                style: "padding: 20px 24px; color: #6b7280; font-size: 13px;",
                em { "No content found." }
            }
        };
    }

    rsx! {
        div {
            "data-testid": "ticket-parts",
            style: "display: flex; flex-direction: column; gap: 14px; padding: 16px 24px 24px;",
            for part in parts {
                {render_part(part, collapsed, 0)}
            }
            if let Some(refs) = refs {
                {render_refs_panel(refs)}
            }
        }
    }
}

fn render_part(
    part: ProjectedPart,
    mut collapsed: Signal<HashSet<String>>,
    depth: usize,
) -> Element {
    let part_id = part.id.clone();
    let is_collapsed = collapsed.read().contains(&part_id);
    let label = kind_label(&part.kind);
    let untyped = part.is_untyped();
    let frozen = part.frozen;
    let filename = format!("{}.md", part.kind);
    let content = part.content.clone();
    let amendments = part.amendments.clone();
    let toggle_id = part_id.clone();
    let indent = if depth > 0 { 20 * depth } else { 0 };
    let is_description = part.kind == "objective";

    rsx! {
        div {
            key: "{part.id}",
            "data-testid": "ticket-part",
            "data-part-kind": "{part.kind}",
            "data-frozen": if frozen { "true" } else { "false" },
            style: "margin-left: {indent}px; border: 1px solid var(--border-subtle); border-radius: 12px; background: color-mix(in srgb, var(--bg-secondary) 74%, transparent); overflow: hidden;",
            div {
                "data-testid": "ticket-part-header",
                style: "display: flex; align-items: center; justify-content: space-between; gap: 10px; padding: 10px 14px; cursor: pointer;",
                onclick: move |_| {
                    let mut set = collapsed.write();
                    if set.contains(&toggle_id) {
                        set.remove(&toggle_id);
                    } else {
                        set.insert(toggle_id.clone());
                    }
                },
                div {
                    style: "display: flex; align-items: center; gap: 8px; min-width: 0;",
                    span {
                        "data-testid": "ticket-part-toggle",
                        style: "color: var(--text-muted); font-size: 11px; width: 12px; display: inline-block;",
                        if is_collapsed { "▶" } else { "▼" }
                    }
                    span {
                        style: "font-size: 12px; font-weight: 600; color: var(--text-primary); text-transform: uppercase; letter-spacing: 0.04em;",
                        "{label}"
                    }
                    if untyped {
                        span {
                            "data-testid": "ticket-part-untyped-badge",
                            style: "padding: 2px 8px; border-radius: 999px; border: 1px solid var(--border-subtle); font-size: 10px; color: var(--text-muted);",
                            "Untyped attachment"
                        }
                    }
                }
                if frozen {
                    span {
                        "data-testid": "ticket-part-frozen-badge",
                        style: "padding: 3px 9px; border-radius: 999px; background: color-mix(in srgb, #f59e0b 20%, transparent); border: 1px solid #f59e0b; color: #fbbf24; font-size: 10px; white-space: nowrap;",
                        "🔒 Frozen at `planned`"
                    }
                }
            }
            if !is_collapsed {
                div {
                    "data-testid": "ticket-part-body",
                    style: "padding: 4px 14px 14px;",
                    if is_description {
                        div {
                            "data-testid": "desc-markdown",
                            FileContentViewer {
                                content: content,
                                filename: filename,
                            }
                        }
                    } else {
                        FileContentViewer {
                            content: content,
                            filename: filename,
                        }
                    }
                }
            }
            if !amendments.is_empty() {
                div {
                    style: "display: flex; flex-direction: column; gap: 10px; padding: 0 14px 14px;",
                    for amendment in amendments {
                        {render_amendment(amendment, part.kind.clone(), collapsed)}
                    }
                }
            }
        }
    }
}

fn render_amendment(
    amendment: ProjectedPart,
    supersedes_kind: String,
    collapsed: Signal<HashSet<String>>,
) -> Element {
    rsx! {
        div {
            "data-testid": "ticket-part-amendment",
            style: "border-left: 3px solid #a5b4fc; padding-left: 10px;",
            div {
                style: "font-size: 10px; color: var(--text-muted); text-transform: uppercase; letter-spacing: 0.06em; margin-bottom: 6px;",
                "Amendment — supersedes {supersedes_kind}"
            }
            {render_part(amendment, collapsed, 1)}
        }
    }
}

fn render_refs_panel(refs: Vec<ProjectedRef>) -> Element {
    if refs.is_empty() {
        return rsx! { Fragment {} };
    }

    rsx! {
        div {
            "data-testid": "ticket-refs",
            style: "margin-top: 8px; border-top: 1px solid var(--border-subtle); padding-top: 14px;",
            div {
                style: "font-size: 11px; color: var(--text-muted); text-transform: uppercase; letter-spacing: 0.08em; margin-bottom: 10px;",
                "Typed references"
            }
            div {
                style: "display: flex; flex-direction: column; gap: 6px;",
                for reference in refs {
                    {render_ref_row(reference)}
                }
            }
        }
    }
}

fn render_ref_row(reference: ProjectedRef) -> Element {
    let (href, resolved) = resolve_ref_href(&reference.kind, &reference.urn);
    let note = reference.note.clone();

    rsx! {
        div {
            "data-testid": "ticket-ref-row",
            "data-ref-kind": "{reference.kind}",
            "data-ref-resolved": if resolved { "true" } else { "false" },
            style: "display: flex; align-items: center; gap: 8px; font-size: 12px; flex-wrap: wrap;",
            span {
                style: "padding: 2px 8px; border-radius: 999px; border: 1px solid var(--border-subtle); color: var(--text-muted); font-size: 10px; text-transform: uppercase;",
                "{reference.kind}"
            }
            if let Some(href) = href {
                a {
                    "data-testid": "ticket-ref-link",
                    href: "{href}",
                    target: "_blank",
                    rel: "noopener noreferrer",
                    style: "color: var(--accent-blue); text-decoration: underline;",
                    "{reference.urn}"
                }
            } else {
                span {
                    "data-testid": "ticket-ref-plain",
                    style: "color: var(--text-secondary);",
                    "{reference.urn}"
                }
                if !resolved {
                    span {
                        "data-testid": "ticket-ref-dangling-badge",
                        style: "padding: 2px 8px; border-radius: 999px; border: 1px solid #f87171; color: #f87171; font-size: 10px;",
                        "unresolved"
                    }
                }
            }
            if let Some(note) = note {
                span {
                    style: "color: var(--text-muted); font-style: italic;",
                    "— {note}"
                }
            }
        }
    }
}

/// Resolves a typed ref to a known viewer deep link. Returns
/// `(Some(href), true)` when resolvable, `(None, true)` for kinds that are
/// intentionally plain text (e.g. `file`), and `(None, false)` when the kind
/// claims a resolvable shape but the URN does not parse — the dangling case.
fn resolve_ref_href(
    kind: &str,
    urn: &str,
) -> (Option<String>, bool) {
    match kind {
        "spec" => match last_segment(urn) {
            Some(id) if looks_like_uuid(id) => {
                (Some(format!("http://localhost:4002/specs/{id}")), true)
            },
            _ => (None, false),
        },
        "log" => match last_segment(urn) {
            Some(name) if !name.is_empty() => {
                let encoded =
                    utf8_percent_encode(name, NON_ALPHANUMERIC).to_string();
                (
                    Some(format!("http://localhost:3000/#/file/{encoded}")),
                    true,
                )
            },
            _ => (None, false),
        },
        // `file` refs are repo-relative paths, not a viewer route; render as
        // plain text (not dangling — this is the intended shape).
        "file" => (None, true),
        // `test_execution`, `rule`, `commit`, and unknown/future kinds have
        // no known viewer route today; render as plain text, never a broken
        // link.
        _ => (None, true),
    }
}

fn last_segment(urn: &str) -> Option<&str> {
    urn.rsplit('/').next().filter(|s| !s.is_empty())
}

fn looks_like_uuid(s: &str) -> bool {
    s.len() == 36
        && s.as_bytes().iter().enumerate().all(|(i, b)| match i {
            8 | 13 | 18 | 23 => *b == b'-',
            _ => b.is_ascii_hexdigit(),
        })
}
