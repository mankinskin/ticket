use dioxus::prelude::*;

use crate::{
    api::{
        HttpTicketBackend,
        TicketBackend,
    },
    types::{
        state_colors,
        TicketRef,
        WorkflowCandidateItem,
        WorkflowNextResponse,
        WorkflowTreeItem,
        WorkflowTreeResponse,
    },
};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum WorkflowSidebarMode {
    Next,
    Blockers,
    UnblockedBy,
}

impl WorkflowSidebarMode {
    pub fn label(self) -> &'static str {
        match self {
            Self::Next => "Actionable next",
            Self::Blockers => "Blockers",
            Self::UnblockedBy => "Unblocked by",
        }
    }

    fn description(self) -> &'static str {
        match self {
            Self::Next => "Server-ranked actionable work for this workspace.",
            Self::Blockers =>
                "Blocking prerequisites for the pinned root ticket.",
            Self::UnblockedBy =>
                "Work that opens up once the pinned root ticket is satisfied.",
        }
    }
}

#[derive(Props, Clone, PartialEq)]
pub struct WorkflowExplorerProps {
    pub workspace: String,
    pub mode: WorkflowSidebarMode,
    #[props(default)]
    pub selected_ticket: Option<TicketRef>,
    #[props(default)]
    pub root_ticket: Option<TicketRef>,
    pub on_select: EventHandler<TicketRef>,
    pub on_use_selected_root: EventHandler<()>,
}

enum WorkflowFetchResult {
    Next(WorkflowNextResponse),
    Tree(WorkflowTreeResponse),
}

