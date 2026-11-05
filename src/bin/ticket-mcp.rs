#![recursion_limit = "256"]

use ticket::server::{self, open_canonical_store};

use std::{path::PathBuf, time::Instant};

use memory_kernel::runtime::init_transport_tracing;

#[tokio::main]
async fn main() {
    if std::env::args()
        .skip(1)
        .any(|arg| matches!(arg.as_str(), "-h" | "--help"))
    {
        println!("Ticket MCP server\n\nUsage: ticket-mcp");
        return;
    }

    init_transport_tracing(
        "ticket=info",
        None,
        None,
        "warn,ticket_mcp=info,ticket_api::storage::store=info",
    );
    let startup_started = Instant::now();
    tracing::info!("ticket_mcp_startup_started");

    let index_root = std::env::var("TICKET_INDEX_ROOT")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            let (path, _source) = ticket_api::workspace::resolve_workspace();
            path
        });

    let store = open_canonical_store(&index_root).unwrap_or_else(|e| {
        eprintln!(
            "Failed to open ticket store at {}: {e}",
            index_root.display()
        );
        std::process::exit(1);
    });
    let index_root = store.index_root.clone();
    let close_started = Instant::now();
    drop(store);
    tracing::info!(
        store_close_ms = close_started.elapsed().as_millis() as u64,
        startup_ms = startup_started.elapsed().as_millis() as u64,
        "ticket_mcp_startup_complete"
    );

    let workspace_names = vec!["default".to_string()];

    eprintln!(
        "ticket-mcp starting (store: {}, workspaces: {:?})",
        index_root.display(),
        workspace_names,
    );

    if let Err(err) = server::run_mcp_server(index_root).await {
        eprintln!("Fatal error: {err}");
        std::process::exit(1);
    }
}
