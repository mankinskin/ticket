#[allow(unused_imports)]
use super::*;

pub(super) fn dry_run_command_payload(command: &TicketCommandCli) -> Option<Value> {
    dry_run_payload_core(command)
        .or_else(|| dry_run_payload_history(command))
        .or_else(|| dry_run_payload_runtime(command))
}

pub(super) fn dry_run_payload_core(command: &TicketCommandCli) -> Option<Value> {
    match command {
        TicketCommandCli::Init => Some(dry_run_payload("init", "initialize ticket workspace")),
        TicketCommandCli::Create(_) => Some(dry_run_payload("create", "create ticket")),
        TicketCommandCli::Update(_) => Some(dry_run_payload("update", "update ticket")),
        TicketCommandCli::Repro(_) => Some(dry_run_payload("repro", "record repro metadata")),
        TicketCommandCli::Delete(_) => Some(dry_run_payload("delete", "permanently delete ticket")),
        TicketCommandCli::Scan(_) => Some(dry_run_payload("scan", "scan/reindex ticket roots")),
        TicketCommandCli::Claim(_) => Some(dry_run_payload("claim", "claim ticket lease")),
        TicketCommandCli::Unclaim(_) => Some(dry_run_payload("unclaim", "release ticket lease")),
        TicketCommandCli::AddRoot(_) => Some(dry_run_payload("add_root", "register scan root")),
        TicketCommandCli::Batch(_) => Some(dry_run_payload("batch", "execute CLI batch commands")),
        TicketCommandCli::WritePart(_) => {
            Some(dry_run_payload("write-part", "write ticket content part"))
        }
        TicketCommandCli::WriteAmendment(_) => Some(dry_run_payload(
            "write-amendment",
            "write amendment part superseding another part",
        )),
        TicketCommandCli::UndoPart(_) => {
            Some(dry_run_payload("undo-part", "restore prior part content"))
        }
        _ => None,
    }
}

pub(super) fn dry_run_payload_history(command: &TicketCommandCli) -> Option<Value> {
    match command {
        TicketCommandCli::Revert(_) => Some(dry_run_payload("revert", "apply historical snapshot")),
        TicketCommandCli::FinalizeMerge(_) => {
            Some(dry_run_payload("finalize_merge", "record merge metadata"))
        }
        TicketCommandCli::Link(_) => Some(dry_run_payload("link", "add directed edge")),
        TicketCommandCli::Unlink(_) => Some(dry_run_payload("unlink", "remove directed edge")),
        TicketCommandCli::PruneDangling(_) => {
            Some(dry_run_payload("prune-dangling", "remove dangling edges"))
        }
        TicketCommandCli::Close(_) => Some(dry_run_payload("close", "fast-forward ticket state")),
        TicketCommandCli::Cancel(_) => Some(dry_run_payload(
            "cancel",
            "cancel ticket via state transition",
        )),
        TicketCommandCli::Attach(_) => Some(dry_run_payload("attach", "attach asset to ticket")),
        _ => None,
    }
}

pub(super) fn dry_run_payload_runtime(command: &TicketCommandCli) -> Option<Value> {
    match command {
        TicketCommandCli::Watch(_) => {
            Some(dry_run_payload("watch", "start watcher/reconcile loop"))
        }
        TicketCommandCli::Serve(_) => Some(dry_run_payload("serve", "start HTTP server")),
        TicketCommandCli::StoreIndex(_) => Some(dry_run_payload(
            "store-index",
            "generate/check ticket catalog",
        )),
        TicketCommandCli::Fmt(_) => Some(dry_run_payload("fmt", "reformat ticket.toml files")),
        TicketCommandCli::Board(_) => Some(dry_run_payload("board", "board state mutation")),
        TicketCommandCli::Workspace(_) => {
            Some(dry_run_payload("workspace", "workspace policy mutation"))
        }
        _ => None,
    }
}

pub(super) fn dry_run_payload(command: &str, action: &str) -> Value {
    json!({
        "command": command,
        "status": "ok",
        "dry_run": true,
        "would_execute": action,
    })
}

pub(super) fn export_command_schema_payload() -> Result<Value, CliRunError> {
    let schema_json = export_command_schema_json()?;
    let schema: Value = serde_json::from_str(&schema_json)?;
    Ok(json!({
        "command": "export_command_schema",
        "status": "ok",
        "schema": schema,
        "known_commands": export_command_schema().commands,
    }))
}

pub(super) fn capability_catalog_payload() -> Result<Value, CliRunError> {
    let mut payload = ticket_api::contracts::capability_catalog::capability_catalog();
    if let Value::Object(map) = &mut payload {
        map.insert("command".to_string(), json!("catalog"));
        map.insert("status".to_string(), json!("ok"));
    }
    Ok(payload)
}
