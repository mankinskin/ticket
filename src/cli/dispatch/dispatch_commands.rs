#[allow(unused_imports)]
use super::*;

pub(in super::super) fn dispatch(
    command: TicketCommandCli,
    index_root_override: Option<&Path>,
    workspace_root_override: Option<&Path>,
    schema_dir_override: Option<&Path>,
    _as_json: bool,
    dry_run: bool,
) -> Result<Value, CliRunError> {
    match command {
        TicketCommandCli::ExportCommandSchema => export_command_schema_payload(),
        TicketCommandCli::Catalog => capability_catalog_payload(),
        TicketCommandCli::Init => {
            if index_root_override.is_none() && workspace_root_override.is_none() {
                return Err(CliRunError::BadRequest(
                    "store initialization requires explicit --workspace <path> or --index-root <path>".to_string(),
                ));
            }
            let normalized_workspace_root = workspace_root_override
                .map(|path| {
                    let selector = path.to_string_lossy();
                    ticket_api::workspace::normalize_explicit_workspace_selector(Some(&selector))
                        .map_err(|error| CliRunError::BadRequest(error.to_string()))
                })
                .transpose()?;
            cmd_init(
                index_root_override,
                normalized_workspace_root
                    .as_deref()
                    .or(workspace_root_override),
                schema_dir_override,
            )
        }
        other => dispatch_store_backed(
            other,
            index_root_override,
            workspace_root_override,
            schema_dir_override,
            dry_run,
        ),
    }
}

pub(super) fn cmd_init(
    index_root_override: Option<&Path>,
    workspace_root_override: Option<&Path>,
    schema_dir_override: Option<&Path>,
) -> Result<Value, CliRunError> {
    let index_root = resolve_index_root(index_root_override, workspace_root_override)?;
    let mut registry = SchemaRegistry::with_builtins();
    if let Some(schema_dir) = schema_dir_override {
        registry.load_dir(schema_dir)?;
    }
    let workspace_root = if index_root_override.is_some() {
        resolve_workspace_root(&index_root, None)
    } else {
        resolve_workspace_root(&index_root, workspace_root_override)
    };
    let store = TicketStore::init_in_workspace_with(&workspace_root, registry)?;
    Ok(json!({
        "command": "init",
        "status": "ok",
        "workspace": store.index_root.display().to_string(),
        "message": "workspace initialized",
    }))
}

pub(super) fn dispatch_store_backed(
    command: TicketCommandCli,
    index_root_override: Option<&Path>,
    workspace_root_override: Option<&Path>,
    schema_dir_override: Option<&Path>,
    dry_run: bool,
) -> Result<Value, CliRunError> {
    if dry_run {
        if let Some(payload) = dry_run_command_payload(&command) {
            return Ok(payload);
        }
    }

    let normalized_workspace_root = if command_mutates_workspace(&command) {
        workspace_root_override
            .map(|path| {
                let selector = path.to_string_lossy();
                ticket_api::workspace::normalize_explicit_workspace_selector(Some(&selector))
                    .map_err(|error| CliRunError::BadRequest(error.to_string()))
            })
            .transpose()?
    } else {
        None
    };
    let workspace_root_override = normalized_workspace_root
        .as_deref()
        .or(workspace_root_override);

    require_explicit_workspace_for_create(&command, index_root_override, workspace_root_override)?;

    let index_root =
        resolve_board_index_root(&command, index_root_override, workspace_root_override)?;
    let workspace_root = resolve_workspace_root(&index_root, workspace_root_override);
    if dry_run && matches!(&command, TicketCommandCli::MigratePartLayout(_)) {
        let TicketCommandCli::MigratePartLayout(args) = command else {
            unreachable!("command variant was checked above");
        };
        return commands::cmd_migrate_part_layout_dry_run(args, &index_root);
    }
    let store_workspace_root =
        if index_root_override.is_some() || matches!(&command, TicketCommandCli::Board(_)) {
            resolve_workspace_root(&index_root, None)
        } else {
            workspace_root.clone()
        };
    let store = open_store(&store_workspace_root, schema_dir_override)?;
    if command_uses_descendant_scan_roots(&command) {
        let reindex = register_descendant_scan_roots(&store, &workspace_root)?;
        if reindex {
            store.scan(true)?;
        }
    }

    dispatch_store_command(command, store, dry_run)
}

