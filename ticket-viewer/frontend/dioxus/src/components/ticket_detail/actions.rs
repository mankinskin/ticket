use dioxus::prelude::*;
use gloo_events::EventListener;
use percent_encoding::{
    utf8_percent_encode,
    NON_ALPHANUMERIC,
};
use wasm_bindgen::JsCast as _;

use crate::{
    api::{
        HttpTicketBackend,
        TicketBackend,
    },
    types::{
        TicketPatch,
        TypeSchema,
    },
};

use super::model::{
    collect_visited_states,
    field_str,
    ConflictState,
};

pub(super) struct SseClose(pub web_sys::EventSource);

impl Drop for SseClose {
    fn drop(&mut self) {
        self.0.close();
    }
}

pub(super) type SseHandle = Option<(SseClose, EventListener)>;

fn sse_url(workspace: &str) -> String {
    let encoded_workspace =
        utf8_percent_encode(workspace, NON_ALPHANUMERIC).to_string();
    format!("/api/stream?workspace={encoded_workspace}")
}

fn parse_upsert_event(event: &web_sys::Event) -> Option<serde_json::Value> {
    let message = event.dyn_ref::<web_sys::MessageEvent>()?;
    let data = message.data().as_string()?;
    serde_json::from_str(&data).ok()
}

fn matches_ticket(
    parsed: &serde_json::Value,
    ticket_id: &str,
) -> bool {
    let event_id = parsed
        .get("id")
        .and_then(|value| value.as_str())
        .unwrap_or("");
    event_id.starts_with(&ticket_id[..8]) || ticket_id.starts_with(event_id)
}

fn conflict_from_server(
    editing_field: Signal<Option<String>>,
    draft_value: Signal<String>,
    server_fields: &serde_json::Value,
) -> Option<ConflictState> {
    let field_key = editing_field()?;
    let server_value = field_str(server_fields, &field_key);
    let my_draft = draft_value();
    if server_value == my_draft {
        None
    } else {
        Some((field_key, server_value, my_draft))
    }
}

pub(super) fn use_ticket_detail_data(
    backend: HttpTicketBackend,
    workspace: String,
    ticket_id: String,
    mut ticket_fields: Signal<serde_json::Value>,
    mut load_error: Signal<Option<String>>,
    mut schema: Signal<Option<TypeSchema>>,
    mut visited_states: Signal<Vec<String>>,
) {
    use_effect(move || {
        let backend = backend.clone();
        let workspace = workspace.clone();
        let ticket_id = ticket_id.clone();

        spawn(async move {
            match backend.get_ticket(&workspace, &ticket_id).await {
                Ok(response) => {
                    let fields = response.ticket.fields.clone();
                    let type_id = fields
                        .get("type")
                        .and_then(|value| value.as_str())
                        .unwrap_or("")
                        .to_string();

                    ticket_fields.set(fields);
                    if !type_id.is_empty() {
                        if let Ok(schema_response) = backend
                            .get_schema_by_type(&workspace, &type_id)
                            .await
                        {
                            schema.set(Some(schema_response.schema));
                        }
                    }
                },
                Err(message) => load_error.set(Some(message)),
            }

            if let Ok(history_response) =
                backend.get_ticket_history(&workspace, &ticket_id).await
            {
                visited_states
                    .set(collect_visited_states(&history_response.entries));
            }
        });
    });
}

pub(super) fn use_conflict_sse(
    workspace: String,
    ticket_id: String,
    editing_field: Signal<Option<String>>,
    draft_value: Signal<String>,
    mut ticket_fields: Signal<serde_json::Value>,
    mut conflict: Signal<Option<ConflictState>>,
    mut sse_handle: Signal<SseHandle>,
) {
    use_effect(move || {
        let url = sse_url(&workspace);

        if let Ok(event_source) = web_sys::EventSource::new(&url) {
            let ticket_id = ticket_id.clone();
            let listener = EventListener::new(
                &event_source,
                "ticket.upsert",
                move |event| {
                    let Some(parsed) = parse_upsert_event(event) else {
                        return;
                    };
                    if !matches_ticket(&parsed, &ticket_id) {
                        return;
                    }

                    let server_fields = parsed
                        .get("fields")
                        .cloned()
                        .unwrap_or(serde_json::Value::Null);
                    if let Some(conflict_state) = conflict_from_server(
                        editing_field,
                        draft_value,
                        &server_fields,
                    ) {
                        conflict.set(Some(conflict_state));
                        return;
                    }

                    if editing_field().is_none() {
                        ticket_fields.set(server_fields);
                    }
                },
            );

            sse_handle.set(Some((SseClose(event_source), listener)));
        }
    });
}

