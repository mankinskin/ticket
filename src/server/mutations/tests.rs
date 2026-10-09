use super::*;

#[test]
fn move_target_normalization_rejects_ambient_aliases() {
    for selector in ["", "  ", "default", ".."] {
        assert!(normalize_workspace_root(selector).is_err());
    }
}

#[tokio::test]
async fn ticket_mutation_store_rejects_ambient_workspace_aliases_before_init() {
    let temp = tempfile::tempdir().unwrap();
    let server = TicketServer::new(temp.path().join(".workflow-tools/ticket"));

    for selector in ["", "  ", "default", ".."] {
        let result = server.with_store_ext(selector, |_| Ok(())).await;
        assert!(result.is_err(), "selector {selector:?} must be rejected");
        assert!(!temp.path().join(".workflow-tools/ticket").exists());
    }
}

#[tokio::test]
async fn explicit_dot_workspace_creates_and_reads_back_in_current_workspace() {
    const CHILD_ENV: &str = "TICKET_MCP_DOT_SELECTOR_CHILD";
    if std::env::var_os(CHILD_ENV).is_none() {
        let temp = tempfile::tempdir().unwrap();
        let parent = temp.path().join("parent");
        let selected = parent.join("selected");
        let sibling = parent.join("sibling");
        std::fs::create_dir_all(&selected).unwrap();
        std::fs::create_dir_all(&sibling).unwrap();

        let output = std::process::Command::new(std::env::current_exe().unwrap())
            .arg("explicit_dot_workspace_creates_and_reads_back_in_current_workspace")
            .arg("--nocapture")
            .env(CHILD_ENV, "1")
            .current_dir(&selected)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "child regression failed: {} {}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );

        let store_root = ticket_api::workspace::canonical_store_root(&selected, ".ticket");
        let result = std::fs::read_dir(store_root.join("tickets"))
            .unwrap()
            .next()
            .expect("ticket entry")
            .unwrap()
            .file_name()
            .to_string_lossy()
            .parse::<Uuid>()
            .unwrap();
        let store = TicketStore::open_or_init(&store_root).unwrap();
        let ticket = store
            .get_indexed(&result)
            .unwrap()
            .expect("ticket in selected canonical store");
        assert_eq!(ticket.title.as_deref(), Some("Dot selector read-back"));
        assert!(!parent.join(".workflow-tools/ticket").exists());
        assert!(!sibling.join(".workflow-tools/ticket").exists());
        return;
    }

    let selected = std::env::current_dir().unwrap();
    let sibling_root = selected
        .parent()
        .unwrap()
        .join("sibling/.workflow-tools/ticket");
    let server = TicketServer::new(sibling_root);
    let result = server
        .create_ticket_tool(CreateTicketInput {
            workspace: ".".to_string(),
            type_id: "task".to_string(),
            title: Some("Dot selector read-back".to_string()),
            state: None,
            fields: vec![],
            description: None,
        })
        .await
        .expect("create in selected workspace");
    assert!(!result.is_error.unwrap_or(false));
    let text = result
        .content
        .iter()
        .find_map(|content| match &content.raw {
            rmcp::model::RawContent::Text(text) => Some(text.text.as_str()),
            _ => None,
        })
        .expect("ticket create response");
    let response: serde_json::Value = serde_json::from_str(text).unwrap();
    assert_eq!(response["workspace"], selected.to_string_lossy().as_ref());
}

#[tokio::test]
async fn create_ticket_tool_rejects_ambient_workspace_aliases() {
    let tmp = tempfile::TempDir::new().expect("tempdir");
    let server = TicketServer::new(tmp.path().to_path_buf());

    for selector in ["", "  ", "default", ".."] {
        let result = server
            .create_ticket_tool(CreateTicketInput {
                workspace: selector.to_string(),
                type_id: "tracker-improvement".to_string(),
                title: Some("Wrong workspace".to_string()),
                state: None,
                fields: vec![],
                description: None,
            })
            .await;

        assert!(result.is_err());
        assert!(!tmp.path().join(".workflow-tools").exists());
    }
}
