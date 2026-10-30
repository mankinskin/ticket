#[allow(unused_imports)]
use super::*;

pub(crate) fn cmd_scan(args: ScanArgs, store: &TicketStore) -> Result<Value, CliRunError> {
    let reindex = args.reindex || args.force;
    let report = store.scan(reindex)?;
    let diags: Vec<Value> = report
        .diagnostics
        .iter()
        .map(|d| json!({ "path": d.path, "reason": d.reason }))
        .collect();
    let mut result = json!({
        "command": "scan",
        "status": "ok",
        "integrated": report.integrated,
        "diagnostics": diags,
    });
    if args.force {
        result["force"] = json!(true);
        result["reconciled"] = json!(report.integrated);
        result["pruned"] = json!(report.pruned);
    }
    Ok(result)
}

pub(crate) fn cmd_attach(args: AttachArgs, store: &TicketStore) -> Result<Value, CliRunError> {
    let id = super::super::resolve_uuid_prefix(&args.id, store)?;
    let dest = store.attach(&id, &args.path, args.asset_name.as_deref())?;
    let title = store
        .get(&id)
        .ok()
        .and_then(|m| {
            m.extra
                .get("title")
                .and_then(Value::as_str)
                .map(String::from)
        })
        .unwrap_or_else(|| "-".to_string());
    Ok(json!({
        "command": "attach",
        "status": "ok",
        "id": id,
        "title": title,
        "asset_path": dest.display().to_string(),
    }))
}

pub(crate) fn cmd_assets(args: IdArgs, store: &TicketStore) -> Result<Value, CliRunError> {
    let id = super::super::resolve_uuid_prefix(&args.id, store)?;
    let names = store.list_assets(&id)?;
    Ok(json!({
        "command": "assets",
        "status": "ok",
        "id": id,
        "count": names.len(),
        "assets": names,
    }))
}
