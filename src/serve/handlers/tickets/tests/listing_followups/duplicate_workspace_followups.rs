#[allow(unused_imports)]
use super::*;

#[tokio::test]
pub(super) async fn duplicate_basename_workspaces_keep_followups_distinct() {
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

    let left_index_root = root
        .path()
        .join("alpha")
        .join("shared")
        .join(".workflow-tools")
        .join("ticket");
    let right_index_root = root
        .path()
        .join("beta")
        .join("shared")
        .join(".workflow-tools")
        .join("ticket");
    std::fs::create_dir_all(left_index_root.join("tickets")).expect("mkdir left store");
    std::fs::create_dir_all(right_index_root.join("tickets")).expect("mkdir right store");

    let left_store = Arc::new(
        ticket_api::storage::store::TicketStore::init(&left_index_root).expect("open left store"),
    );
    let right_store = Arc::new(
        ticket_api::storage::store::TicketStore::init(&right_index_root).expect("open right store"),
    );
    left_store
        .add_scan_root(ticket_api::model::filesystem::ScanRoot {
            path: left_index_root.join("tickets"),
            label: "tickets".to_string(),
        })
        .expect("add left scan root");
    right_store
        .add_scan_root(ticket_api::model::filesystem::ScanRoot {
            path: right_index_root.join("tickets"),
            label: "tickets".to_string(),
        })
        .expect("add right scan root");

    let left_id = left_store
        .create(
            None,
            "tracker-improvement",
            Some("left shared ticket"),
            Some("planning"),
            BTreeMap::new(),
            None,
            Some("left shared description"),
        )
        .expect("create left ticket");
    let right_id = right_store
        .create(
            None,
            "tracker-improvement",
            Some("right shared ticket"),
            Some("planning"),
            BTreeMap::new(),
            None,
            Some("right shared description"),
        )
        .expect("create right ticket");

    for (store, id, asset_content) in [
        (Arc::clone(&left_store), left_id, "left shared asset"),
        (Arc::clone(&right_store), right_id, "right shared asset"),
    ] {
        let ticket_dir = store
            .get_indexed(&id)
            .expect("get indexed ticket")
            .expect("indexed ticket");
        std::fs::create_dir_all(ticket_dir.path.join("assets")).expect("mkdir assets");
        std::fs::write(
            ticket_dir.path.join("assets").join("plan.md"),
            asset_content,
        )
        .expect("write asset");
    }

    parent_store
        .add_scan_root(ticket_api::model::filesystem::ScanRoot {
            path: left_index_root.join("tickets"),
            label: "tickets".to_string(),
        })
        .expect("add left scan root to parent");
    parent_store
        .add_scan_root(ticket_api::model::filesystem::ScanRoot {
            path: right_index_root.join("tickets"),
            label: "tickets".to_string(),
        })
        .expect("add right scan root to parent");
    parent_store.scan(true).expect("scan parent store");

    let state = make_state(parent_store);
    let mut shared_workspaces = state
        .registry
        .workspace_infos()
        .into_iter()
        .filter(|info| info.label == "shared")
        .map(|info| info.name)
        .collect::<Vec<_>>();
    shared_workspaces.sort();
    assert_eq!(shared_workspaces.len(), 2);
    assert_ne!(shared_workspaces[0], shared_workspaces[1]);

    for workspace in shared_workspaces {
        let list = list_tickets(
            State(state.clone()),
            Extension(RequestIdExt(format!("rid-list-{workspace}"))),
            Query(WorkspaceParam {
                workspace: workspace.to_string(),
                state: None,
                query: Some("shared ticket".to_string()),
                limit: Some(10),
                cursor: None,
            }),
        )
        .await;
        assert_eq!(list.status(), StatusCode::OK);
        let list_bytes = to_bytes(list.into_body(), 1024 * 1024)
            .await
            .expect("list body");
        let list_payload: serde_json::Value =
            serde_json::from_slice(&list_bytes).expect("list json");
        let title = list_payload["items"][0]["title"].as_str().expect("title");
        let (expected_description, expected_asset) = match title {
            "left shared ticket" => ("left shared description", "left shared asset"),
            "right shared ticket" => ("right shared description", "right shared asset"),
            other => panic!("unexpected shared workspace title: {other}"),
        };
        let id = Uuid::parse_str(list_payload["items"][0]["id"].as_str().expect("ticket id"))
            .expect("valid ticket id");
        assert_eq!(list_payload["active_workspace"], workspace);
        assert_eq!(list_payload["workspace"], workspace);
        assert_eq!(
            list_payload["items"][0]["ticket_ref"]["workspace"],
            workspace
        );
        assert_eq!(list_payload["items"][0]["ticket_ref"]["id"], id.to_string());
        assert_eq!(list_payload["items"][0]["title"], title);

        let detail = get_ticket(
            State(state.clone()),
            Extension(RequestIdExt(format!("rid-detail-{workspace}"))),
            Path(id),
            Query(TicketIdParam {
                workspace: workspace.to_string(),
                view: None,
                parts: None,
            }),
        )
        .await;
        assert_eq!(detail.status(), StatusCode::OK);
        let detail_bytes = to_bytes(detail.into_body(), 1024 * 1024)
            .await
            .expect("detail body");
        let detail_payload: serde_json::Value =
            serde_json::from_slice(&detail_bytes).expect("detail json");
        assert_eq!(detail_payload["active_workspace"], workspace);
        assert_eq!(
            detail_payload["ticket"]["ticket_ref"]["workspace"],
            workspace
        );

        let description_response = get_ticket_description(
            State(state.clone()),
            Extension(RequestIdExt(format!("rid-description-{workspace}"))),
            Path(id),
            Query(TicketIdParam {
                workspace: workspace.to_string(),
                view: None,
                parts: None,
            }),
        )
        .await;
        assert_eq!(description_response.status(), StatusCode::OK);
        let description_bytes = to_bytes(description_response.into_body(), 1024 * 1024)
            .await
            .expect("description body");
        let description_payload: serde_json::Value =
            serde_json::from_slice(&description_bytes).expect("description json");
        assert_eq!(description_payload["active_workspace"], workspace);
        assert_eq!(description_payload["ticket_ref"]["workspace"], workspace);
        assert_eq!(description_payload["description"], expected_description);

        let history = get_ticket_history(
            State(state.clone()),
            Extension(RequestIdExt(format!("rid-history-{workspace}"))),
            Path(id),
            Query(TicketIdParam {
                workspace: workspace.to_string(),
                view: None,
                parts: None,
            }),
        )
        .await;
        assert_eq!(history.status(), StatusCode::OK);
        let history_bytes = to_bytes(history.into_body(), 1024 * 1024)
            .await
            .expect("history body");
        let history_payload: serde_json::Value =
            serde_json::from_slice(&history_bytes).expect("history json");
        assert_eq!(history_payload["active_workspace"], workspace);
        assert_eq!(history_payload["ticket_ref"]["workspace"], workspace);

        let files = list_ticket_files(
            State(state.clone()),
            Extension(RequestIdExt(format!("rid-files-{workspace}"))),
            Path(id),
            Query(TicketIdParam {
                workspace: workspace.to_string(),
                view: None,
                parts: None,
            }),
        )
        .await;
        assert_eq!(files.status(), StatusCode::OK);
        let files_bytes = to_bytes(files.into_body(), 1024 * 1024)
            .await
            .expect("files body");
        let files_payload: serde_json::Value =
            serde_json::from_slice(&files_bytes).expect("files json");
        assert_eq!(files_payload["active_workspace"], workspace);
        assert_eq!(files_payload["ticket_ref"]["workspace"], workspace);

        let asset = get_ticket_asset(
            State(state.clone()),
            Extension(RequestIdExt(format!("rid-asset-{workspace}"))),
            Path(id),
            Query(TicketAssetParam {
                workspace: workspace.to_string(),
                path: "assets/plan.md".to_string(),
            }),
        )
        .await;
        assert_eq!(asset.status(), StatusCode::OK);
        let asset_bytes = to_bytes(asset.into_body(), 1024 * 1024)
            .await
            .expect("asset body");
        let asset_payload: serde_json::Value =
            serde_json::from_slice(&asset_bytes).expect("asset json");
        assert_eq!(asset_payload["active_workspace"], workspace);
        assert_eq!(asset_payload["ticket_ref"]["workspace"], workspace);
        assert_eq!(asset_payload["content"], expected_asset);
    }
}