pub(super) fn save_field(
    backend: HttpTicketBackend,
    workspace: String,
    ticket_id: String,
    mut ticket_fields: Signal<serde_json::Value>,
    mut editing_field: Signal<Option<String>>,
    mut save_pending: Signal<bool>,
    mut save_error: Signal<Option<String>>,
    field_key: String,
    new_value: String,
) {
    ticket_fields.with_mut(|fields| {
        if let serde_json::Value::Object(ref mut map) = fields {
            map.insert(
                field_key.clone(),
                serde_json::Value::String(new_value.clone()),
            );
        }
    });
    editing_field.set(None);
    save_pending.set(true);
    save_error.set(None);

    spawn(async move {
        let patch = TicketPatch {
            workspace: workspace.clone(),
            state: None,
            title: if field_key == "title" {
                Some(new_value.clone())
            } else {
                None
            },
            fields: if field_key != "title" {
                let mut map = serde_json::Map::new();
                map.insert(field_key, serde_json::Value::String(new_value));
                Some(serde_json::Value::Object(map))
            } else {
                None
            },
        };

        match backend.patch_ticket(&workspace, &ticket_id, &patch).await {
            Ok(response) => {
                ticket_fields.set(response.ticket.fields);
                save_pending.set(false);
            },
            Err(message) => {
                save_pending.set(false);
                save_error.set(Some(message));
                if let Ok(response) =
                    backend.get_ticket(&workspace, &ticket_id).await
                {
                    ticket_fields.set(response.ticket.fields);
                }
            },
        }
    });
}

pub(super) fn save_bool_field(
    backend: HttpTicketBackend,
    workspace: String,
    ticket_id: String,
    mut ticket_fields: Signal<serde_json::Value>,
    mut save_pending: Signal<bool>,
    mut save_error: Signal<Option<String>>,
    field_key: String,
    checked: bool,
) {
    ticket_fields.with_mut(|fields| {
        if let serde_json::Value::Object(ref mut map) = fields {
            map.insert(field_key.clone(), serde_json::Value::Bool(checked));
        }
    });
    save_pending.set(true);
    save_error.set(None);

    spawn(async move {
        let mut field_map = serde_json::Map::new();
        field_map.insert(field_key.clone(), serde_json::Value::Bool(checked));
        let patch = TicketPatch {
            workspace: workspace.clone(),
            state: None,
            title: None,
            fields: Some(serde_json::Value::Object(field_map)),
        };

        match backend.patch_ticket(&workspace, &ticket_id, &patch).await {
            Ok(response) => {
                ticket_fields.set(response.ticket.fields);
                save_pending.set(false);
            },
            Err(message) => {
                save_pending.set(false);
                save_error.set(Some(message));
                if let Ok(response) =
                    backend.get_ticket(&workspace, &ticket_id).await
                {
                    ticket_fields.set(response.ticket.fields);
                }
            },
        }
    });
}