#[component]
pub fn WorkflowExplorer(props: WorkflowExplorerProps) -> Element {
    let mut loading: Signal<bool> = use_signal(|| false);
    let mut error: Signal<Option<String>> = use_signal(|| None);
    let mut next_response: Signal<Option<WorkflowNextResponse>> =
        use_signal(|| None);
    let mut tree_response: Signal<Option<WorkflowTreeResponse>> =
        use_signal(|| None);
    let mut request_seq: Signal<u64> = use_signal(|| 0);

    {
        let workspace = props.workspace.clone();
        let mode = props.mode;
        let root_ticket = props.root_ticket.clone();
        use_effect(move || {
            let request_id = request_seq.with_mut(|value| {
                *value += 1;
                *value
            });
            let request_workspace = workspace.clone();
            error.set(None);
            next_response.set(None);
            tree_response.set(None);

            if !matches!(mode, WorkflowSidebarMode::Next)
                && root_ticket.is_none()
            {
                loading.set(false);
                return;
            }

            loading.set(true);
            let root_id = root_ticket.as_ref().map(|ticket| ticket.id.clone());
            spawn(async move {
                let backend = HttpTicketBackend::new(None);
                let result = match mode {
                    WorkflowSidebarMode::Next => backend
                        .get_workflow_next(&request_workspace)
                        .await
                        .map(WorkflowFetchResult::Next),
                    WorkflowSidebarMode::Blockers => backend
                        .get_workflow_blockers(
                            &request_workspace,
                            root_id.as_deref().unwrap_or_default(),
                        )
                        .await
                        .map(WorkflowFetchResult::Tree),
                    WorkflowSidebarMode::UnblockedBy => backend
                        .get_workflow_unblocked_by(
                            &request_workspace,
                            root_id.as_deref().unwrap_or_default(),
                        )
                        .await
                        .map(WorkflowFetchResult::Tree),
                };

                if *request_seq.peek() != request_id {
                    return;
                }

                match result {
                    Ok(WorkflowFetchResult::Next(response)) => {
                        next_response.set(Some(response));
                    },
                    Ok(WorkflowFetchResult::Tree(response)) => {
                        tree_response.set(Some(response));
                    },
                    Err(message) => error.set(Some(message)),
                }
                loading.set(false);
            });
        });
    }

    let mode = props.mode;
    let root_ticket = props.root_ticket.clone();
    let selected_ticket = props.selected_ticket.clone();
    let can_use_selected_root = selected_ticket
        .as_ref()
        .filter(|ticket| Some(*ticket) != root_ticket.as_ref())
        .is_some();
    let current_root_label = root_ticket
        .as_ref()
        .map(ticket_ref_label)
        .unwrap_or_else(|| "No root pinned".to_string());

    rsx! {
        div {
            style: "
                display: flex;
                flex-direction: column;
                height: 100%;
                min-height: 0;
            ",
            div {
                style: "
                    padding: 10px 12px 8px;
                    border-bottom: 1px solid var(--border-subtle);
                    display: flex;
                    flex-direction: column;
                    gap: 6px;
                ",
                div {
                    style: "display: flex; align-items: center; justify-content: space-between; gap: 8px;",
                    strong {
                        style: "font-size: 12px; color: var(--text-primary);",
                        "{mode.label()}"
                    }
                    if !matches!(mode, WorkflowSidebarMode::Next) && can_use_selected_root {
                        button {
                            class: "btn btn-secondary btn-sm",
                            style: "font-size: 11px; padding: 3px 8px; min-height: 24px;",
                            "data-testid": "workflow-use-selected-root",
                            onclick: move |_| props.on_use_selected_root.call(()),
                            "Use selected"
                        }
                    }
                }
                div {
                    style: "font-size: 11px; color: var(--text-muted); line-height: 1.4;",
                    "{mode.description()}"
                }
                if !matches!(mode, WorkflowSidebarMode::Next) {
                    div {
                        style: "font-size: 11px; color: var(--text-secondary);",
                        "Pinned root: {current_root_label}"
                    }
                }
            }
            div {
                style: "
                    flex: 1;
                    min-height: 0;
                    overflow: auto;
                    display: flex;
                    flex-direction: column;
                    gap: 8px;
                    padding: 10px 12px 14px;
                ",
                if *loading.read() {
                    div {
                        class: "sidebar-loading",
                        "data-testid": "workflow-loading",
                        "Loading workflow…"
                    }
                } else if let Some(message) = error.read().clone() {
                    div {
                        style: "
                            padding: 10px 12px;
                            border: 1px solid color-mix(in srgb, var(--error) 40%, transparent);
                            border-radius: 8px;
                            background: color-mix(in srgb, var(--error) 10%, transparent);
                            color: var(--error);
                            font-size: 12px;
                            line-height: 1.4;
                        ",
                        "data-testid": "workflow-error",
                        "Failed to load workflow data: {message}"
                    }
                } else if !matches!(mode, WorkflowSidebarMode::Next) && root_ticket.is_none() {
                    div {
                        style: "
                            padding: 12px;
                            border: 1px dashed var(--border-subtle);
                            border-radius: 8px;
                            color: var(--text-muted);
                            font-size: 12px;
                            line-height: 1.4;
                            display: flex;
                            flex-direction: column;
                            gap: 8px;
                        ",
                        "data-testid": "workflow-root-empty",
                        div { "Pick a ticket in Browse or Actionable next, then pin it here." }
                        if props.selected_ticket.is_some() {
                            button {
                                class: "btn btn-secondary btn-sm",
                                style: "align-self: flex-start; font-size: 11px; padding: 3px 8px; min-height: 24px;",
                                "data-testid": "workflow-root-empty-use-selected",
                                onclick: move |_| props.on_use_selected_root.call(()),
                                "Inspect current selection"
                            }
                        }
                    }
                } else if matches!(mode, WorkflowSidebarMode::Next) {
                    {render_next_panel(
                        props.workspace.clone(),
                        next_response.read().clone(),
                        props.on_select.clone(),
                    )}
                } else {
                    {render_tree_panel(
                        props.workspace.clone(),
                        mode,
                        tree_response.read().clone(),
                        props.on_select.clone(),
                    )}
                }
            }
        }
    }
}

