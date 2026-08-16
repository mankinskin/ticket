use std::collections::BTreeMap;

use dioxus::prelude::*;

use crate::{
    api::{
        HttpTicketBackend,
        TicketBackend,
    },
    types::{
        BatchCommand,
        BatchRequest,
    },
};

use super::model::{
    BulkOp,
    CommandResult,
};

pub(crate) fn toggle_pending_op(
    mut pending_op: Signal<Option<BulkOp>>,
    mut global_error: Signal<Option<String>>,
    op: BulkOp,
) {
    let next = if *pending_op.read() == Some(op.clone()) {
        None
    } else {
        Some(op)
    };
    pending_op.set(next);
    global_error.set(None);
}

pub(crate) fn apply_batch_operation(
    workspace: String,
    ids: Vec<String>,
    op: BulkOp,
    mut running: Signal<bool>,
    mut results: Signal<Vec<CommandResult>>,
    mut pending_op: Signal<Option<BulkOp>>,
    mut global_error: Signal<Option<String>>,
) {
    running.set(true);
    global_error.set(None);

    let request = BatchRequest {
        workspace,
        commands: build_commands(&ids, &op),
    };

    spawn(async move {
        let backend = HttpTicketBackend::new(None);
        match backend.batch_tickets(&request).await {
            Ok(response) => {
                results.set(build_results(&request.commands, &response.status));
                running.set(false);
                pending_op.set(None);
            },
            Err(error) => {
                global_error.set(Some(error));
                running.set(false);
            },
        }
    });
}

fn build_commands(
    ids: &[String],
    op: &BulkOp,
) -> Vec<BatchCommand> {
    ids.iter()
        .map(|id| match op {
            BulkOp::SetState(state) => BatchCommand::Update {
                id: id.clone(),
                state: Some(state.clone()),
                from_state: None,
                fields: Default::default(),
            },
            BulkOp::Close => BatchCommand::Close {
                id: id.clone(),
                target_state: None,
            },
            BulkOp::Cancel => BatchCommand::Cancel {
                id: id.clone(),
                reason: None,
            },
            BulkOp::SetPriority(priority) => BatchCommand::Update {
                id: id.clone(),
                state: None,
                from_state: None,
                fields: priority_field(priority),
            },
        })
        .collect()
}

fn build_results(
    commands: &[BatchCommand],
    status: &str,
) -> Vec<CommandResult> {
    commands
        .iter()
        .map(|command| CommandResult {
            id: command_id(command),
            ok: true,
            message: format!("{status} — ok"),
        })
        .collect()
}

fn command_id(command: &BatchCommand) -> String {
    match command {
        BatchCommand::Update { id, .. }
        | BatchCommand::Close { id, .. }
        | BatchCommand::Cancel { id, .. } => id.clone(),
    }
}

fn priority_field(priority: &str) -> BTreeMap<String, serde_json::Value> {
    let mut fields = BTreeMap::new();
    fields.insert(
        "priority".to_string(),
        serde_json::Value::String(priority.to_string()),
    );
    fields
}
