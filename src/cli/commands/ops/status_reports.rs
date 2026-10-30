#[allow(unused_imports)]
use super::*;

pub(crate) fn cmd_status(args: StatusArgs, store: &TicketStore) -> Result<Value, CliRunError> {
    status::run(args, store)
}

pub(crate) fn cmd_ready_overview(
    args: ReadyOverviewArgs,
    store: &TicketStore,
) -> Result<Value, CliRunError> {
    let status_payload = cmd_status(
        StatusArgs {
            filter: args.filter.clone(),
            show_blocked: true,
        },
        store,
    )?;

    let scope = args
        .scope
        .unwrap_or_else(|| "ready tickets currently open in the active index".to_string());

    Ok(json!({
        "command": "ready_overview",
        "status": "ok",
        "date": Utc::now().format("%Y-%m-%d").to_string(),
        "scope": scope,
        "summary": status_payload["summary"],
        "planned": status_payload["planned"],
        "ready_count": status_payload["summary"]["planned"],
    }))
}

pub(crate) fn cmd_next(args: NextArgs, store: &TicketStore) -> Result<Value, CliRunError> {
    next::run(args, store)
}

pub(crate) fn cmd_blockers(args: BlockersArgs, store: &TicketStore) -> Result<Value, CliRunError> {
    next::run_blockers(args, store)
}

pub(crate) fn cmd_unblocked_by(
    args: UnblockedByArgs,
    store: &TicketStore,
) -> Result<Value, CliRunError> {
    next::run_unblocked_by(args, store)
}

pub(crate) fn cmd_health(args: HealthArgs, store: &TicketStore) -> Result<Value, CliRunError> {
    health::run(args, store)
}