fn render_next_panel(
    workspace: String,
    response: Option<WorkflowNextResponse>,
    on_select: EventHandler<TicketRef>,
) -> Element {
    let Some(response) = response else {
        return rsx! {
            div {
                class: "sidebar-loading",
                "data-testid": "workflow-next-empty",
                "No workflow results yet."
            }
        };
    };

    let total = response.count;
    let items = response.items;
    let active_workspace = if response.active_workspace.is_empty() {
        workspace
    } else {
        response.active_workspace
    };

    rsx! {
        div {
            style: "display: flex; flex-direction: column; gap: 8px;",
            div {
                style: "font-size: 11px; color: var(--text-muted);",
                "{total} ranked item(s)"
            }
            if items.is_empty() {
                div {
                    style: empty_panel_style(),
                    "data-testid": "workflow-next-empty",
                    "No actionable workflow candidates are currently available."
                }
            } else {
                for item in items {
                    {render_candidate_card(
                        &active_workspace,
                        item,
                        false,
                        on_select.clone(),
                    )}
                }
            }
        }
    }
}

fn render_tree_panel(
    workspace: String,
    mode: WorkflowSidebarMode,
    response: Option<WorkflowTreeResponse>,
    on_select: EventHandler<TicketRef>,
) -> Element {
    let Some(response) = response else {
        return rsx! {
            div {
                class: "sidebar-loading",
                "data-testid": "workflow-tree-empty",
                "No workflow tree available yet."
            }
        };
    };

    let active_workspace = if response.active_workspace.is_empty() {
        workspace
    } else {
        response.active_workspace
    };
    let frontier_total = response.frontier_count;
    let reachable_dependents = response.reachable_dependents;
    let blocked_dependents = response.blocked_dependents;
    let root = response.root;
    let frontier_items = response.frontier_items;

    rsx! {
        div {
            style: "display: flex; flex-direction: column; gap: 10px;",
            div {
                style: panel_style(true),
                "data-testid": "workflow-root-card",
                div {
                    style: "display: flex; align-items: flex-start; justify-content: space-between; gap: 8px;",
                    div {
                        style: "min-width: 0; display: flex; flex-direction: column; gap: 4px;",
                        strong {
                            style: "font-size: 13px; color: var(--text-primary);",
                            "{title_or_untitled(root.title.as_deref())}"
                        }
                        div {
                            style: "font-size: 11px; color: var(--text-muted);",
                            "{short_id(&root.id)} · {root.ticket_type}"
                        }
                    }
                    {render_state_badge(root.state.as_deref().unwrap_or("open"))}
                }
                div {
                    style: "font-size: 11px; color: var(--text-secondary); display: flex; flex-wrap: wrap; gap: 6px 10px;",
                    span { "Frontier leaves {frontier_total}" }
                    span { "Remaining blockers {root.remaining_blocker_count}" }
                    span { "State gap {root.dependency_state_gap}" }
                    if let Some(count) = reachable_dependents {
                        span { "Reachable dependents {count}" }
                    }
                    if let Some(count) = blocked_dependents {
                        span { "Still blocked {count}" }
                    }
                }
                if let Some(evidence) = workflow_evidence(
                    root.became_actionable_at.as_deref(),
                    root.last_blocker_progress_at.as_deref(),
                ) {
                    div {
                        style: evidence_style(),
                        "Evidence: {evidence}"
                    }
                }
            }
            div {
                style: "display: flex; flex-direction: column; gap: 8px;",
                "data-testid": if matches!(mode, WorkflowSidebarMode::Blockers) {
                    "workflow-tree-blockers"
                } else {
                    "workflow-tree-unblocked-by"
                },
                {render_tree_node(
                    &active_workspace,
                    root,
                    0,
                    on_select.clone(),
                )}
            }
            div {
                style: "display: flex; flex-direction: column; gap: 8px;",
                div {
                    style: "font-size: 11px; color: var(--text-muted);",
                    "Frontier evidence"
                }
                if frontier_items.is_empty() {
                    div {
                        style: empty_panel_style(),
                        "data-testid": "workflow-frontier-empty",
                        "No frontier tickets are currently exposed for this tree."
                    }
                } else {
                    for item in frontier_items {
                        {render_candidate_card(
                            &active_workspace,
                            item,
                            true,
                            on_select.clone(),
                        )}
                    }
                }
            }
        }
    }
}

