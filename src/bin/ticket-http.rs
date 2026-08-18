//! Standalone binary for the ticket HTTP server.
//!
//! Usage:
//!   ticket-http --port 4000 [--host 127.0.0.1] [--index-root <path>]
//!               [--log-level debug|info|warn|error]
//!               [--log-file /path/to/ticket-http.log]
//!
//! Tracing
//! -------
//! The server uses `tracing-subscriber` for structured logging.
//!
//! Log level precedence (highest wins):
//!   1. `--log-level` CLI argument
//!   2. `RUST_LOG` environment variable
//!   3. default: `debug` in debug builds, `info` in release builds
//!
//! All events are written to stderr.  When `--log-file <path>` is given, a
//! copy of every event is also appended to that file (non-blocking, auto-rolled
//! daily).  This is the primary path for capturing the "ticket serialization
//! error" family of failures.

use memory_kernel::runtime::init_transport_tracing;
use ticket_api::storage::store::TicketStore;
use ticket::serve::{
    ServeConfig,
    WorkspaceRegistry,
};

fn main() {
    if std::env::args().skip(1).any(|arg| matches!(arg.as_str(), "-h" | "--help")) {
        println!("Ticket HTTP server\n\nUsage: ticket-http [--port <port>] [--host <host>] [--index-root <path>] [--log-level <level>] [--log-file <path>]");
        return;
    }

    let mut port: u16 = 4000;
    let mut host = "127.0.0.1".to_string();
    let mut index_root: Option<String> = None;
    let mut log_level: Option<String> = None;
    let mut log_file: Option<String> = None;

    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--port" =>
                if let Some(v) = args.next() {
                    port = v.parse().unwrap_or(port);
                },
            "--host" =>
                if let Some(v) = args.next() {
                    host = v;
                },
            "--index-root" => {
                index_root = args.next();
            },
            "--log-level" => {
                log_level = args.next();
            },
            "--log-file" => {
                log_file = args.next();
            },
            _ => {},
        }
    }

    init_transport_tracing(
        "ticket_http=info",
        log_level.as_deref(),
        log_file.as_deref().map(std::path::Path::new),
        default_log_level(),
    );

    let root = index_root.map(std::path::PathBuf::from).unwrap_or_else(|| {
        let (path, _source) = ticket_api::workspace::resolve_workspace();
        path
    });
    let workspace_root =
        ticket_api::workspace::resolve_workspace_root_from_store_root(
            &root,
            ticket_api::workspace::TICKET_INDEX_DIR,
        );
    let store =
        TicketStore::open_or_init(&root).expect("failed to open ticket store");
    ticket::serve::reapply_workspace_scan_policy(&store, &workspace_root)
        .expect("failed to apply workspace policy");

    let registry = WorkspaceRegistry::single_opened(std::sync::Arc::new(store));

    let config = ServeConfig { host, port };

    tracing::info!(
        port,
        host = %config.host,
        "ticket-http starting"
    );

    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("failed to start tokio runtime");

    rt.block_on(async {
        ticket::serve::serve(config, registry)
            .await
            .expect("server error");
    });
}

fn default_log_level() -> &'static str {
    #[cfg(debug_assertions)]
    {
        "debug"
    }
    #[cfg(not(debug_assertions))]
    {
        "info"
    }
}
