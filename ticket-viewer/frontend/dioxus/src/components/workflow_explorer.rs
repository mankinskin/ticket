use dioxus::prelude::*;

use crate::{
    api::{HttpTicketBackend, TicketBackend},
    types::{
        TicketRef, WorkflowCandidateItem, WorkflowNextResponse, WorkflowTreeItem,
        WorkflowTreeResponse, state_colors,
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
            Self::Blockers => "Blocking prerequisites for the pinned root ticket.",
            Self::UnblockedBy => "Work that opens up once the pinned root ticket is satisfied.",
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
    let mut next_response: Signal<Option<WorkflowNextResponse>> = use_signal(|| None);
    let mut tree_response: Signal<Option<WorkflowTreeResponse>> = use_signal(|| None);
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

            if !matches!(mode, WorkflowSidebarMode::Next) && root_ticket.is_none() {
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
                    }
                    Ok(WorkflowFetchResult::Tree(response)) => {
                        tree_response.set(Some(response));
                    }
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

#[path = "workflow_explorer/panels.rs"]
mod panels;
#[allow(unused_imports)]
pub use panels::*;

#[path = "workflow_explorer/formatting.rs"]
mod formatting;
#[allow(unused_imports)]
pub use formatting::*;
