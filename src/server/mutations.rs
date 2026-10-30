use serde_json::Value;
use ticket_api::model::edge::EdgeRecord;
use uuid::Uuid;

use super::{types::*, *};

#[path = "mutations/helpers.rs"]
mod mutations_helpers;
use mutations_helpers::{
    detail_from_manifest, indexed_ticket_path, move_outcome_json, move_plan_json,
    move_recovery_json, normalize_workspace_root, parse_field_patch, resolve_edge_for_remove,
};

impl TicketServer {}

#[cfg(test)]
#[path = "mutations/tests.rs"]
mod tests;

#[path = "mutations/ticket_crud.rs"]
mod ticket_crud;

#[path = "mutations/edge_operations.rs"]
mod edge_operations;

#[path = "mutations/move_operations.rs"]
mod move_operations;

#[path = "mutations/undo.rs"]
mod undo;
