use dioxus::prelude::*;
use viewer_api_dioxus::FileContentViewer;

use crate::api::{
    HttpTicketBackend,
    TicketBackend,
};

#[allow(unused_mut)]
pub(super) fn render_edit_panel(
    workspace: String,
    ticket_id: String,
    mut desc_text: Signal<Option<String>>,
    mut edit_draft: Signal<String>,
    mut edit_pending: Signal<bool>,
    mut edit_success: Signal<bool>,
    mut edit_error: Signal<Option<String>>,
) -> Element {
    let edit_btn_opacity = if edit_pending() { "0.6" } else { "1" };

    rsx! {
        div {
            "data-testid": "edit-panel",
            style: "
                display: flex;
                gap: 16px;
                min-height: 480px;
                height: 100%;
            ",
            div {
                style: "
                    flex: 1;
                    display: flex;
                    flex-direction: column;
                    gap: 8px;
                    min-width: 0;
                ",
                textarea {
                    "data-testid": "edit-textarea",
                    "aria-label": "Markdown editor",
                    style: "
                        flex: 1;
                        min-height: 380px;
                        width: 100%;
                        box-sizing: border-box;
                        background: #13131f;
                        color: #e0e0e8;
                        border: 1px solid #3a3a4a;
                        border-radius: 4px;
                        padding: 10px 12px;
                        font-family: 'Cascadia Code', 'Fira Code', monospace;
                        font-size: 12.5px;
                        line-height: 1.6;
                        resize: vertical;
                        outline: none;
                    ",
                    disabled: edit_pending(),
                    value: "{edit_draft()}",
                    oninput: move |event: Event<FormData>| {
                        edit_draft.set(event.value().to_string());
                        edit_success.set(false);
                        edit_error.set(None);
                    },
                    onblur: {
                        let workspace = workspace.clone();
                        let ticket_id = ticket_id.clone();
                        move |_| {
                            if edit_pending() {
                                return;
                            }
                            let text = edit_draft();
                            save_description(
                                workspace.clone(),
                                ticket_id.clone(),
                                text,
                                desc_text,
                                edit_draft,
                                edit_pending,
                                edit_success,
                                edit_error,
                            );
                        }
                    },
                }
                div {
                    style: "
                        display: flex;
                        align-items: center;
                        gap: 10px;
                        flex-shrink: 0;
                    ",
                    button {
                        "data-testid": "edit-save-btn",
                        style: "
                            padding: 5px 16px;
                            border-radius: 4px;
                            border: none;
                            background: #6366f1;
                            color: white;
                            cursor: pointer;
                            font-size: 12px;
                            font-weight: 600;
                            opacity: {edit_btn_opacity};
                        ",
                        disabled: edit_pending(),
                        onclick: {
                            let workspace = workspace.clone();
                            let ticket_id = ticket_id.clone();
                            move |_| {
                                if edit_pending() {
                                    return;
                                }
                                let text = edit_draft();
                                save_description(
                                    workspace.clone(),
                                    ticket_id.clone(),
                                    text,
                                    desc_text,
                                    edit_draft,
                                    edit_pending,
                                    edit_success,
                                    edit_error,
                                );
                            }
                        },
                        if edit_pending() { "Saving…" } else { "Save" }
                    }
                    if edit_success() {
                        span {
                            "data-testid": "edit-success",
                            style: "font-size: 12px; color: #4ade80;",
                            "✓ Saved"
                        }
                    }
                    if let Some(error) = edit_error() {
                        span {
                            "data-testid": "edit-error",
                            style: "font-size: 12px; color: #f87171;",
                            "Error: {error}"
                        }
                    }
                }
            }
            div {
                style: "
                    flex: 1;
                    overflow-y: auto;
                    min-width: 0;
                    border-left: 1px solid #3a3a4a;
                    padding-left: 16px;
                ",
                if edit_draft().is_empty() {
                    em {
                        "data-testid": "edit-preview-empty",
                        style: "color: #6b7280; font-size: 13px;",
                        "Preview will appear here…"
                    }
                } else {
                    div {
                        "data-testid": "edit-preview",
                        FileContentViewer {
                            content: edit_draft(),
                            filename: "description.md".to_string(),
                        }
                    }
                }
            }
        }
    }
}

fn save_description(
    workspace: String,
    ticket_id: String,
    text: String,
    mut desc_text: Signal<Option<String>>,
    mut edit_draft: Signal<String>,
    mut edit_pending: Signal<bool>,
    mut edit_success: Signal<bool>,
    mut edit_error: Signal<Option<String>>,
) {
    edit_pending.set(true);
    edit_error.set(None);
    edit_success.set(false);

    spawn(async move {
        let token = HttpTicketBackend::read_auth_token();
        let backend = HttpTicketBackend::new(token);
        match backend
            .update_ticket_description(&workspace, &ticket_id, &text)
            .await
        {
            Ok(()) => {
                desc_text.set(if text.is_empty() { None } else { Some(text) });
                edit_pending.set(false);
                edit_success.set(true);
            },
            Err(error) => {
                edit_draft.set(desc_text().unwrap_or_default());
                edit_pending.set(false);
                edit_error.set(Some(error));
            },
        }
    });
}