pub(super) fn command_mutates_workspace(command: &TicketCommandCli) -> bool {
    matches!(
        command,
        TicketCommandCli::Create(_)
            | TicketCommandCli::Update(_)
            | TicketCommandCli::Repro(_)
            | TicketCommandCli::Delete(_)
            | TicketCommandCli::Scan(_)
            | TicketCommandCli::Claim(_)
            | TicketCommandCli::Unclaim(_)
            | TicketCommandCli::WritePart(_)
            | TicketCommandCli::WriteAmendment(_)
            | TicketCommandCli::UndoPart(_)
            | TicketCommandCli::AddRoot(_)
            | TicketCommandCli::Batch(_)
            | TicketCommandCli::Revert(_)
            | TicketCommandCli::FinalizeMerge(_)
            | TicketCommandCli::Link(_)
            | TicketCommandCli::Unlink(_)
            | TicketCommandCli::PruneDangling(_)
            | TicketCommandCli::Close(_)
            | TicketCommandCli::Cancel(_)
            | TicketCommandCli::Move(_)
            | TicketCommandCli::MigratePartLayout(_)
            | TicketCommandCli::Attach(_)
    )
}

pub(super) fn require_explicit_workspace_for_create(
    command: &TicketCommandCli,
    index_root_override: Option<&Path>,
    workspace_root_override: Option<&Path>,
) -> Result<(), CliRunError> {
    if matches!(
        command,
        TicketCommandCli::Create(_) | TicketCommandCli::Batch(_)
    ) && index_root_override.is_none()
        && workspace_root_override.is_none()
    {
        return Err(CliRunError::BadRequest(
            "entity creation requires explicit --workspace <path> or --index-root <path>"
                .to_string(),
        ));
    }
    if matches!(command, TicketCommandCli::MigratePartLayout(_))
        && index_root_override.is_none()
        && workspace_root_override.is_none()
    {
        return Err(CliRunError::BadRequest(
            "migrate-part-layout requires explicit --workspace <path> or --index-root <path>"
                .to_string(),
        ));
    }
    Ok(())
}

pub(super) fn dispatch_store_command(
    command: TicketCommandCli,
    store: TicketStore,
    dry_run: bool,
) -> Result<Value, CliRunError> {
    match command {
        TicketCommandCli::Create(_)
        | TicketCommandCli::Get(_)
        | TicketCommandCli::Describe(_)
        | TicketCommandCli::Update(_)
        | TicketCommandCli::Repro(_)
        | TicketCommandCli::List(_)
        | TicketCommandCli::Delete(_)
        | TicketCommandCli::Scan(_)
        | TicketCommandCli::Claim(_)
        | TicketCommandCli::Unclaim(_)
        | TicketCommandCli::ListParts(_)
        | TicketCommandCli::GetPart(_)
        | TicketCommandCli::WritePart(_)
        | TicketCommandCli::WriteAmendment(_)
        | TicketCommandCli::UndoPart(_) => dispatch_store_command_core(command, &store),
        TicketCommandCli::Leases
        | TicketCommandCli::Search(_)
        | TicketCommandCli::Query(_)
        | TicketCommandCli::AddRoot(_)
        | TicketCommandCli::Batch(_)
        | TicketCommandCli::History(_)
        | TicketCommandCli::Diff(_)
        | TicketCommandCli::Revert(_)
        | TicketCommandCli::FinalizeMerge(_) => dispatch_store_command_history(command, &store),
        TicketCommandCli::Link(_)
        | TicketCommandCli::Unlink(_)
        | TicketCommandCli::Links(_)
        | TicketCommandCli::PruneDangling(_)
        | TicketCommandCli::Subgraph(_)
        | TicketCommandCli::Topgraph(_)
        | TicketCommandCli::Watch(_)
        | TicketCommandCli::Status(_)
        | TicketCommandCli::ReadyOverview(_)
        | TicketCommandCli::Next(_)
        | TicketCommandCli::Blockers(_)
        | TicketCommandCli::UnblockedBy(_) => dispatch_store_command_graph(command, &store),
        TicketCommandCli::Serve(_)
        | TicketCommandCli::Close(_)
        | TicketCommandCli::Cancel(_)
        | TicketCommandCli::Move(_)
        | TicketCommandCli::MigratePartLayout(_)
        | TicketCommandCli::Attach(_)
        | TicketCommandCli::Assets(_)
        | TicketCommandCli::Transitions(_)
        | TicketCommandCli::Health(_)
        | TicketCommandCli::StoreIndex(_)
        | TicketCommandCli::Audit
        | TicketCommandCli::Fmt(_)
        | TicketCommandCli::Board(_)
        | TicketCommandCli::Workspace(_)
        | TicketCommandCli::ValidateLinks => dispatch_store_command_ops(command, store, dry_run),
        TicketCommandCli::ExportCommandSchema | TicketCommandCli::Init => {
            unreachable!("handled before store dispatch")
        }
        TicketCommandCli::Catalog => {
            unreachable!("handled before store dispatch")
        }
    }
}

