#[allow(unused_imports)]
use super::*;

/// Verify that multiple concurrent subgraph requests all complete without
/// deadlocking. This exercises the `spawn_blocking` path in the graph
/// handlers: if blocking storage I/O were performed on an async worker
/// thread, the single-threaded test runtime would stall and the timeouts
/// below would fire.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
pub(super) async fn concurrent_subgraph_requests_all_complete() {
    use tokio::time::{Duration, timeout};

    let dir = tempfile::tempdir().expect("tempdir");
    let store = Arc::new(TicketStore::init(dir.path()).expect("open store"));
    let workspace = primary_workspace_name(dir.path());
    store
        .add_scan_root(ScanRoot {
            path: dir.path().join("tickets"),
            label: "default".into(),
        })
        .expect("add scan root");

    // Create 8 tickets so each concurrent request has a unique root.
    let ids: Vec<Uuid> = (0..8)
        .map(|i| {
            store
                .create(
                    None,
                    "tracker-improvement",
                    Some(&format!("Concurrent ticket {i}")),
                    None,
                    BTreeMap::new(),
                    None,
                    None,
                )
                .expect("create ticket")
        })
        .collect();

    let app = make_router_from_store(Arc::clone(&store));

    let handles: Vec<_> = ids
        .iter()
        .map(|id| {
            // `Router` implements `Clone` - each task gets its own clone.
            let app = app.clone();
            let id = *id;
            let workspace = workspace.clone();
            tokio::spawn(async move {
                let req = Request::builder()
                    .uri(format!(
                        "/api/graph/subgraph?workspace={workspace}&root={id}&depth=2"
                    ))
                    .body(Body::empty())
                    .unwrap();
                timeout(Duration::from_secs(5), app.oneshot(req))
                    .await
                    .expect("request should complete within 5 s")
                    .expect("oneshot should not error")
            })
        })
        .collect();

    for handle in handles {
        let resp = handle.await.expect("task panicked");
        assert_eq!(
            resp.status(),
            StatusCode::OK,
            "subgraph request returned non-200"
        );
    }
}

/// Verify that multiple concurrent ticket-list requests all complete.
/// The list handler hits the storage layer on every call; running 8 at
/// once confirms there is no mutex starvation or deadlock.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
pub(super) async fn concurrent_list_requests_all_complete() {
    use tokio::time::{Duration, timeout};
    let dir = tempfile::tempdir().expect("tempdir");
    let store = Arc::new(TicketStore::init(dir.path()).expect("open store"));
    let workspace = primary_workspace_name(dir.path());
    store
        .add_scan_root(ScanRoot {
            path: dir.path().join("tickets"),
            label: "default".into(),
        })
        .expect("add scan root");

    // Create a few tickets so the list response is non-trivial.
    for i in 0..5 {
        store
            .create(
                None,
                "tracker-improvement",
                Some(&format!("List ticket {i}")),
                None,
                BTreeMap::new(),
                None,
                None,
            )
            .expect("create ticket");
    }

    let app = make_router_from_store(Arc::clone(&store));

    let handles: Vec<_> = (0..8)
        .map(|_| {
            let app = app.clone();
            let workspace = workspace.clone();
            tokio::spawn(async move {
                let req = Request::builder()
                    .uri(format!("/api/tickets?workspace={workspace}"))
                    .body(Body::empty())
                    .unwrap();
                timeout(Duration::from_secs(5), app.oneshot(req))
                    .await
                    .expect("request should complete within 5 s")
                    .expect("oneshot should not error")
            })
        })
        .collect();

    for handle in handles {
        let resp = handle.await.expect("task panicked");
        assert_eq!(resp.status(), StatusCode::OK);
    }
}
