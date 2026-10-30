#[allow(unused_imports)]
use super::*;

pub(super) fn render_next_panel(
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

pub(super) fn render_tree_panel(
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

pub(super) fn render_tree_node(
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
    let ticket_ref = if !ticket_ref.workspace.is_empty() && !ticket_ref.id.is_empty() {
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

pub(super) fn render_candidate_card(
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
    let ticket_ref = if !ticket_ref.workspace.is_empty() && !ticket_ref.id.is_empty() {
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

pub(super) fn render_state_badge(state: &str) -> Element {
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