pub(super) fn dispatch_store_command_core(
    command: TicketCommandCli,
    store: &TicketStore,
) -> Result<Value, CliRunError> {
    match command {
        TicketCommandCli::Create(args) => commands::cmd_create(args, store),
        TicketCommandCli::Get(args) => commands::cmd_get(args, store),
        TicketCommandCli::Describe(args) => commands::cmd_describe(args, store),
        TicketCommandCli::Update(args) => commands::cmd_update(args, store),
        TicketCommandCli::Repro(args) => commands::cmd_repro(args, store),
        TicketCommandCli::List(args) => commands::cmd_list(args, store),
        TicketCommandCli::Delete(args) => commands::cmd_delete(args, store),
        TicketCommandCli::Scan(args) => commands::cmd_scan(args, store),
        TicketCommandCli::Claim(args) => commands::cmd_claim(args, store),
        TicketCommandCli::Unclaim(args) => commands::cmd_unclaim(args, store),
        TicketCommandCli::ListParts(args) => commands::cmd_list_parts(args, store),
        TicketCommandCli::GetPart(args) => commands::cmd_get_part(args, store),
        TicketCommandCli::WritePart(args) => commands::cmd_write_part(args, store),
        TicketCommandCli::WriteAmendment(args) => commands::cmd_write_amendment(args, store),
        TicketCommandCli::UndoPart(args) => commands::cmd_undo_part(args, store),
        _ => unreachable!("handled in core store dispatch"),
    }
}

pub(super) fn dispatch_store_command_history(
    command: TicketCommandCli,
    store: &TicketStore,
) -> Result<Value, CliRunError> {
    match command {
        TicketCommandCli::Leases => commands::cmd_leases(store),
        TicketCommandCli::Search(args) => commands::cmd_search(args, store),
        TicketCommandCli::Query(args) => commands::cmd_search(args, store),
        TicketCommandCli::AddRoot(args) => commands::cmd_add_root(args, store),
        TicketCommandCli::Batch(args) => batch::cmd_batch(args, store),
        TicketCommandCli::History(args) => commands::cmd_history(args, store),
        TicketCommandCli::Diff(args) => commands::cmd_diff(args, store),
        TicketCommandCli::Revert(args) => commands::cmd_revert(args, store),
        TicketCommandCli::FinalizeMerge(args) => {
            let id = commands::resolve_uuid_prefix(&args.id, store)?;
            Ok(json!({
                "command": "finalize_merge",
                "status": "phase2_stub",
                "id": id,
                "merge_commit": args.merge_commit
            }))
        }
        _ => unreachable!("handled in history store dispatch"),
    }
}

pub(super) fn dispatch_store_command_graph(
    command: TicketCommandCli,
    store: &TicketStore,
) -> Result<Value, CliRunError> {
    match command {
        TicketCommandCli::Link(_)
        | TicketCommandCli::Unlink(_)
        | TicketCommandCli::Links(_)
        | TicketCommandCli::PruneDangling(_)
        | TicketCommandCli::Subgraph(_)
        | TicketCommandCli::Topgraph(_) => dispatch_store_command_graph_edges(command, store),
        TicketCommandCli::Watch(_)
        | TicketCommandCli::Status(_)
        | TicketCommandCli::ReadyOverview(_)
        | TicketCommandCli::Next(_)
        | TicketCommandCli::Blockers(_)
        | TicketCommandCli::UnblockedBy(_) => dispatch_store_command_graph_workflow(command, store),
        _ => unreachable!("handled in graph store dispatch"),
    }
}

