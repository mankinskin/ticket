use std::collections::HashSet;

use dioxus::prelude::*;

use crate::{
    api::{
        HttpTicketBackend,
        TicketBackend,
    },
    types::{
        HistoryEntry,
        ProjectedPart,
        ProjectedRef,
    },
};

use super::{
    edit::render_edit_panel,
    helpers::{
        content_tab_label,
        fields_to_toml,
    },
    render::{
        render_document_panel,
        render_full_document_panel,
        render_history_panel,
        render_tab_bar,
        render_toml_panel,
        TicketDocumentContext,
    },
    Tab,
};

#[component]
pub fn TicketContent(
    workspace: String,
    summary_workspace: String,
    ticket_id: String,
    fields: serde_json::Value,
    #[props(default)] ticket_title: Option<String>,
    #[props(default)] ticket_state: Option<String>,
    #[props(default)] ticket_type: Option<String>,
    #[props(default)] created_at: Option<String>,
    #[props(default)] updated_at: Option<String>,
    #[props(default)] asset_path: Option<String>,
) -> Element {
    let mut active_tab: Signal<Tab> = use_signal(|| Tab::Description);
    let mut document_title: Signal<Option<String>> =
        use_signal(|| ticket_title.clone());
    let mut document_state: Signal<Option<String>> =
        use_signal(|| ticket_state.clone());
    let mut document_type: Signal<Option<String>> =
        use_signal(|| ticket_type.clone());
    let mut document_fields: Signal<serde_json::Value> =
        use_signal(|| fields.clone());
    let mut document_created_at: Signal<Option<String>> =
        use_signal(|| created_at.clone());
    let mut document_updated_at: Signal<Option<String>> =
        use_signal(|| updated_at.clone());
    let mut desc_loading: Signal<bool> = use_signal(|| true);
    let mut desc_text: Signal<Option<String>> = use_signal(|| None);
    let mut desc_error: Signal<Option<String>> = use_signal(|| None);
    let mut full_loading: Signal<bool> = use_signal(|| true);
    let mut full_error: Signal<Option<String>> = use_signal(|| None);
    let mut full_parts: Signal<Vec<ProjectedPart>> = use_signal(Vec::new);
    let mut full_refs: Signal<Option<Vec<ProjectedRef>>> = use_signal(|| None);
    let collapsed_parts: Signal<HashSet<String>> = use_signal(HashSet::new);
    let mut history_entries: Signal<Vec<HistoryEntry>> = use_signal(|| vec![]);
    let history_refresh_key: Signal<u32> = use_signal(|| 0);
    let mut edit_draft: Signal<String> = use_signal(String::new);
    let edit_pending: Signal<bool> = use_signal(|| false);
    let mut edit_success: Signal<bool> = use_signal(|| false);
    let mut edit_error: Signal<Option<String>> = use_signal(|| None);

    let is_description = asset_path
        .as_deref()
        .map(|path| path == "description.md")
        .unwrap_or(true);
    let content_tab_label =
        content_tab_label(asset_path.as_deref(), is_description);
    let show_edit_tab = is_description && HttpTicketBackend::has_auth_token();
    let document_context = TicketDocumentContext {
        workspace: workspace.clone(),
        ticket_id: ticket_id.clone(),
        title: document_title(),
        ticket_state: document_state(),
        ticket_type: document_type(),
        created_at: document_created_at(),
        updated_at: document_updated_at(),
        fields: document_fields.read().clone(),
    };

    {
        let workspace = workspace.clone();
        let summary_workspace = summary_workspace.clone();
        let ticket_id = ticket_id.clone();
        use_effect(move || {
            let workspace = workspace.clone();
            let summary_workspace = summary_workspace.clone();
            let ticket_id = ticket_id.clone();
            let summary_query = format!("id:{ticket_id}");
            spawn(async move {
                let backend = HttpTicketBackend::new(None);
                if let Ok(response) = backend
                    .list_tickets(
                        &summary_workspace,
                        None,
                        Some(summary_query.as_str()),
                        Some(1),
                    )
                    .await
                {
                    if let Some(summary) =
                        response.items.into_iter().find(|ticket| {
                            ticket.resolved_ticket_ref(&summary_workspace).id
                                == ticket_id
                        })
                    {
                        document_title.set(summary.title);
                        document_state.set(summary.state);
                        document_type.set(summary.ticket_type);
                        document_created_at.set(Some(summary.created_at));
                        document_updated_at.set(
                            if summary.updated_at.trim().is_empty() {
                                None
                            } else {
                                Some(summary.updated_at)
                            },
                        );
                        document_fields.set(summary.fields);
                    }
                }
                if let Ok(response) =
                    backend.get_ticket(&workspace, &ticket_id).await
                {
                    document_fields.set(response.ticket.fields);
                    document_created_at.set(Some(response.ticket.created_at));
                }
            });
        });
    }

    {
        let workspace = workspace.clone();
        let ticket_id = ticket_id.clone();
        let asset_path = asset_path.clone();
        let is_description = is_description;
        use_effect(move || {
            let workspace = workspace.clone();
            let ticket_id = ticket_id.clone();
            let asset_path = asset_path.clone();
            desc_loading.set(true);
            desc_text.set(None);
            desc_error.set(None);
            edit_draft.set(String::new());
            edit_success.set(false);
            edit_error.set(None);
            active_tab.set(Tab::Description);
            spawn(async move {
                let backend = HttpTicketBackend::new(None);
                if is_description {
                    match backend
                        .get_ticket_description(&workspace, &ticket_id)
                        .await
                    {
                        Ok(response) => {
                            let content =
                                response.description.unwrap_or_default();
                            edit_draft.set(content.clone());
                            desc_text.set(if content.is_empty() {
                                None
                            } else {
                                Some(content)
                            });
                            desc_loading.set(false);
                        },
                        Err(error) => {
                            desc_error.set(Some(error));
                        },
                    }
                } else {
                    let path = asset_path.unwrap_or_default();
                    match backend
                        .get_ticket_asset(&workspace, &ticket_id, &path)
                        .await
                    {
                        Ok(response) => {
                            let content = response.content;
                            desc_text.set(if content.is_empty() {
                                None
                            } else {
                                Some(content)
                            });
                            desc_loading.set(false);
                        },
                        Err(error) => {
                            desc_loading.set(false);
                            desc_error.set(Some(error));
                        },
                    }
                }
            });
        });
    }

    {
        let workspace = workspace.clone();
        let ticket_id = ticket_id.clone();
        let is_description = is_description;
        use_effect(move || {
            let workspace = workspace.clone();
            let ticket_id = ticket_id.clone();
            if !is_description {
                full_loading.set(false);
                full_parts.set(Vec::new());
                full_refs.set(None);
                return;
            }
            full_loading.set(true);
            full_error.set(None);
            spawn(async move {
                let backend = HttpTicketBackend::new(None);
                match backend.get_ticket_full(&workspace, &ticket_id).await {
                    Ok(response) => {
                        full_parts.set(response.ticket.parts);
                        full_refs.set(response.ticket.refs);
                        full_loading.set(false);
                    },
                    Err(error) => {
                        full_error.set(Some(error));
                        full_loading.set(false);
                    },
                }
            });
        });
    }

    {
        let workspace = workspace.clone();
        let ticket_id = ticket_id.clone();
        use_effect(move || {
            let _refresh = history_refresh_key();
            let workspace = workspace.clone();
            let ticket_id = ticket_id.clone();
            history_entries.set(vec![]);
            spawn(async move {
                let backend = HttpTicketBackend::new(None);
                if let Ok(response) =
                    backend.get_ticket_history(&workspace, &ticket_id).await
                {
                    history_entries.set(response.entries);
                }
            });
        });
    }

    let toml_text = fields_to_toml(&ticket_id, &document_fields.read());
    let body_style = if active_tab() == Tab::History {
        "flex: 1; overflow: hidden;"
    } else if active_tab() == Tab::Description {
        "flex: 1; overflow-y: auto;"
    } else {
        "flex: 1; overflow-y: auto; padding: 20px 24px;"
    };

    rsx! {
        div {
            "data-testid": "ticket-content",
            // Keep the frosted blur on a stable CSS class instead of the changing
            // inline style string used by the overlay width. The outer wrapper owns
            // the border/shadow frame; this inner shell owns the translucent blur.
            class: "ticket-content-shell",
            style: "
                background: transparent;
            ",
            {render_tab_bar(
                active_tab,
                content_tab_label,
                show_edit_tab,
                history_entries().len(),
            )}
            div {
                "data-testid": "ticket-content-body",
                style: "{body_style}",
                if active_tab() == Tab::Description {
                    if is_description {
                        {render_full_document_panel(
                            document_context.clone(),
                            full_loading(),
                            full_error(),
                            full_parts(),
                            full_refs(),
                            collapsed_parts,
                        )}
                    } else {
                        {render_document_panel(
                            document_context.clone(),
                            desc_loading(),
                            desc_error(),
                            desc_text(),
                            asset_path.clone(),
                        )}
                    }
                }
                if active_tab() == Tab::Toml {
                    {render_toml_panel(toml_text)}
                }
                if active_tab() == Tab::History {
                    {render_history_panel(
                        workspace.clone(),
                        ticket_id.clone(),
                        history_entries(),
                        history_refresh_key,
                    )}
                }
                if active_tab() == Tab::Edit {
                    {render_edit_panel(
                        workspace,
                        ticket_id,
                        desc_text,
                        edit_draft,
                        edit_pending,
                        edit_success,
                        edit_error,
                    )}
                }
            }
        }
    }
}