pub(super) fn transition_state(
    backend: HttpTicketBackend,
    workspace: String,
    ticket_id: String,
    mut ticket_fields: Signal<serde_json::Value>,
    mut visited_states: Signal<Vec<String>>,
    mut transition_pending: Signal<bool>,
    mut transition_error: Signal<Option<String>>,
    new_state: String,
) {
    let previous_state = ticket_fields()
        .get("state")
        .and_then(|value| value.as_str())
        .unwrap_or("")
        .to_string();

    ticket_fields.with_mut(|fields| {
        if let serde_json::Value::Object(ref mut map) = fields {
            map.insert(
                "state".to_string(),
                serde_json::Value::String(new_state.clone()),
            );
        }
    });
    transition_pending.set(true);
    transition_error.set(None);

    spawn(async move {
        let patch = TicketPatch {
            workspace: workspace.clone(),
            state: Some(new_state.clone()),
            title: None,
            fields: None,
        };

        match backend.patch_ticket(&workspace, &ticket_id, &patch).await {
            Ok(response) => {
                let fields = response.ticket.fields.clone();
                let reached = fields
                    .get("state")
                    .and_then(|value| value.as_str())
                    .unwrap_or("")
                    .to_string();
                ticket_fields.set(fields);
                transition_pending.set(false);
                if !reached.is_empty() {
                    visited_states.with_mut(|states| {
                        if !states.contains(&reached) {
                            states.push(reached);
                        }
                    });
                }
            },
            Err(message) => {
                ticket_fields.with_mut(|fields| {
                    if let serde_json::Value::Object(ref mut map) = fields {
                        map.insert(
                            "state".to_string(),
                            serde_json::Value::String(previous_state.clone()),
                        );
                    }
                });
                transition_pending.set(false);
                transition_error.set(Some(message));
            },
        }
    });
}

pub(super) fn undo_transition(
    backend: HttpTicketBackend,
    workspace: String,
    ticket_id: String,
    mut ticket_fields: Signal<serde_json::Value>,
    mut visited_states: Signal<Vec<String>>,
    mut transition_pending: Signal<bool>,
    mut transition_error: Signal<Option<String>>,
) {
    transition_pending.set(true);
    transition_error.set(None);

    spawn(async move {
        match backend.undo_ticket(&workspace, &ticket_id).await {
            Ok(response) => {
                ticket_fields.set(response.ticket.fields);
                transition_pending.set(false);
                if let Ok(history_response) =
                    backend.get_ticket_history(&workspace, &ticket_id).await
                {
                    visited_states
                        .set(collect_visited_states(&history_response.entries));
                }
            },
            Err(message) => {
                transition_pending.set(false);
                transition_error.set(Some(message));
            },
        }
    });
}

pub(super) fn discard_conflict(
    mut ticket_fields: Signal<serde_json::Value>,
    mut conflict: Signal<Option<ConflictState>>,
    mut editing_field: Signal<Option<String>>,
) {
    if let Some((field_key, server_value, _)) = conflict() {
        ticket_fields.with_mut(|fields| {
            if let serde_json::Value::Object(ref mut map) = fields {
                map.insert(
                    field_key.clone(),
                    serde_json::Value::String(server_value.clone()),
                );
            }
        });
    }
    conflict.set(None);
    editing_field.set(None);
}

pub(super) fn keep_conflict(
    mut conflict: Signal<Option<ConflictState>>,
    mut editing_field: Signal<Option<String>>,
) {
    let field_key = conflict().map(|(key, _, _)| key);
    conflict.set(None);
    editing_field.set(field_key);
}

pub(super) fn submit_ticket_feedback(
    backend: HttpTicketBackend,
    workspace: String,
    ticket_id: String,
    mut feedback_pending: Signal<bool>,
    mut feedback_error: Signal<Option<String>>,
    rating: String,
) {
    feedback_pending.set(true);
    feedback_error.set(None);

    spawn(async move {
        let workspace_slug = {
            let trimmed = workspace.trim();
            if trimmed.is_empty() || trimmed.contains('/') || trimmed.contains('\\') {
                "default".to_string()
            } else {
                trimmed.to_string()
            }
        };
        let target = format!("ce://{workspace_slug}/ticket/{ticket_id}");
        let note = Some("Ticket detail feedback submitted from ticket-viewer.");

        match backend
            .ingest_feedback(
                &workspace,
                &target,
                &rating,
                note,
                "frontend",
            )
            .await
        {
            Ok(()) => {
                feedback_pending.set(false);
            },
            Err(message) => {
                feedback_pending.set(false);
                feedback_error.set(Some(message));
            },
        }
    });
}
