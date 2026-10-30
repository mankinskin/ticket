#[allow(unused_imports)]
use super::*;

#[tokio::test]
pub(super) async fn search_list_excludes_stale_search_hits_and_followups() {
    let dir = tempfile::tempdir().expect("tempdir");
    let store = make_store(dir.path());

    let id = store
        .create(
            None,
            "tracker-improvement",
            Some("deleted-hit regression ticket"),
            Some("planning"),
            BTreeMap::new(),
            None,
            Some("deleted-hit regression description"),
        )
        .expect("create ticket");
    let ticket_dir = store
        .get_indexed(&id)
        .expect("get indexed ticket")
        .expect("indexed ticket");
    std::fs::create_dir_all(ticket_dir.path.join("assets")).expect("mkdir assets");
    std::fs::write(
        ticket_dir.path.join("assets").join("plan.md"),
        "deleted hit asset",
    )
    .expect("write asset");

    store.delete(&id).expect("delete ticket");
    TantivySearchIndex::open_or_create(&store.index_root.join("search_index"))
        .expect("open search index")
        .upsert(
            &id,
            Some("deleted-hit regression ticket"),
            Some("deleted-hit regression description"),
            Some("planned"),
            Some("tracker-improvement"),
            Some(&chrono::Utc::now().to_rfc3339()),
            None,
        )
        .expect("upsert deleted residual search doc");

    let state = make_state(Arc::clone(&store));
    let workspace = state.registry.primary_workspace_name().to_string();

    let list = list_tickets(
        State(state.clone()),
        Extension(RequestIdExt("rid-deleted-list".to_string())),
        Query(WorkspaceParam {
            workspace: workspace.clone(),
            state: None,
            query: Some("deleted-hit regression".to_string()),
            limit: Some(10),
            cursor: None,
        }),
    )
    .await;
    assert_eq!(list.status(), StatusCode::OK);
    let list_bytes = to_bytes(list.into_body(), 1024 * 1024)
        .await
        .expect("list body");
    let list_payload: serde_json::Value = serde_json::from_slice(&list_bytes).expect("list json");
    assert!(
        list_payload["items"]
            .as_array()
            .expect("items array")
            .is_empty(),
        "deleted residual search hits must be dropped"
    );

    let detail = get_ticket(
        State(state.clone()),
        Extension(RequestIdExt("rid-deleted-detail".to_string())),
        Path(id),
        Query(TicketIdParam {
            workspace: workspace.clone(),
            view: None,
            parts: None,
        }),
    )
    .await;
    assert_eq!(detail.status(), StatusCode::NOT_FOUND);

    let description = get_ticket_description(
        State(state.clone()),
        Extension(RequestIdExt("rid-deleted-description".to_string())),
        Path(id),
        Query(TicketIdParam {
            workspace: workspace.clone(),
            view: None,
            parts: None,
        }),
    )
    .await;
    assert_eq!(description.status(), StatusCode::NOT_FOUND);

    let history = get_ticket_history(
        State(state.clone()),
        Extension(RequestIdExt("rid-deleted-history".to_string())),
        Path(id),
        Query(TicketIdParam {
            workspace: workspace.clone(),
            view: None,
            parts: None,
        }),
    )
    .await;
    assert_eq!(history.status(), StatusCode::NOT_FOUND);

    let files = list_ticket_files(
        State(state.clone()),
        Extension(RequestIdExt("rid-deleted-files".to_string())),
        Path(id),
        Query(TicketIdParam {
            workspace: workspace.clone(),
            view: None,
            parts: None,
        }),
    )
    .await;
    assert_eq!(files.status(), StatusCode::NOT_FOUND);

    let asset = get_ticket_asset(
        State(state),
        Extension(RequestIdExt("rid-deleted-asset".to_string())),
        Path(id),
        Query(TicketAssetParam {
            workspace,
            path: "assets/plan.md".to_string(),
        }),
    )
    .await;
    assert_eq!(asset.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
pub(super) async fn search_list_combines_query_and_state_before_limit() {
    let dir = tempfile::tempdir().expect("tempdir");
    let store = make_store(dir.path());

    store
        .create(
            None,
            "tracker-improvement",
            Some("needle needle needle wrong-state"),
            Some("planning"),
            BTreeMap::new(),
            None,
            None,
        )
        .expect("create higher-ranked wrong-state ticket");

    let ready_id = store
        .create(
            None,
            "tracker-improvement",
            Some("needle right-state"),
            Some("ready"),
            BTreeMap::new(),
            None,
            None,
        )
        .expect("create matching ready ticket");

    let state = make_state(Arc::clone(&store));
    let workspace = state.registry.primary_workspace_name().to_string();

    let response = list_tickets(
        State(state),
        Extension(RequestIdExt("rid-test".to_string())),
        Query(WorkspaceParam {
            workspace: workspace.clone(),
            state: Some("ready".to_string()),
            query: Some("needle".to_string()),
            limit: Some(1),
            cursor: None,
        }),
    )
    .await;

    let bytes = to_bytes(response.into_body(), 1024 * 1024)
        .await
        .expect("read body");
    let payload: serde_json::Value = serde_json::from_slice(&bytes).expect("json body");
    let items = payload["items"].as_array().expect("items array");
    let ready_id = ready_id.to_string();

    assert_eq!(items.len(), 1);
    assert_eq!(items[0]["id"].as_str(), Some(ready_id.as_str()));
    assert_eq!(items[0]["state"].as_str(), Some("ready"));
}
