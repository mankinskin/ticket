//! Ticket Viewer — single-process HTTP server that serves the SPA frontend
//! and the ticket REST/SSE API together.
//!
//! The viewer imports `ticket-http` as a library and mounts its router
//! directly, eliminating the need for a separate backend process or proxy.
//!
//! # Usage
//!
//! ```bash
//! # Serve the SPA + API on a single port (default 3002)
//! ticket-viewer
//!
//! # Custom port and workspace
//! ticket-viewer --port 3002 --workspace my-project
//! ```
//!
//! # Environment variables
//! - `PORT`       — HTTP listen port (default: 3002)
/// - `STATIC_DIR` — Path to pre-built SPA static files (default: trunk build output)
use std::{
    env,
    io::Write,
    path::{
        Path,
        PathBuf,
    },
    sync::Arc,
};
use tracing::info;
use viewer_api::{
    client_log::{
        client_log_router,
        ClientLogState,
    },
    display_host,
    init_tracing,
    with_static_files,
};

use feedback_http::AppState as FeedbackAppState;
use ticket_api::storage::store::TicketStore;
use ticket_http::serve::{
    AppState,
    StreamBroker,
    WorkspaceRegistry,
};

struct CliOptions {
    port: u16,
    static_dir: PathBuf,
    workspace: Option<String>,
    index_root: Option<PathBuf>,
}

fn parse_cli_options() -> CliOptions {
    let mut port: u16 = env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(3002);

    let mut static_dir: PathBuf = env::var("STATIC_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            // Prefer the trunk WASM frontend build output (trunk build --release).
            // Fall back to the legacy Preact/Vite static/ directory.
            let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
            let trunk_dist = manifest.join("frontend/dioxus/dist");
            if trunk_dist.exists() {
                trunk_dist
            } else {
                manifest.join("static")
            }
        });

    let mut workspace: Option<String> = None;
    let mut index_root: Option<PathBuf> = None;

    let mut args = env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--port" =>
                if let Some(value) = args.next() {
                    if let Ok(parsed) = value.parse::<u16>() {
                        port = parsed;
                    }
                },
            "--static-dir" =>
                if let Some(value) = args.next() {
                    static_dir = PathBuf::from(value);
                },
            "--workspace" => {
                workspace = args.next();
            },
            "--index-root" => {
                index_root = args.next().map(PathBuf::from);
            },
            _ => {},
        }
    }

    CliOptions {
        port,
        static_dir,
        workspace,
        index_root,
    }
}

async fn shutdown_signal() {
    let ctrl_c = tokio::signal::ctrl_c();

    #[cfg(unix)]
    {
        let mut sigterm = tokio::signal::unix::signal(
            tokio::signal::unix::SignalKind::terminate(),
        )
        .expect("failed to register SIGTERM handler");
        tokio::select! {
            _ = ctrl_c => {}
            _ = sigterm.recv() => {}
        }
    }

    #[cfg(not(unix))]
    {
        let _ = ctrl_c.await;
    }
}

