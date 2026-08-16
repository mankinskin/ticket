use dioxus::prelude::*;

use crate::{
    api::HttpTicketBackend,
    types::{
        SchemaListResponse,
        TicketSummary,
    },
};

use super::{
    actions::{
        build_request,
        load_schemas,
        selected_type_schema,
        spawn_create_ticket,
        validate_draft,
    },
    draft::{
        clear_draft,
        load_draft,
        DraftState,
    },
    fields::{
        render_schema_fields,
        render_static_fields,
        render_type_selector,
    },
};

#[derive(Props, Clone, PartialEq)]
pub struct CreateTicketModalProps {
    pub workspace: String,
    #[props(default)]
    pub prefill: Option<TicketSummary>,
    pub on_cancel: EventHandler<()>,
}

#[component]
pub fn CreateTicketModal(props: CreateTicketModalProps) -> Element {
    let workspace = props.workspace.clone();
    let nav = use_navigator();
    let backend = HttpTicketBackend::new(None);

    #[cfg_attr(not(target_arch = "wasm32"), allow(unused_mut))]
    let mut schema_list: Signal<Option<SchemaListResponse>> =
        use_signal(|| None);
    #[cfg_attr(not(target_arch = "wasm32"), allow(unused_mut))]
    let mut schema_err: Signal<Option<String>> = use_signal(|| None);
    #[cfg_attr(not(target_arch = "wasm32"), allow(unused_mut))]
    let mut draft: Signal<DraftState> =
        use_signal(|| initial_draft(&props.prefill));
    let show_optional: Signal<bool> = use_signal(|| false);
    #[cfg_attr(not(target_arch = "wasm32"), allow(unused_mut))]
    let mut submitting: Signal<bool> = use_signal(|| false);
    #[cfg_attr(not(target_arch = "wasm32"), allow(unused_mut))]
    let mut submit_err: Signal<Option<String>> = use_signal(|| None);

    load_schemas(backend.clone(), workspace.clone(), schema_list, schema_err);

    let current_type_id = draft.read().type_id.clone();
    let maybe_type_schema =
        selected_type_schema(schema_list.read().as_ref(), &current_type_id);

    let on_submit = EventHandler::new({
        let backend = backend.clone();
        let workspace = workspace.clone();
        let nav = nav.clone();
        move |_: MouseEvent| {
            if submitting() {
                return;
            }

            let draft_snapshot = draft.read().clone();
            if let Some(error) = validate_draft(&draft_snapshot) {
                submit_err.set(Some(error));
                return;
            }

            submitting.set(true);
            submit_err.set(None);
            let request = build_request(&draft_snapshot);
            spawn_create_ticket(
                backend.clone(),
                workspace.clone(),
                request,
                nav.clone(),
                submitting,
                submit_err,
            );
        }
    });

    let on_cancel = EventHandler::new({
        let handler = props.on_cancel.clone();
        move |_: MouseEvent| {
            clear_draft();
            handler.call(());
        }
    });

    rsx! {
        div {
            style: "
                position: fixed; inset: 0; z-index: 200;
                background: rgba(0,0,0,0.65);
                display: flex; align-items: center; justify-content: center;
                font-family: sans-serif;
            ",
            div {
                style: "
                    background: #1a1a2e; border: 1px solid #3b3b5a;
                    border-radius: 10px; width: 560px; max-width: 96vw;
                    max-height: 90vh; display: flex; flex-direction: column;
                    color: #e0e0e8; box-shadow: 0 8px 40px rgba(0,0,0,0.6);
                ",
                {render_header(on_cancel.clone())}
                div {
                    style: "
                        padding: 1.25rem 1.5rem; overflow-y: auto; flex: 1;
                        display: flex; flex-direction: column; gap: 1rem;
                    ",
                    if let Some(error) = schema_err.read().as_deref() {
                        {render_error_banner(format!("Failed to load schemas: {error}"))}
                    }
                    {render_type_selector(schema_list, &current_type_id, draft)}
                    {render_static_fields(draft)}
                    if let Some(type_schema) = &maybe_type_schema {
                        {render_schema_fields(type_schema, draft, show_optional)}
                    }
                    if let Some(error) = submit_err.read().as_deref() {
                        {render_error_banner(error.to_string())}
                    }
                }
                {render_footer(submitting(), on_cancel, on_submit)}
            }
        }
    }
}

fn initial_draft(prefill: &Option<TicketSummary>) -> DraftState {
    prefill
        .as_ref()
        .map(DraftState::from_prefill)
        .unwrap_or_else(|| load_draft().unwrap_or_default())
}

fn render_header(on_cancel: EventHandler<MouseEvent>) -> Element {
    rsx! {
        div {
            style: "
                padding: 1.25rem 1.5rem 0.75rem;
                border-bottom: 1px solid #2d2d4a;
                display: flex; align-items: center; justify-content: space-between;
            ",
            h2 {
                style: "margin: 0; font-size: 16px; font-weight: 700;",
                "New Ticket"
            }
            button {
                style: "
                    background: none; border: none; color: #8080a0;
                    cursor: pointer; font-size: 18px; line-height: 1;
                ",
                onclick: move |evt| on_cancel.call(evt),
                "✕"
            }
        }
    }
}

fn render_footer(
    submitting: bool,
    on_cancel: EventHandler<MouseEvent>,
    on_submit: EventHandler<MouseEvent>,
) -> Element {
    let submit_opacity = if submitting { "0.6" } else { "1" };

    rsx! {
        div {
            style: "
                padding: 1rem 1.5rem;
                border-top: 1px solid #2d2d4a;
                display: flex; gap: 0.75rem; justify-content: flex-end;
            ",
            button {
                r#type: "button",
                style: "
                    padding: 8px 18px; border-radius: 6px; border: none;
                    background: #2a2a42; color: #a0a0c0;
                    cursor: pointer; font-size: 14px;
                ",
                disabled: submitting,
                onclick: move |evt| on_cancel.call(evt),
                "Cancel"
            }
            button {
                r#type: "button",
                style: "
                    padding: 8px 18px; border-radius: 6px; border: none;
                    background: #3b82f6; color: white;
                    cursor: pointer; font-size: 14px; font-weight: 600;
                    opacity: {submit_opacity};
                ",
                disabled: submitting,
                onclick: move |evt| on_submit.call(evt),
                if submitting { "Creating…" } else { "Create Ticket" }
            }
        }
    }
}

fn render_error_banner(message: String) -> Element {
    rsx! {
        div {
            style: "
                background: #3b1a1a; border: 1px solid #ef4444;
                border-radius: 6px; padding: 0.5rem 0.75rem;
                font-size: 13px; color: #fca5a5;
            ",
            "{message}"
        }
    }
}
