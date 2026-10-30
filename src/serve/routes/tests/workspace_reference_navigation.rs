#[allow(unused_imports)]
use super::*;

#[tokio::test]
pub(super) async fn descendant_ticket_ref_from_list_is_followable() {
    let dir = tempfile::tempdir().expect("tempdir");
    let child_dir = dir.path().join("child");
    std::fs::create_dir_all(&child_dir).expect("create child dir");

    let parent_store = open_workspace_store(dir.path());
    let child_store = open_workspace_store(child_dir.as_path());
    let child_workspace = primary_workspace_name(child_dir.as_path());

    let child_id = child_store
        .create(
            None,
            "tracker-improvement",
            Some("Child ticket"),
            None,
            BTreeMap::new(),
            None,
            None,
        )
        .expect("create child ticket");

    register_child_workspace(&parent_store, &child_store);
    drop(child_store);

    let app = make_router_from_store(Arc::clone(&parent_store));
    let workspace = primary_workspace_name(dir.path());

    let list_request = Request::builder()
        .method(Method::GET)
        .uri(format!("/api/tickets?workspace={workspace}"))
        .body(Body::empty())
        .unwrap();

    let list_response = app.clone().oneshot(list_request).await.unwrap();
    assert_eq!(list_response.status(), StatusCode::OK);

    let list_bytes = to_bytes(list_response.into_body(), 1024 * 1024)
        .await
        .unwrap();
    let list_payload: serde_json::Value = serde_json::from_slice(&list_bytes).unwrap();
    assert_eq!(
        list_payload["items"][0]["ticket_ref"]["workspace"], child_workspace,
        "unexpected list payload: {list_payload}"
    );
    assert_eq!(
        list_payload["items"][0]["ticket_ref"]["id"],
        child_id.to_string()
    );

    let detail_request = Request::builder()
        .method(Method::GET)
        .uri(format!(
            "/api/tickets/{child_id}?workspace={child_workspace}"
        ))
        .body(Body::empty())
        .unwrap();

    let detail_response = app.oneshot(detail_request).await.unwrap();
    assert_eq!(detail_response.status(), StatusCode::OK);

    let detail_bytes = to_bytes(detail_response.into_body(), 1024 * 1024)
        .await
        .unwrap();
    let detail_payload: serde_json::Value = serde_json::from_slice(&detail_bytes).unwrap();
    assert_eq!(detail_payload["active_workspace"], child_workspace);
    assert_eq!(
        detail_payload["ticket"]["ticket_ref"]["workspace"],
        detail_payload["active_workspace"]
    );
    assert_eq!(
        detail_payload["ticket"]["ticket_ref"]["id"],
        child_id.to_string()
    );
}

