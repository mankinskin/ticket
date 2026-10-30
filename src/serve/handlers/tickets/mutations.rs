use std::collections::BTreeMap;

use axum::{
    extract::{Extension, Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Json, Response},
};
use serde_json::Value;
use uuid::Uuid;

use viewer_api::{
    auth::extract_bearer_token,
    error::{ApiError, RequestIdExt},
};

use crate::serve::{
    AppState,
    error::{storage_err, task_join_err},
};

use super::types::{
    CancelTicketBody, CloseTicketBody, CreateTicketBody, DeleteResponse, MoveTicketBody,
    MoveTicketResponse, MutationResponse, MutationWorkspaceParam, ReleaseLeaseBody,
    ReleaseLeaseResponse, RevertTicketBody, TicketDetail, UpdateTicketBody, ticket_ref_for_id,
};

pub(super) fn author_from_headers(headers: &HeaderMap) -> Option<String> {
    extract_bearer_token(headers).map(str::to_string)
}

fn indexed_created_at(
    store: &ticket_api::storage::store::TicketStore,
    id: &Uuid,
) -> chrono::DateTime<chrono::Utc> {
    store
        .get_indexed(id)
        .ok()
        .flatten()
        .map(|ticket| ticket.created_at)
        .unwrap_or_else(chrono::Utc::now)
}

fn current_ticket_response(
    store: &ticket_api::storage::store::TicketStore,
    request_id: &str,
    workspace: &str,
    id: &Uuid,
) -> Response {
    let manifest = match store.get(id) {
        Ok(manifest) => manifest,
        Err(e) => return storage_err(e, request_id),
    };

    Json(MutationResponse {
        request_id: request_id.to_string(),
        active_workspace: workspace.to_string(),
        workspace: workspace.to_string(),
        ticket: TicketDetail {
            id: manifest.id.to_string(),
            ticket_ref: match ticket_ref_for_id(store, workspace, id) {
                Ok(ticket_ref) => ticket_ref,
                Err(e) => return storage_err(e, request_id),
            },
            created_at: indexed_created_at(store, id),
            fields: manifest.extra,
        },
    })
    .into_response()
}

fn move_plan_json(report: &ticket_api::storage::move_planner::MovePreflightReport) -> Value {
    serde_json::json!({
        "supported": report.supported(),
        "source_workspace_root": normalize_display_path(&report.source_workspace_root),
        "target_workspace_root": normalize_display_path(&report.target_workspace_root),
        "source_store_root": normalize_display_path(&report.source_store_root),
        "target_store_root": normalize_display_path(&report.target_store_root),
        "source_ticket_path": normalize_display_path(&report.source_entity_path),
        "destination_ticket_path": normalize_display_path(&report.destination_entity_path),
        "path_reference_files": report.path_reference_files.iter().map(|p| normalize_display_path(p)).collect::<Vec<_>>(),
        "reference_visibility": report.reference_visibility,
        "active_board_entries": report.active_board_entries,
        "historical_board_entries": report.historical_board_entries,
        "active_leases": report.active_leases,
        "blockers": report.blockers,
        "captured_at": report.captured_at,
    })
}

fn move_outcome_json(outcome: &ticket_api::storage::move_execution::MoveExecutionOutcome) -> Value {
    serde_json::json!({
        "resumed": outcome.resumed,
        "rolled_back": outcome.rolled_back,
        "journal": {
            "id": outcome.journal.id,
            "ticket_id": outcome.journal.entity_id,
            "phase": outcome.journal.phase,
            "steps": outcome.journal.steps,
            "rollback_steps": outcome.journal.rollback_steps,
            "failure": outcome.journal.failure,
            "next_recovery_step": outcome.journal.next_recovery_step,
            "rewritten_path_files": outcome.journal.rewritten_path_files,
            "manual_followups": outcome.journal.manual_followups,
            "migrated_board_entries": outcome.journal.migrated_board_entries,
            "created_at": outcome.journal.created_at,
            "updated_at": outcome.journal.updated_at,
        }
    })
}

fn move_recovery_json() -> Value {
    serde_json::json!({
        "resume": "ticket move --resume <journal-uuid>",
        "rollback": "ticket move --rollback <journal-uuid>",
    })
}

fn normalize_workspace_root(value: &std::path::Path) -> std::path::PathBuf {
    std::fs::canonicalize(value).unwrap_or_else(|_| value.to_path_buf())
}

fn normalize_display_path(path: &std::path::Path) -> String {
    ticket_api::workspace::normalize_path_for_display(path)
}

#[path = "mutations/ticket_operations.rs"]
mod ticket_operations;
#[allow(unused_imports)]
pub use ticket_operations::*;

#[path = "mutations/move_operations.rs"]
mod move_operations;
#[allow(unused_imports)]
pub use move_operations::*;

#[path = "mutations/ticket_history.rs"]
mod ticket_history;
#[allow(unused_imports)]
pub use ticket_history::*;