fn render_tree_node(
    active_workspace: &str,
    node: WorkflowTreeItem,
    depth: usize,
    on_select: EventHandler<TicketRef>,
) -> Element {
    let WorkflowTreeItem {
        id,
        ticket_ref,
        title,
        state,
        ticket_type,
        priority,
        remaining_blocker_count,
        unresolved_frontier_leaf_count,
        frontier_leaf_ids,
        blocker_distance,
        is_frontier,
        dependency_count,
        immediate_dependees,
        transitive_reverse_dependents,
        affected_reverse_dependent_reach,
        dependency_state_gap,
        became_actionable_at,
        last_blocker_progress_at,
        children,
    } = node;
    let ticket_ref =
        if !ticket_ref.workspace.is_empty() && !ticket_ref.id.is_empty() {
            ticket_ref
        } else {
            TicketRef::new(active_workspace, id.clone())
        };
    let depth_px = depth * 14;

    rsx! {
        div {
            key: "workflow-tree-{id}",
            style: "display: flex; flex-direction: column; gap: 8px; margin-left: {depth_px}px;",
            button {
                class: "btn",
                style: panel_style(is_frontier),
                "data-testid": "workflow-tree-node-{id}",
                "data-depth": "{depth}",
                onclick: move |_| on_select.call(ticket_ref.clone()),
                div {
                    style: "display: flex; align-items: flex-start; justify-content: space-between; gap: 8px; width: 100%; text-align: left;",
                    div {
                        style: "min-width: 0; display: flex; flex-direction: column; gap: 4px;",
                        div {
                            style: "display: flex; align-items: center; flex-wrap: wrap; gap: 6px;",
                            strong {
                                style: "font-size: 12px; color: var(--text-primary);",
                                "{title_or_untitled(title.as_deref())}"
                            }
                            if is_frontier {
                                span {
                                    style: frontier_pill_style(),
                                    "data-testid": "workflow-tree-frontier-{id}",
                                    "Frontier"
                                }
                            }
                        }
                        div {
                            style: "font-size: 11px; color: var(--text-muted);",
                            "{short_id(&id)} · {ticket_type} · priority {priority}"
                        }
                    }
                    {render_state_badge(state.as_deref().unwrap_or("open"))}
                }
                div {
                    style: "font-size: 11px; color: var(--text-secondary); display: flex; flex-wrap: wrap; gap: 6px 10px; width: 100%; text-align: left;",
                    span { "Distance {blocker_distance}" }
                    span { "Remaining blockers {remaining_blocker_count}" }
                    span { "Frontier leaves {unresolved_frontier_leaf_count}" }
                    span { "Dependencies {dependency_count}" }
                    span { "Immediate dependee count {immediate_dependees}" }
                    span { "Reach {affected_reverse_dependent_reach}" }
                    span { "Gap {dependency_state_gap}" }
                }
                if let Some(evidence) = workflow_evidence(
                    became_actionable_at.as_deref(),
                    last_blocker_progress_at.as_deref(),
                ) {
                    div {
                        style: evidence_style(),
                        "Evidence: {evidence}"
                    }
                }
                if !frontier_leaf_ids.is_empty() || transitive_reverse_dependents > 0 {
                    div {
                        style: "font-size: 11px; color: var(--text-muted); width: 100%; text-align: left;",
                        if !frontier_leaf_ids.is_empty() {
                            span { "Leaf ids {render_short_id_list(&frontier_leaf_ids)}" }
                        }
                        if transitive_reverse_dependents > 0 {
                            span { " · Reverse dependents {transitive_reverse_dependents}" }
                        }
                    }
                }
            }
            for child in children {
                {render_tree_node(
                    active_workspace,
                    child,
                    depth + 1,
                    on_select.clone(),
                )}
            }
        }
    }
}

