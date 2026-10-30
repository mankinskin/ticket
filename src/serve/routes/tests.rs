//! HTTP-level integration tests for the revert route.
//!
//! These tests drive the **full Axum router** (route dispatch, middleware,
//! request parsing, response serialisation) using `tower::ServiceExt` - no
//! real TCP socket required.

use super::build_router;
use crate::serve::{AppState, StreamBroker, WorkspaceRegistry};

use axum::{
    body::{Body, to_bytes},
    http::{Method, Request, StatusCode, header},
};
use std::{collections::BTreeMap, sync::Arc};
use ticket_api::{model::filesystem::ScanRoot, storage::store::TicketStore};
use tower::ServiceExt;
use uuid::Uuid;

fn primary_workspace_name(dir: &std::path::Path) -> String {
    crate::serve::registry::canonical_workspace_name_for_index_root(dir, "workspace")
}

fn make_router(dir: &std::path::Path) -> axum::Router {
    let store = Arc::new(TicketStore::init(dir).expect("open store"));
    store
        .add_scan_root(ScanRoot {
            path: dir.join("tickets"),
            label: "default".into(),
        })
        .expect("add scan root");
    let state = AppState::new(
        Arc::new(WorkspaceRegistry::single_opened(Arc::clone(&store))),
        Arc::new(StreamBroker::new()),
    );
    build_router(state)
}

/// Build a router around an already-opened store (avoids double-open of the SQLite database).
fn make_router_from_store(store: Arc<TicketStore>) -> axum::Router {
    let state = AppState::new(
        Arc::new(WorkspaceRegistry::single_opened(store)),
        Arc::new(StreamBroker::new()),
    );
    build_router(state)
}

/// Create a ticket via the store and return its UUID string.
fn create_ticket(dir: &std::path::Path, title: &str) -> (Arc<TicketStore>, Uuid) {
    let store = Arc::new(TicketStore::init(dir).expect("open store"));
    store
        .add_scan_root(ScanRoot {
            path: dir.join("tickets"),
            label: "default".into(),
        })
        .expect("add scan root");
    let id = store
        .create(
            None,
            "tracker-improvement",
            Some(title),
            None,
            BTreeMap::new(),
            None,
            None,
        )
        .expect("create ticket");
    (store, id)
}

fn open_workspace_store(dir: &std::path::Path) -> Arc<TicketStore> {
    let store = Arc::new(TicketStore::init(dir).expect("open store"));
    store
        .add_scan_root(ScanRoot {
            path: dir.join("tickets"),
            label: "default".into(),
        })
        .expect("add scan root");
    store
}

fn register_child_workspace(parent_store: &TicketStore, child_store: &TicketStore) {
    parent_store
        .add_scan_root(ScanRoot {
            path: child_store.index_root.join("tickets"),
            label: "child".into(),
        })
        .expect("add child scan root");
    parent_store.scan(true).expect("scan child workspace");
}

#[path = "tests/history_and_revert.rs"]
mod history_and_revert;
#[allow(unused_imports)]
pub use history_and_revert::*;

#[path = "tests/concurrent_requests.rs"]
mod concurrent_requests;
#[allow(unused_imports)]
pub use concurrent_requests::*;

#[path = "tests/workspace_reference_navigation.rs"]
mod workspace_reference_navigation;
#[allow(unused_imports)]
pub use workspace_reference_navigation::*;