fn open_local_ticket_store(
    index_root: &Path
) -> Result<TicketStore, ticket_api::error::StorageError> {
    TicketStore::open(index_root)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let options = parse_cli_options();

    init_tracing("info,ticket_http::serve::handlers=debug");
    info!(
        port = options.port,
        static_dir = %options.static_dir.display(),
        workspace = ?options.workspace,
        "Ticket Viewer starting (single-process mode)"
    );

    let index_root = options.index_root.unwrap_or_else(|| {
        let (path, _source) = ticket_api::workspace::resolve_workspace();
        path
    });

    info!(index_root = %index_root.display(), "Using ticket index root");

    // Always open a single workspace from the resolved index root.
    // We deliberately do NOT load WorkspaceRegistry::from_config() — the
    // global ~/.ticket-workspaces.toml belongs to the CLI, not to this
    // server, which must serve whatever workspace is local to its cwd.
    let store = open_local_ticket_store(&index_root).map_err(|error| {
        std::io::Error::other(format!(
            "Failed to open ticket store at {}: {error}",
            index_root.display()
        ))
    })?;
    let workspace_root =
        ticket_api::workspace::resolve_workspace_root_from_store_root(
            &index_root,
            ticket_api::workspace::TICKET_INDEX_DIR,
        );
    ticket_http::serve::reapply_workspace_scan_policy(
        &store,
        &workspace_root,
    )
    .map_err(|error| {
        std::io::Error::other(format!(
            "failed to apply ticket workspace policy for {}: {error}",
            workspace_root.display()
        ))
    })?;
    let registry = WorkspaceRegistry::single_opened(Arc::new(store));

    // Build the ticket-http AppState and wire up streaming.
    let state =
        AppState::new(Arc::new(registry), Arc::new(StreamBroker::new()));

    // Pre-initialize all known workspaces at startup.
    let workspace_names = state.registry.workspace_names();
    for ws in &workspace_names {
        let _ = state.ensure_workspace_runtime(ws);
    }

    // Build the ticket API router from ticket-http (includes /healthz).
    let app = ticket_http::serve::routes::build_router(state);
    let feedback_state = FeedbackAppState {
        store_root: ticket_api::workspace::resolve_store_root_from(
            &workspace_root,
            ".feedback",
        ),
        workspace_slug: "default".to_string(),
    };
    let app = app.merge(feedback_http::app(feedback_state));
    let app = app.merge(client_log_router(ClientLogState::default()));

    let app =
        with_static_files(app, Some(options.static_dir).filter(|p| p.exists()));

    let addr = format!("0.0.0.0:{}", options.port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    let actual_port = listener.local_addr()?.port();
    info!(
        "Listening on http://{}:{}",
        display_host("0.0.0.0"),
        actual_port
    );
    // Print the actual port on stdout so callers (e.g. the VS Code extension)
    // can discover it when the server was started with --port 0.
    // Explicit flush is required because Rust uses full buffering (not line
    // buffering) when stdout is a pipe, so without it the line sits in the
    // buffer while the server loop runs and callers never see it.
    println!("TICKET_VIEWER_PORT={actual_port}");
    std::io::stdout().flush().expect("failed to flush stdout");
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use tempfile::tempdir;

    use super::open_local_ticket_store;
    use ticket_api::storage::{
        index::RedbIndexStore,
        store::TicketStore,
    };

    #[test]
    fn startup_requires_an_initialized_ticket_store() {
        let dir = tempdir().unwrap();
        let Err(error) = open_local_ticket_store(dir.path()) else {
            panic!("expected an error for an uninitialized ticket store");
        };

        assert!(error.to_string().contains("workspace not initialized"));
        assert!(!dir.path().join(".ticket").exists());
    }

    #[test]
    fn startup_rebuilds_existing_empty_ticket_index() {
        let dir = tempdir().unwrap();
        let store = TicketStore::init(dir.path()).unwrap();
        let ticket_id = store
            .create(
                None,
                "tracker-improvement",
                Some("repair existing empty ticket index"),
                Some("planned"),
                Default::default(),
                None,
                None,
            )
            .unwrap();

        let index_root = store.index_root.clone();
        drop(store);

        std::fs::remove_file(index_root.join("tickets.db")).unwrap();
        let _ = std::fs::remove_file(index_root.join("tickets.db-shm"));
        let _ = std::fs::remove_file(index_root.join("tickets.db-wal"));
        let _ = std::fs::remove_dir_all(index_root.join("search_index"));
        RedbIndexStore::open(&index_root.join("tickets.db")).unwrap();

        let rebuilt = open_local_ticket_store(dir.path()).unwrap();
        let manifest = rebuilt.get(&ticket_id).unwrap();

        assert_eq!(manifest.id, ticket_id);
    }
}