fn render_candidate_card(
    active_workspace: &str,
    item: WorkflowCandidateItem,
    frontier: bool,
    on_select: EventHandler<TicketRef>,
) -> Element {
    let WorkflowCandidateItem {
        rank,
        id,
        ticket_ref,
        title,
        state,
        ticket_type,
        priority,
        dependency_count,
        remaining_blocker_count,
        dependee_count,
        transitive_reverse_dependents,
        affected_reverse_dependent_reach,
        max_affected_dependent_state,
        dependency_state_gap,
        became_actionable_at,
        last_blocker_progress_at,
        created_at: _,
    } = item;
    let ticket_ref =
        if !ticket_ref.workspace.is_empty() && !ticket_ref.id.is_empty() {
            ticket_ref
        } else {
            TicketRef::new(active_workspace, id.clone())
        };

    rsx! {
        button {
            key: "workflow-candidate-{id}",
            class: "btn",
            style: panel_style(frontier),
            "data-testid": if frontier {
                format!("workflow-frontier-item-{id}")
            } else {
                format!("workflow-next-item-{id}")
            },
            onclick: move |_| on_select.call(ticket_ref.clone()),
            div {
                style: "display: flex; align-items: flex-start; justify-content: space-between; gap: 8px; width: 100%; text-align: left;",
                div {
                    style: "min-width: 0; display: flex; flex-direction: column; gap: 4px;",
                    div {
                        style: "display: flex; align-items: center; flex-wrap: wrap; gap: 6px;",
                        span {
                            style: rank_pill_style(frontier),
                            "#{rank}"
                        }
                        strong {
                            style: "font-size: 12px; color: var(--text-primary);",
                            "{title_or_untitled(title.as_deref())}"
                        }
                    }
                    div {
                        style: "font-size: 11px; color: var(--text-muted);",
                        "{short_id(&id)} · {ticket_type} · priority {priority}"
                    }
                }
                {render_state_badge(state.as_deref().unwrap_or("open"))}
            }
            div {
                style: "font-size: 11px; color: var(--text-secondary); display: flex; flex-wrap: wrap; gap: 6px 10px; width: 100%; text-align: left;",
                span { "Remaining blockers {remaining_blocker_count}" }
                span { "Dependencies {dependency_count}" }
                span { "Dependee count {dependee_count}" }
                span { "Reach {affected_reverse_dependent_reach}" }
                span { "Gap {dependency_state_gap}" }
                if transitive_reverse_dependents > 0 {
                    span { "Reverse dependents {transitive_reverse_dependents}" }
                }
            }
            if let Some(state) = max_affected_dependent_state {
                div {
                    style: "font-size: 11px; color: var(--text-muted); width: 100%; text-align: left;",
                    "Highest affected dependent state {state}"
                }
            }
            if let Some(evidence) = workflow_evidence(
                became_actionable_at.as_deref(),
                last_blocker_progress_at.as_deref(),
            ) {
                div {
                    style: evidence_style(),
                    "Evidence: {evidence}"
                }
            }
        }
    }
}

fn render_state_badge(state: &str) -> Element {
    let (bg, fg) = state_colors(state);

    rsx! {
        span {
            style: "
                padding: 2px 7px;
                border-radius: 999px;
                background: {bg};
                color: {fg};
                font-size: 10px;
                font-weight: 600;
                white-space: nowrap;
                flex-shrink: 0;
            ",
            "{state}"
        }
    }
}

