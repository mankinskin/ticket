#[allow(unused_imports)]
use super::*;

#[tokio::test]
pub(super) async fn list_rejects_synthetic_or_unknown_public_workspace_identifiers() {
    let dir = tempfile::tempdir().expect("tempdir");
    let store = make_store(dir.path());
    store
        .create(
            None,
            "tracker-improvement",
            Some("workspace alias rejection regression"),
            Some("planning"),
            BTreeMap::new(),
            None,
            None,
        )
        .expect("create ticket");

    let state = make_state(Arc::clone(&store));

    for workspace in ["default", "..", "../..", "missing-workspace"] {
        let response = list_tickets(
            State(state.clone()),
            Extension(RequestIdExt(format!("rid-{workspace}"))),
            Query(WorkspaceParam {
                workspace: workspace.to_string(),
                state: None,
                query: None,
                limit: Some(10),
                cursor: None,
            }),
        )
        .await;

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        let bytes = to_bytes(response.into_body(), 1024 * 1024)
            .await
            .expect("read body");
        let payload: serde_json::Value = serde_json::from_slice(&bytes).expect("json body");

        assert_eq!(payload["code"], "not_found");
        assert!(
            payload["message"]
                .as_str()
                .expect("message")
                .contains("workspace")
        );
        assert!(payload.get("request_id").is_some());
    }
}
