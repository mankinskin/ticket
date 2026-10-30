#[allow(unused_imports)]
use super::*;

#[tokio::test]
pub(super) async fn legacy_workspace_label_collision_returns_typed_bad_request() {
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
        .create(
            None,
            "tracker-improvement",
            Some("left shared ticket"),
            Some("planning"),
            BTreeMap::new(),
            None,
            None,
        )
        .expect("create left ticket");
    right_store
        .create(
            None,
            "tracker-improvement",
            Some("right shared ticket"),
            Some("planning"),
            BTreeMap::new(),
            None,
            None,
        )
        .expect("create right ticket");

    parent_store
        .add_scan_root(ticket_api::model::filesystem::ScanRoot {
            path: left_index_root.join("tickets"),
            label: "tickets".to_string(),
        })
        .expect("add left scan root");
    parent_store
        .add_scan_root(ticket_api::model::filesystem::ScanRoot {
            path: right_index_root.join("tickets"),
            label: "tickets".to_string(),
        })
        .expect("add right scan root");
    parent_store.scan(true).expect("scan parent store");

    let state = make_state(parent_store);
    let response = list_tickets(
        State(state),
        Extension(RequestIdExt("rid-ambiguous-label".to_string())),
        Query(WorkspaceParam {
            workspace: "shared".to_string(),
            state: None,
            query: None,
            limit: Some(10),
            cursor: None,
        }),
    )
    .await;

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let bytes = to_bytes(response.into_body(), 1024 * 1024)
        .await
        .expect("read body");
    let payload: serde_json::Value = serde_json::from_slice(&bytes).expect("json body");

    assert_eq!(payload["code"], "workspace.ambiguous_label");
    assert_eq!(payload["details"]["requested"], "shared");
    assert_eq!(
        payload["details"]["matches"]
            .as_array()
            .expect("matches array")
            .len(),
        2
    );
}

#[tokio::test]
pub(super) async fn unique_display_workspace_label_returns_typed_bad_request() {
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

    let _id = child_store
        .create(
            None,
            "tracker-improvement",
            Some("legacy workspace alias ticket"),
            Some("planning"),
            BTreeMap::new(),
            None,
            None,
        )
        .expect("create child ticket");

    parent_store
        .add_scan_root(ticket_api::model::filesystem::ScanRoot {
            path: child_index_root.join("tickets"),
            label: "tickets".to_string(),
        })
        .expect("add child scan root to parent");
    parent_store.scan(true).expect("scan parent store");

    let state = make_state(parent_store);
    let canonical_workspace = state
        .registry
        .workspace_infos()
        .into_iter()
        .find(|info| info.label == "child")
        .expect("child workspace info")
        .name;

    let response = list_tickets(
        State(state),
        Extension(RequestIdExt("rid-legacy-alias".to_string())),
        Query(WorkspaceParam {
            workspace: "child".to_string(),
            state: None,
            query: Some("legacy workspace alias".to_string()),
            limit: Some(10),
            cursor: None,
        }),
    )
    .await;

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let bytes = to_bytes(response.into_body(), 1024 * 1024)
        .await
        .expect("read body");
    let payload: serde_json::Value = serde_json::from_slice(&bytes).expect("json body");

    assert_eq!(payload["code"], "workspace.display_label_not_allowed");
    assert_eq!(payload["details"]["requested"], "child");
    assert_eq!(
        payload["details"]["canonical"],
        canonical_workspace.as_str()
    );
    assert!(
        payload["message"]
            .as_str()
            .expect("message")
            .contains(canonical_workspace.as_str())
    );
}
