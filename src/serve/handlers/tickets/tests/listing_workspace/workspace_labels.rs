#[allow(unused_imports)]
use super::*;

#[tokio::test]
pub(super) async fn list_tickets_uses_scan_root_label_for_ticket_ref_workspace() {
    let dir = tempfile::tempdir().expect("tempdir");
    let store = make_store(dir.path());
    let child_root = dir.path().join("child").join("tickets");
    std::fs::create_dir_all(&child_root).expect("mkdir child root");

    store
        .add_scan_root(ticket_api::model::filesystem::ScanRoot {
            path: child_root.clone(),
            label: "child".to_string(),
        })
        .expect("add child scan root");

    let id = store
        .create(
            None,
            "tracker-improvement",
            Some("child-owned ticket"),
            Some("planning"),
            BTreeMap::new(),
            Some(child_root.as_path()),
            None,
        )
        .expect("create child ticket");

    let state = make_state(Arc::clone(&store));
    let workspace = state.registry.primary_workspace_name().to_string();
    let response = list_tickets(
        State(state),
        Extension(RequestIdExt("rid-child".to_string())),
        Query(WorkspaceParam {
            workspace: workspace.clone(),
            state: None,
            query: Some("child-owned".to_string()),
            limit: Some(10),
            cursor: None,
        }),
    )
    .await;

    let bytes = to_bytes(response.into_body(), 1024 * 1024)
        .await
        .expect("read body");
    let payload: serde_json::Value = serde_json::from_slice(&bytes).expect("json body");

    assert_eq!(payload["items"][0]["ticket_ref"]["workspace"], "child");
    assert_eq!(payload["items"][0]["ticket_ref"]["id"], id.to_string());
}

#[tokio::test]
pub(super) async fn search_list_prefers_authoritative_mixed_workspace_hit() {
    let root = tempfile::tempdir().expect("tempdir");
    let parent_store = Arc::new(
        ticket_api::storage::store::TicketStore::init(root.path()).expect("open parent store"),
    );
    parent_store
        .add_scan_root(ticket_api::model::filesystem::ScanRoot {
            path: root.path().join("tickets"),
            label: "default".to_string(),
        })
        .expect("add parent scan root");

    let child_index_root = root
        .path()
        .join("child")
        .join(".workflow-tools")
        .join("ticket");
    std::fs::create_dir_all(child_index_root.join("tickets")).expect("mkdir child store");
    let child_store = Arc::new(
        ticket_api::storage::store::TicketStore::init(&child_index_root).expect("open child store"),
    );
    child_store
        .add_scan_root(ticket_api::model::filesystem::ScanRoot {
            path: child_index_root.join("tickets"),
            label: "tickets".to_string(),
        })
        .expect("add child scan root");

    let id = child_store
        .create(
            None,
            "tracker-improvement",
            Some("mixed-workspace child authoritative ticket"),
            Some("ready"),
            BTreeMap::new(),
            None,
            Some("mixed-workspace authoritative description"),
        )
        .expect("create child ticket");

    parent_store
        .add_scan_root(ticket_api::model::filesystem::ScanRoot {
            path: child_index_root.join("tickets"),
            label: "tickets".to_string(),
        })
        .expect("add child scan root to parent");
    parent_store.scan(true).expect("scan parent store");

    let poisoned_index = ticket_api::storage::index::RedbIndexStore::open(
        &parent_store.index_root.join("tickets.db"),
    )
    .expect("open parent index");
    let mut poisoned = parent_store
        .get_indexed(&id)
        .expect("get parent indexed ticket")
        .expect("parent indexed ticket");
    poisoned.path = parent_store.index_root.join("tickets").join(id.to_string());
    poisoned.title = Some("mixed-workspace stale parent placeholder".to_string());
    poisoned.state = Some("open".to_string());
    poisoned_index
        .insert_ticket(&poisoned)
        .expect("poison parent indexed row");
    TantivySearchIndex::open_or_create(&parent_store.index_root.join("search_index"))
        .expect("open parent search index")
        .upsert(
            &id,
            Some("mixed-workspace stale parent placeholder"),
            Some("mixed-workspace stale parent body"),
            Some("open"),
            Some("tracker-improvement"),
            Some(&chrono::Utc::now().to_rfc3339()),
            None,
        )
        .expect("upsert stale parent search doc");

    let state = make_state(Arc::clone(&parent_store));
    let workspace = state.registry.primary_workspace_name().to_string();
    let child_workspace = state
        .registry
        .workspace_infos()
        .into_iter()
        .find(|info| info.label == "child")
        .expect("child workspace info")
        .name;
    let response = list_tickets(
        State(state),
        Extension(RequestIdExt("rid-authoritative-hit".to_string())),
        Query(WorkspaceParam {
            workspace,
            state: None,
            query: Some("mixed-workspace".to_string()),
            limit: Some(10),
            cursor: None,
        }),
    )
    .await;

    assert_eq!(response.status(), StatusCode::OK);
    let bytes = to_bytes(response.into_body(), 1024 * 1024)
        .await
        .expect("read body");
    let payload: serde_json::Value = serde_json::from_slice(&bytes).expect("json body");
    let items = payload["items"].as_array().expect("items array");

    assert_eq!(items.len(), 1);
    assert_eq!(
        items[0]["ticket_ref"]["workspace"],
        child_workspace.as_str()
    );
    assert_eq!(items[0]["ticket_ref"]["id"], id.to_string());
    assert_eq!(
        items[0]["title"],
        serde_json::Value::String("mixed-workspace child authoritative ticket".to_string())
    );
    assert_eq!(items[0]["state"], "ready");
    assert_ne!(items[0]["created_at"], "1970-01-01T00:00:00Z");
}
