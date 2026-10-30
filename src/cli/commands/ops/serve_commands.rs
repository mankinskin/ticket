#[allow(unused_imports)]
use super::*;

pub(crate) fn cmd_add_root(args: AddRootArgs, store: &TicketStore) -> Result<Value, CliRunError> {
    use ticket_api::model::filesystem::ScanRoot;
    let path = args.path.canonicalize().unwrap_or(args.path.clone());
    std::fs::create_dir_all(&path).map_err(StorageError::Io)?;
    store.add_scan_root(ScanRoot {
        path: path.clone(),
        label: args.label.clone(),
    })?;
    Ok(json!({
        "command": "add_root",
        "status": "ok",
        "path": path,
        "label": args.label,
    }))
}

pub(crate) fn cmd_serve(args: ServeCliArgs, store: TicketStore) -> Result<Value, CliRunError> {
    #[cfg(feature = "http")]
    {
        use crate::serve::{ServeConfig, WorkspaceRegistry, serve};

        let registry = WorkspaceRegistry::single_opened(std::sync::Arc::new(store));

        let config = ServeConfig {
            host: args.host,
            port: args.port,
        };

        let rt = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .map_err(|e| CliRunError::BadRequest(format!("failed to start tokio runtime: {e}")))?;

        rt.block_on(async {
            serve(config, registry)
                .await
                .map_err(|e| CliRunError::BadRequest(e.to_string()))
        })?;

        Err(CliRunError::BadRequest("server exited unexpectedly".into()))
    }
    #[cfg(not(feature = "http"))]
    {
        let _ = (args, store);
        Err(CliRunError::BadRequest(
            "ticket was built without the http feature; enable --features http to serve".into(),
        ))
    }
}

pub(crate) fn cmd_watch(args: WatchArgs, store: &TicketStore) -> Result<Value, CliRunError> {
    use ticket_api::watcher::reconciler::{run_watch_loop, start_watcher};
    eprintln!(
        "Starting filesystem watcher (debounce={}ms). Press Ctrl+C to stop.",
        args.debounce_ms
    );
    let handle = start_watcher(store).map_err(CliRunError::Storage)?;
    run_watch_loop(&handle, store, args.debounce_ms);
    Ok(json!({ "command": "watch", "status": "stopped" }))
}
