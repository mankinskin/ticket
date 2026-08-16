use dioxus::prelude::*;

use crate::{
    api::HttpTicketBackend,
    types::TypeSchema,
};

use super::{
    actions::{
        discard_conflict,
        keep_conflict,
        save_bool_field,
        save_field,
        submit_ticket_feedback,
        transition_state,
        undo_transition,
        use_conflict_sse,
        use_ticket_detail_data,
        SseHandle,
    },
    model::{
        missing_required_states,
        terminal_states,
        valid_next_states,
        ConflictState,
    },
    ui::{
        render_ticket_detail,
        TicketDetailHandlers,
        TicketDetailViewState,
    },
};

#[component]
pub fn TicketDetail(
    workspace: String,
    id: String,
) -> Element {
    let backend = HttpTicketBackend::new(None);
    #[cfg_attr(not(target_arch = "wasm32"), allow(unused_mut))]
    let mut ticket_fields: Signal<serde_json::Value> =
        use_signal(|| serde_json::Value::Null);
    #[cfg_attr(not(target_arch = "wasm32"), allow(unused_mut))]
    let mut load_error: Signal<Option<String>> = use_signal(|| None);
    #[cfg_attr(not(target_arch = "wasm32"), allow(unused_mut))]
    let mut editing_field: Signal<Option<String>> = use_signal(|| None);
    #[cfg_attr(not(target_arch = "wasm32"), allow(unused_mut))]
    let mut draft_value: Signal<String> = use_signal(String::new);
    #[cfg_attr(not(target_arch = "wasm32"), allow(unused_mut))]
    let mut save_pending: Signal<bool> = use_signal(|| false);
    #[cfg_attr(not(target_arch = "wasm32"), allow(unused_mut))]
    let mut save_error: Signal<Option<String>> = use_signal(|| None);
    #[cfg_attr(not(target_arch = "wasm32"), allow(unused_mut))]
    let mut schema: Signal<Option<TypeSchema>> = use_signal(|| None);
    #[cfg_attr(not(target_arch = "wasm32"), allow(unused_mut))]
    let mut visited_states: Signal<Vec<String>> = use_signal(Vec::new);
    #[cfg_attr(not(target_arch = "wasm32"), allow(unused_mut))]
    let mut transition_pending: Signal<bool> = use_signal(|| false);
    #[cfg_attr(not(target_arch = "wasm32"), allow(unused_mut))]
    let mut transition_error: Signal<Option<String>> = use_signal(|| None);
    #[cfg_attr(not(target_arch = "wasm32"), allow(unused_mut))]
    let mut conflict: Signal<Option<ConflictState>> = use_signal(|| None);
    #[cfg_attr(not(target_arch = "wasm32"), allow(unused_mut))]
    let mut sse_handle: Signal<SseHandle> = use_signal(|| None);
    #[cfg_attr(not(target_arch = "wasm32"), allow(unused_mut))]
    let mut feedback_pending: Signal<bool> = use_signal(|| false);
    #[cfg_attr(not(target_arch = "wasm32"), allow(unused_mut))]
    let mut feedback_error: Signal<Option<String>> = use_signal(|| None);

    use_ticket_detail_data(
        backend.clone(),
        workspace.clone(),
        id.clone(),
        ticket_fields,
        load_error,
        schema,
        visited_states,
    );
    use_conflict_sse(
        workspace.clone(),
        id.clone(),
        editing_field,
        draft_value,
        ticket_fields,
        conflict,
        sse_handle,
    );

    let fields = ticket_fields();
    let current_state = fields
        .get("state")
        .and_then(|value| value.as_str())
        .unwrap_or("")
        .to_string();
    let current_schema = schema();
    let visited = visited_states();

    let view = TicketDetailViewState {
        fields,
        load_error: load_error(),
        save_error: save_error(),
        current_editing: editing_field(),
        current_draft: draft_value(),
        save_pending: save_pending(),
        current_state: current_state.clone(),
        valid_next_states: valid_next_states(
            current_schema.as_ref(),
            &current_state,
        ),
        missing_required_states: missing_required_states(
            current_schema.as_ref(),
            &visited,
        ),
        terminal_states: terminal_states(current_schema.as_ref()),
        transition_pending: transition_pending(),
        transition_error: transition_error(),
        conflict: conflict(),
        feedback_pending: feedback_pending(),
        feedback_error: feedback_error(),
    };

    let handlers = TicketDetailHandlers {
        on_discard: EventHandler::new(move |_| {
            discard_conflict(ticket_fields, conflict, editing_field);
        }),
        on_keep: EventHandler::new(move |_| {
            keep_conflict(conflict, editing_field);
        }),
        on_start_edit: EventHandler::new(move |field_key: String| {
            editing_field.set(Some(field_key));
        }),
        on_draft_change: EventHandler::new(move |value: String| {
            draft_value.set(value);
        }),
        on_save_field: {
            let backend = backend.clone();
            let workspace = workspace.clone();
            let id = id.clone();
            EventHandler::new(move |(field_key, value): (String, String)| {
                save_field(
                    backend.clone(),
                    workspace.clone(),
                    id.clone(),
                    ticket_fields,
                    editing_field,
                    save_pending,
                    save_error,
                    field_key,
                    value,
                );
            })
        },
        on_save_bool: {
            let backend = backend.clone();
            let workspace = workspace.clone();
            let id = id.clone();
            EventHandler::new(move |(field_key, checked): (String, bool)| {
                save_bool_field(
                    backend.clone(),
                    workspace.clone(),
                    id.clone(),
                    ticket_fields,
                    save_pending,
                    save_error,
                    field_key,
                    checked,
                );
            })
        },
        on_cancel_edit: EventHandler::new(move |_| {
            editing_field.set(None);
        }),
        on_transition: {
            let backend = backend.clone();
            let workspace = workspace.clone();
            let id = id.clone();
            EventHandler::new(move |new_state: String| {
                transition_state(
                    backend.clone(),
                    workspace.clone(),
                    id.clone(),
                    ticket_fields,
                    visited_states,
                    transition_pending,
                    transition_error,
                    new_state,
                );
            })
        },
        on_undo: {
            let backend = backend.clone();
            let workspace = workspace.clone();
            let id = id.clone();
            EventHandler::new(move |_| {
                undo_transition(
                    backend.clone(),
                    workspace.clone(),
                    id.clone(),
                    ticket_fields,
                    visited_states,
                    transition_pending,
                    transition_error,
                );
            })
        },
        on_feedback: {
            let backend = backend.clone();
            let workspace = workspace.clone();
            let id = id.clone();
            EventHandler::new(move |rating: String| {
                submit_ticket_feedback(
                    backend.clone(),
                    workspace.clone(),
                    id.clone(),
                    feedback_pending,
                    feedback_error,
                    rating,
                );
            })
        },
    };

    let _ = sse_handle;

    render_ticket_detail(view, handlers)
}