pub(super) fn dispatch_store_command_graph_edges(
    command: TicketCommandCli,
    store: &TicketStore,
) -> Result<Value, CliRunError> {
    match command {
        TicketCommandCli::Link(args) => commands::cmd_link(args, store),
        TicketCommandCli::Unlink(args) => commands::cmd_unlink(args, store),
        TicketCommandCli::Links(args) => commands::cmd_links(args, store),
        TicketCommandCli::PruneDangling(args) => commands::cmd_prune_dangling(args, store),
        TicketCommandCli::Subgraph(args) => commands::cmd_subgraph(args, store),
        TicketCommandCli::Topgraph(args) => commands::cmd_topgraph(args, store),
        _ => unreachable!("handled in graph edge dispatch"),
    }
}

pub(super) fn dispatch_store_command_graph_workflow(
    command: TicketCommandCli,
    store: &TicketStore,
) -> Result<Value, CliRunError> {
    match command {
        TicketCommandCli::Watch(args) => commands::cmd_watch(args, store),
        TicketCommandCli::Status(args) => commands::cmd_status(args, store),
        TicketCommandCli::ReadyOverview(args) => commands::cmd_ready_overview(args, store),
        TicketCommandCli::Next(args) => commands::cmd_next(args, store),
        TicketCommandCli::Blockers(args) => commands::cmd_blockers(args, store),
        TicketCommandCli::UnblockedBy(args) => commands::cmd_unblocked_by(args, store),
        _ => unreachable!("handled in graph workflow dispatch"),
    }
}

pub(super) fn dispatch_store_command_ops(
    command: TicketCommandCli,
    store: TicketStore,
    dry_run: bool,
) -> Result<Value, CliRunError> {
    match command {
        TicketCommandCli::Serve(_)
        | TicketCommandCli::Close(_)
        | TicketCommandCli::Cancel(_)
        | TicketCommandCli::Move(_)
        | TicketCommandCli::MigratePartLayout(_)
        | TicketCommandCli::Attach(_)
        | TicketCommandCli::Assets(_)
        | TicketCommandCli::Transitions(_) => {
            dispatch_store_command_ops_runtime(command, store, dry_run)
        }
        TicketCommandCli::Health(_)
        | TicketCommandCli::StoreIndex(_)
        | TicketCommandCli::Audit
        | TicketCommandCli::Fmt(_)
        | TicketCommandCli::Board(_)
        | TicketCommandCli::Workspace(_)
        | TicketCommandCli::ValidateLinks => dispatch_store_command_ops_admin(command, store),
        _ => unreachable!("handled in ops store dispatch"),
    }
}

pub(super) fn dispatch_store_command_ops_runtime(
    command: TicketCommandCli,
    store: TicketStore,
    dry_run: bool,
) -> Result<Value, CliRunError> {
    match command {
        TicketCommandCli::Serve(args) => commands::cmd_serve(args, store),
        TicketCommandCli::Close(args) => commands::cmd_close(args, &store),
        TicketCommandCli::Cancel(args) => commands::cmd_cancel(args, &store),
        TicketCommandCli::Move(args) => commands::cmd_move(args, &store, dry_run),
        TicketCommandCli::MigratePartLayout(args) => {
            commands::cmd_migrate_part_layout(args, &store, dry_run)
        }
        TicketCommandCli::Attach(args) => commands::cmd_attach(args, &store),
        TicketCommandCli::Assets(args) => commands::cmd_assets(args, &store),
        TicketCommandCli::Transitions(args) => commands::cmd_transitions(args, &store),
        _ => unreachable!("handled in ops runtime dispatch"),
    }
}

pub(super) fn dispatch_store_command_ops_admin(
    command: TicketCommandCli,
    store: TicketStore,
) -> Result<Value, CliRunError> {
    match command {
        TicketCommandCli::Health(args) => commands::cmd_health(args, &store),
        TicketCommandCli::StoreIndex(args) => commands::cmd_store_index(args, &store),
        TicketCommandCli::Audit => commands::cmd_audit(&store),
        TicketCommandCli::Fmt(args) => commands::cmd_fmt(args, &store),
        TicketCommandCli::Board(args) => commands::cmd_board(args, &store),
        TicketCommandCli::Workspace(args) => commands::cmd_workspace(args, &store),
        TicketCommandCli::ValidateLinks => {
            let aggregate_root = ticket_api::workspace::working_dir()
                .ok_or_else(|| {
                    CliRunError::BadRequest(
                        "could not determine the CLI process working directory for reference resolution".to_string(),
                    )
                })?;
            commands::cmd_validate_links(&store, &aggregate_root)
        }
        _ => unreachable!("handled in ops admin dispatch"),
    }
}
