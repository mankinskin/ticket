//! Integration tests verifying that ticket-viewer resolves workspaces
//! identically to the CLI — via `ticket_api::workspace` — and that the
//! HTTP API reflects the resolved store contents.

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
    workspace::{
        find_local_root_from,
        resolve_workspace_from,
        TICKET_INDEX_DIR,
    },
};
use ticket_http::serve::{
    AppState,
    StreamBroker,
    WorkspaceRegistry,
};

// ── Helpers ──────────────────────────────────────────────────────────────────

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

fn build_app(store: Arc<TicketStore>) -> (axum::Router, String) {
    let registry = Arc::new(WorkspaceRegistry::single_opened(store));
    let workspace = registry.primary_workspace_name().to_string();
    let state = AppState::new(registry, Arc::new(StreamBroker::new()));
    let _ = state.ensure_workspace_runtime(&workspace);
    (ticket_http::build_router(state), workspace)
}

async fn get_ticket_count(
    app: axum::Router,
    workspace: &str,
) -> (StatusCode, usize) {
    let req = Request::builder()
        .uri(format!("/api/tickets?workspace={workspace}"))
        .body(axum::body::Body::empty())
        .unwrap();

    let resp = app.oneshot(req).await.expect("request should succeed");
    let status = resp.status();
    let bytes = to_bytes(resp.into_body(), 4 * 1024 * 1024)
        .await
        .expect("read body");
    let payload: serde_json::Value =
        serde_json::from_slice(&bytes).expect("parse json");
    let count = payload["items"].as_array().map(|a| a.len()).unwrap_or(0);
    (status, count)
}

// ── Workspace resolution tests ───────────────────────────────────────────────

#[test]
fn local_workspace_file_is_found_walking_up() {
    let root = tempfile::tempdir().expect("tempdir");
    let nested = root.path().join("a").join("b").join("c");
    std::fs::create_dir_all(&nested).expect("mkdir");

    std::fs::create_dir(root.path().join(TICKET_INDEX_DIR))
        .expect("create .ticket");

    let found = find_local_root_from(&nested, TICKET_INDEX_DIR);
    assert!(found.is_some(), "should find .ticket walking up");
    assert_eq!(found.unwrap(), root.path().join(TICKET_INDEX_DIR),);
}

#[test]
fn local_workspace_file_not_found_in_empty_tree() {
    let root = tempfile::tempdir().expect("tempdir");
    let found = find_local_root_from(root.path(), TICKET_INDEX_DIR);
    if let Some(path) = found {
        assert!(
            !path.starts_with(root.path()),
            "should not find .ticket inside the temp dir",
        );
    }
}

#[test]
fn workspace_file_relative_path_resolves_from_file_parent() {
    let root = tempfile::tempdir().expect("tempdir");
    let ticket_dir = root.path().join(TICKET_INDEX_DIR);
    std::fs::create_dir_all(&ticket_dir).expect("mkdir .ticket");
    let probe_file = root.path().join("notes.txt");
    std::fs::write(&probe_file, "workspace probe").expect("write probe");

    let (resolved, _source) = resolve_workspace_from(&probe_file);

    assert_eq!(resolved, ticket_dir);
}

// ── HTTP API integration tests ───────────────────────────────────────────────

#[tokio::test]
async fn empty_store_returns_zero_tickets() {
    let dir = tempfile::tempdir().expect("tempdir");
    let store = open_store(dir.path());
    let (app, workspace) = build_app(store);

    let (status, count) = get_ticket_count(app, &workspace).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(count, 0, "empty store should return 0 tickets");
}

#[tokio::test]
async fn store_with_tickets_returns_correct_count() {
    let dir = tempfile::tempdir().expect("tempdir");
    let store = open_store(dir.path());

    // Create 3 tickets
    for i in 0..3 {
        store
            .create(
                None,
                "tracker-improvement",
                Some(&format!("test ticket {i}")),
                None,
                BTreeMap::new(),
                None,
                None,
            )
            .expect("create ticket");
    }

    let (app, workspace) = build_app(store);
    let (status, count) = get_ticket_count(app, &workspace).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(count, 3, "store with 3 tickets should return 3");
}

#[tokio::test]
async fn resolved_workspace_serves_correct_store() {
    let root = tempfile::tempdir().expect("tempdir");
    let ticket_dir = root.path().join(TICKET_INDEX_DIR);
    std::fs::create_dir_all(&ticket_dir).expect("mkdir");

    let (index_root, _source) = resolve_workspace_from(root.path());

    let store =
        Arc::new(TicketStore::open_or_init(&index_root).expect("open store"));
    store
        .add_scan_root(ScanRoot {
            path: index_root.join("tickets"),
            label: "default".into(),
        })
        .expect("add scan root");

    store
        .create(
            None,
            "tracker-improvement",
            Some("resolved workspace ticket"),
            None,
            BTreeMap::new(),
            None,
            None,
        )
        .expect("create ticket");

    let (app, workspace) = build_app(store);
    let (status, count) = get_ticket_count(app, &workspace).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(count, 1, "resolved workspace should serve its tickets");
}

#[tokio::test]
async fn separate_workspaces_are_isolated() {
    // Two independent temp dirs, each with their own store.
    // Tickets in one should not appear in the other.

    let dir_a = tempfile::tempdir().expect("tempdir a");
    let dir_b = tempfile::tempdir().expect("tempdir b");

    let store_a = open_store(dir_a.path());
    let store_b = open_store(dir_b.path());

    // Create 2 tickets in store A, 0 in store B
    for i in 0..2 {
        store_a
            .create(
                None,
                "tracker-improvement",
                Some(&format!("store_a ticket {i}")),
                None,
                BTreeMap::new(),
                None,
                None,
            )
            .expect("create in store_a");
    }

    let (app_a, workspace_a) = build_app(store_a);
    let (app_b, workspace_b) = build_app(store_b);

    let (_, count_a) = get_ticket_count(app_a, &workspace_a).await;
    let (_, count_b) = get_ticket_count(app_b, &workspace_b).await;

    assert_eq!(count_a, 2, "store_a should have 2 tickets");
    assert_eq!(count_b, 0, "store_b should have 0 tickets");
}
