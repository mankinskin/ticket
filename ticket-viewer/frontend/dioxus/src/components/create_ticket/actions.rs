use std::collections::BTreeMap;

use dioxus::prelude::*;
use dioxus_router::Navigator;

use crate::{
    api::{
        HttpTicketBackend,
        TicketBackend,
    },
    routes::Route,
    types::{
        CreateTicketRequest,
        SchemaListResponse,
        TypeSchema,
    },
};

use super::draft::{
    clear_draft,
    DraftState,
};

pub(crate) fn load_schemas(
    backend: HttpTicketBackend,
    workspace: String,
    mut schema_list: Signal<Option<SchemaListResponse>>,
    mut schema_err: Signal<Option<String>>,
) {
    use_effect(move || {
        let backend = backend.clone();
        let workspace = workspace.clone();
        spawn(async move {
            match backend.list_schemas(&workspace).await {
                Ok(response) => schema_list.set(Some(response)),
                Err(error) => schema_err.set(Some(error)),
            }
        });
    });
}

pub(crate) fn selected_type_schema(
    schema_list: Option<&SchemaListResponse>,
    current_type_id: &str,
) -> Option<TypeSchema> {
    schema_list.and_then(|schema_list| {
        schema_list
            .types
            .iter()
            .find(|schema| schema.type_id == current_type_id)
            .cloned()
    })
}

pub(crate) fn validate_draft(draft: &DraftState) -> Option<String> {
    if draft.title.trim().is_empty() {
        return Some("Title is required.".to_string());
    }
    if draft.type_id.is_empty() {
        return Some("Please select a ticket type.".to_string());
    }
    None
}

pub(crate) fn build_request(draft: &DraftState) -> CreateTicketRequest {
    let mut fields: BTreeMap<String, serde_json::Value> = BTreeMap::new();
    if !draft.priority.is_empty() {
        fields.insert(
            "priority".to_string(),
            serde_json::Value::String(draft.priority.clone()),
        );
    }
    if !draft.component.is_empty() {
        fields.insert(
            "component".to_string(),
            serde_json::Value::String(draft.component.clone()),
        );
    }
    for (key, value) in &draft.extra {
        if !value.is_empty() {
            fields
                .insert(key.clone(), serde_json::Value::String(value.clone()));
        }
    }

    CreateTicketRequest {
        type_id: draft.type_id.clone(),
        title: (!draft.title.is_empty()).then(|| draft.title.clone()),
        description: (!draft.description.is_empty())
            .then(|| draft.description.clone()),
        fields: (!fields.is_empty()).then_some(fields),
    }
}

pub(crate) fn spawn_create_ticket(
    backend: HttpTicketBackend,
    workspace: String,
    request: CreateTicketRequest,
    nav: Navigator,
    mut submitting: Signal<bool>,
    mut submit_err: Signal<Option<String>>,
) {
    spawn(async move {
        match backend.create_ticket(&workspace, &request).await {
            Ok(response) => {
                clear_draft();
                let ticket_ref =
                    response.ticket.resolved_ticket_ref(&workspace);
                nav.push(Route::TicketDetailPage {
                    workspace: ticket_ref.workspace,
                    id: ticket_ref.id,
                });
            },
            Err(error) => {
                submit_err.set(Some(error));
                submitting.set(false);
            },
        }
    });
}