#[tokio::test]
pub(super) async fn ancestor_graph_ref_from_child_workspace_is_followable() {
    let dir = tempfile::tempdir().expect("tempdir");
    let child_dir = dir.path().join("child");
    std::fs::create_dir_all(&child_dir).expect("create child dir");

    let parent_store = open_workspace_store(dir.path());
    let child_store = open_workspace_store(&child_dir);

    let parent_id = parent_store
        .create(
            None,
            "tracker-improvement",
            Some("Parent ticket"),
            None,
            BTreeMap::new(),
            None,
            None,
        )
        .expect("create parent ticket");
    let child_id = child_store
        .create(
            None,
            "tracker-improvement",
            Some("Child ticket"),
            None,
            BTreeMap::new(),
            None,
            None,
        )
        .expect("create child ticket");

    register_child_workspace(&parent_store, &child_store);
    parent_store
        .add_edge(ticket_api::model::edge::EdgeRecord {
            from: child_id,
            to: parent_id,
            kind: "depends_on".into(),
            created_at: chrono::Utc::now(),
        })
        .expect("add mixed-workspace edge");
    child_store.scan(true).expect("scan child edge");

    let app = make_router_from_store(Arc::clone(&parent_store));
    let child_workspace = primary_workspace_name(child_dir.as_path());
    let parent_workspace = primary_workspace_name(dir.path());

    let graph_request = Request::builder()
        .method(Method::GET)
        .uri(format!(
            "/api/graph/subgraph?workspace={child_workspace}&root={child_id}&depth=1"
        ))
        .body(Body::empty())
        .unwrap();

    let graph_response = app.clone().oneshot(graph_request).await.unwrap();
    assert_eq!(graph_response.status(), StatusCode::OK);

    let graph_bytes = to_bytes(graph_response.into_body(), 1024 * 1024)
        .await
        .unwrap();
    let graph_payload: serde_json::Value = serde_json::from_slice(&graph_bytes).unwrap();

    let parent_node = graph_payload["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|node| node["id"] == parent_id.to_string())
        .expect("parent node present");
    assert_eq!(
        parent_node["ticket_ref"]["workspace"],
        parent_workspace.clone()
    );
    assert_eq!(parent_node["ticket_ref"]["id"], parent_id.to_string());
    assert_eq!(
        graph_payload["edges"][0]["to_ref"]["workspace"],
        parent_workspace.clone()
    );

    let history_request = Request::builder()
        .method(Method::GET)
        .uri(format!(
            "/api/tickets/{parent_id}/history?workspace={parent_workspace}"
        ))
        .body(Body::empty())
        .unwrap();

    let history_response = app.oneshot(history_request).await.unwrap();
    assert_eq!(history_response.status(), StatusCode::OK);

    let history_bytes = to_bytes(history_response.into_body(), 1024 * 1024)
        .await
        .unwrap();
    let history_payload: serde_json::Value = serde_json::from_slice(&history_bytes).unwrap();
    assert_eq!(
        history_payload["active_workspace"],
        parent_workspace.clone()
    );
    assert_eq!(history_payload["ticket_ref"]["workspace"], parent_workspace);
    assert_eq!(history_payload["ticket_ref"]["id"], parent_id.to_string());
}

#[tokio::test]
pub(super) async fn workspace_graph_includes_isolated_local_and_cross_workspace_nodes() {
    let dir = tempfile::tempdir().expect("tempdir");
    let child_dir = dir.path().join("child");
    std::fs::create_dir_all(&child_dir).expect("create child dir");

    let parent_store = open_workspace_store(dir.path());
    let child_store = open_workspace_store(child_dir.as_path());

    let parent_id = parent_store
        .create(
            None,
            "tracker-improvement",
            Some("Parent ticket"),
            None,
            BTreeMap::new(),
            None,
            None,
        )
        .expect("create parent ticket");
    let child_id = child_store
        .create(
            None,
            "tracker-improvement",
            Some("Child ticket"),
            None,
            BTreeMap::new(),
            None,
            None,
        )
        .expect("create child ticket");
    let isolated_id = child_store
        .create(
            None,
            "tracker-improvement",
            Some("Isolated ticket"),
            None,
            BTreeMap::new(),
            None,
            None,
        )
        .expect("create isolated ticket");

    register_child_workspace(&parent_store, &child_store);
    parent_store
        .add_edge(ticket_api::model::edge::EdgeRecord {
            from: child_id,
            to: parent_id,
            kind: "depends_on".into(),
            created_at: chrono::Utc::now(),
        })
        .expect("add mixed-workspace edge");
    child_store.scan(true).expect("scan child edge");

    let app = make_router_from_store(Arc::clone(&parent_store));
    let workspace_request = Request::builder()
        .method(Method::GET)
        .uri("/api/workspaces")
        .body(Body::empty())
        .unwrap();
    let workspace_response = app.clone().oneshot(workspace_request).await.unwrap();
    assert_eq!(workspace_response.status(), StatusCode::OK);
    let workspace_bytes = to_bytes(workspace_response.into_body(), 1024 * 1024)
        .await
        .unwrap();
    let workspace_payload: serde_json::Value = serde_json::from_slice(&workspace_bytes).unwrap();
    let child_workspace = primary_workspace_name(child_dir.as_path());
    assert!(
        workspace_payload["workspaces"]
            .as_array()
            .unwrap()
            .iter()
            .any(|workspace| workspace["name"] == child_workspace)
    );
    let request = Request::builder()
        .method(Method::GET)
        .uri(format!("/api/graph/workspace?workspace={child_workspace}"))
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let bytes = to_bytes(response.into_body(), 1024 * 1024).await.unwrap();
    let payload: serde_json::Value = serde_json::from_slice(&bytes).unwrap();

    let nodes = payload["nodes"].as_array().expect("nodes array");
    assert!(
        nodes.iter().any(|node| node["id"] == child_id.to_string()),
        "workspace graph should include the local graph root"
    );
    assert!(
        nodes.iter().any(|node| {
            node["id"] == isolated_id.to_string()
                && node["ticket_ref"]["workspace"] == child_workspace
        }),
        "workspace graph should include isolated local tickets"
    );
    assert!(
        nodes.iter().any(|node| node["id"] == parent_id.to_string()),
        "workspace graph should include cross-workspace edge endpoints"
    );

    let edges = payload["edges"].as_array().expect("edges array");
    assert!(
        edges.iter().any(|edge| {
            edge["from"] == child_id.to_string() && edge["to"] == parent_id.to_string()
        }),
        "workspace graph should preserve mixed-workspace relationship metadata"
    );
    assert_eq!(payload["truncated"], false);
}
