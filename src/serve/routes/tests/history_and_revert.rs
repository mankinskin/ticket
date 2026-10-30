#[allow(unused_imports)]
use super::*;

#[tokio::test]
pub(super) async fn revert_route_returns_200_with_restored_state() {
    let dir = tempfile::tempdir().expect("tempdir");
    let (store, id) = create_ticket(dir.path(), "Router revert test");
    let workspace = primary_workspace_name(dir.path());

    // Advance state so there is a revision 1 (new) and revision 2 (ready).
    store
        .update(&id, BTreeMap::new(), None, Some("ready"), None, None)
        .expect("advance to ready");

    let app = make_router_from_store(Arc::clone(&store));

    let body = serde_json::json!({ "revision": 1 }).to_string();
    let request = Request::builder()
        .method(Method::POST)
        .uri(format!("/api/tickets/{id}/revert?workspace={workspace}"))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(body))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let bytes = to_bytes(response.into_body(), 1024 * 1024).await.unwrap();
    let payload: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(payload["ticket"]["fields"]["state"], "planning");
    assert_eq!(payload["ticket"]["fields"]["title"], "Router revert test");
    // request_id header is injected by middleware - must be present.
    assert!(payload.get("request_id").is_some());
    assert_eq!(payload["workspace"], workspace);
}

#[tokio::test]
pub(super) async fn revert_route_returns_400_for_missing_revision() {
    let dir = tempfile::tempdir().expect("tempdir");
    let (_store, id) = create_ticket(dir.path(), "T");
    let workspace = primary_workspace_name(dir.path());

    let app = make_router_from_store(Arc::clone(&_store));

    // revision 99 does not exist - only revision 1 was created.
    let body = serde_json::json!({ "revision": 99 }).to_string();
    let request = Request::builder()
        .method(Method::POST)
        .uri(format!("/api/tickets/{id}/revert?workspace={workspace}"))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(body))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    let bytes = to_bytes(response.into_body(), 1024 * 1024).await.unwrap();
    let payload: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(payload["code"], "revision_not_found");
}

#[tokio::test]
pub(super) async fn revert_route_returns_404_for_unknown_ticket() {
    let dir = tempfile::tempdir().expect("tempdir");
    let app = make_router(dir.path());
    let workspace = primary_workspace_name(dir.path());

    let fake_id = Uuid::new_v4();
    let body = serde_json::json!({ "revision": 1 }).to_string();
    let request = Request::builder()
        .method(Method::POST)
        .uri(format!(
            "/api/tickets/{fake_id}/revert?workspace={workspace}"
        ))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(body))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
pub(super) async fn revert_route_rejects_wrong_http_method() {
    let dir = tempfile::tempdir().expect("tempdir");
    let (_store, id) = create_ticket(dir.path(), "T");
    let app = make_router_from_store(Arc::clone(&_store));
    let workspace = primary_workspace_name(dir.path());

    // GET is not registered for the revert path.
    let request = Request::builder()
        .method(Method::GET)
        .uri(format!("/api/tickets/{id}/revert?workspace={workspace}"))
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::METHOD_NOT_ALLOWED);
}

#[tokio::test]
pub(super) async fn history_route_returns_200_with_revision_entries() {
    let dir = tempfile::tempdir().expect("tempdir");
    let (store, id) = create_ticket(dir.path(), "History smoke");
    let workspace = primary_workspace_name(dir.path());

    // Add a second revision so history has 2 entries.
    store
        .update(&id, BTreeMap::new(), None, Some("ready"), None, None)
        .expect("advance state");

    let app = make_router_from_store(Arc::clone(&store));

    let request = Request::builder()
        .method(Method::GET)
        .uri(format!("/api/tickets/{id}/history?workspace={workspace}"))
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let bytes = to_bytes(response.into_body(), 1024 * 1024).await.unwrap();
    let payload: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(payload["count"], 2);
    // Oldest-first: first entry is the initial creation revision.
    assert_eq!(payload["entries"][0]["rev"], 1);
    assert_eq!(payload["entries"][0]["fields"]["state"], "planning");
}