fn workflow_evidence(
    became_actionable_at: Option<&str>,
    last_blocker_progress_at: Option<&str>,
) -> Option<String> {
    match (
        format_workflow_timestamp(became_actionable_at),
        format_workflow_timestamp(last_blocker_progress_at),
    ) {
        (Some(actionable), Some(progress)) => Some(format!(
            "Actionable since {actionable}; last blocker progress {progress}"
        )),
        (Some(actionable), None) =>
            Some(format!("Actionable since {actionable}")),
        (None, Some(progress)) =>
            Some(format!("Last blocker progress {progress}")),
        (None, None) => None,
    }
}

fn format_workflow_timestamp(value: Option<&str>) -> Option<String> {
    let value = value?;
    let (date, time_with_zone) = value.split_once('T')?;
    let mut cutoff = time_with_zone.len();
    for marker in ['.', '+', 'Z'] {
        if let Some(index) = time_with_zone.find(marker) {
            cutoff = cutoff.min(index);
        }
    }
    let time = &time_with_zone[..cutoff];
    let suffix = if value.ends_with("+00:00") || value.ends_with('Z') {
        " UTC"
    } else {
        ""
    };
    Some(format!("{date} {time}{suffix}"))
}

fn ticket_ref_label(ticket_ref: &TicketRef) -> String {
    format!("{} ({})", ticket_ref.id, ticket_ref.workspace)
}

fn title_or_untitled(title: Option<&str>) -> &str {
    title.unwrap_or("Untitled")
}

fn short_id(id: &str) -> String {
    id.chars().take(8).collect()
}

fn render_short_id_list(ids: &[String]) -> String {
    ids.iter()
        .map(|id| short_id(id))
        .collect::<Vec<_>>()
        .join(", ")
}

fn panel_style(highlight: bool) -> String {
    format!(
        "
            display: flex;
            flex-direction: column;
            gap: 6px;
            width: 100%;
            padding: 10px 12px;
            border: 1px solid {};
            border-radius: 10px;
            background: {};
            color: var(--text-primary);
            cursor: pointer;
            text-align: left;
        ",
        if highlight {
            "color-mix(in srgb, var(--accent-blue) 50%, var(--border-subtle))"
        } else {
            "var(--border-subtle)"
        },
        if highlight {
            "color-mix(in srgb, var(--accent-blue) 10%, var(--bg-secondary))"
        } else {
            "var(--bg-secondary)"
        },
    )
}

fn frontier_pill_style() -> &'static str {
    "
        padding: 2px 6px;
        border-radius: 999px;
        background: color-mix(in srgb, var(--accent-blue) 18%, transparent);
        color: var(--accent-blue);
        font-size: 10px;
        font-weight: 700;
        letter-spacing: 0.02em;
        text-transform: uppercase;
    "
}

fn rank_pill_style(frontier: bool) -> &'static str {
    if frontier {
        "
            min-width: 28px;
            padding: 2px 6px;
            border-radius: 999px;
            background: color-mix(in srgb, var(--accent-blue) 20%, transparent);
            color: var(--accent-blue);
            font-size: 10px;
            font-weight: 700;
            text-align: center;
        "
    } else {
        "
            min-width: 28px;
            padding: 2px 6px;
            border-radius: 999px;
            background: color-mix(in srgb, var(--text-muted) 20%, transparent);
            color: var(--text-secondary);
            font-size: 10px;
            font-weight: 700;
            text-align: center;
        "
    }
}

fn evidence_style() -> &'static str {
    "
        font-size: 11px;
        color: var(--text-muted);
        width: 100%;
        text-align: left;
        line-height: 1.4;
    "
}

fn empty_panel_style() -> &'static str {
    "
        padding: 12px;
        border: 1px dashed var(--border-subtle);
        border-radius: 8px;
        color: var(--text-muted);
        font-size: 12px;
        line-height: 1.4;
    "
}
