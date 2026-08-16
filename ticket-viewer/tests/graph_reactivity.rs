//! Integration tests verifying that graph reactivity works correctly.
//! When a ticket is updated, the graph layout cache is invalidated and
//! a fresh layout is fetched.

use std::{
    collections::BTreeMap,
    sync::Arc,
};

use axum::{
    body::to_bytes,
    http::{
        Request,
        StatusCode,
    },
};
use tower::ServiceExt; // for `oneshot`

use ticket_api::{
    model::filesystem::ScanRoot,
    storage::store::TicketStore,
};
use ticket_http::serve::{
    AppState,
    StreamBroker,
    WorkspaceRegistry,
};

fn open_store(dir: &std::path::Path) -> Arc<TicketStore> {
    let store = Arc::new(TicketStore::open_or_init(dir).expect("open store"));
    store
        .add_scan_root(ScanRoot {
            path: dir.join("tickets"),
            label: "default".into(),
        })
        .expect("add scan root");
    store
}

fn build_app(store: Arc<TicketStore>) -> (axum::Router, String, AppState) {
    let registry = Arc::new(WorkspaceRegistry::single_opened(store));
    let workspace = registry.primary_workspace_name().to_string();
    let broker = Arc::new(StreamBroker::new());
    let state = AppState::new(registry, broker);
    let _ = state.ensure_workspace_runtime(&workspace);
    (ticket_http::build_router(state.clone()), workspace, state)
}

#[tokio::test]
async fn test_sse_ticket_upsert_emitted() {
    let dir = tempfile::tempdir().expect("tempdir");
    let store = open_store(dir.path());
    let (_app, workspace, state) = build_app(store.clone());

    let mut rx = state.broker.subscribe(&workspace);

    // Create a ticket to trigger ticket.upsert
    let ticket_id = store
        .create(
            None,
            "tracker-improvement",
            Some("reactivity test ticket"),
            None,
            BTreeMap::new(),
            None,
            None,
        )
        .expect("create ticket");

    // We should receive a ticket.upsert event
    let event =
        tokio::time::timeout(std::time::Duration::from_secs(2), async {
            loop {
                match rx.recv().await {
                    Ok((_id, ev)) =>
                        if ev.event_name() == "ticket.upsert" {
                            return ev;
                        },
                    Err(e) => panic!("SSE receive error: {:?}", e),
                }
            }
        })
        .await
        .expect("Timeout waiting for ticket.upsert SSE event");

    assert_eq!(event.event_name(), "ticket.upsert");
    let data_str = event.data_json().expect("serialize event data");
    assert!(data_str.contains(&ticket_id.to_string()));
}
