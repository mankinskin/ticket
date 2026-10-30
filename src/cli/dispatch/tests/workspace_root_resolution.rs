#[allow(unused_imports)]
use super::*;

#[test]
pub(super) fn resolve_index_root_prefers_explicit_workspace_root() {
    let dir = tempdir().unwrap();
    let repo = dir.path().join("repo");
    let child = repo.join("memory-api");
    std::fs::create_dir_all(repo.join(".workflow-tools").join("ticket")).unwrap();
    std::fs::create_dir_all(child.join(".workflow-tools").join("ticket")).unwrap();

    let resolved = resolve_index_root_from(None, Some(&child), None, Some(&repo)).unwrap();

    assert_eq!(resolved, child.join(".workflow-tools").join("ticket"));
}

#[test]
pub(super) fn create_rejects_ambient_workspace_aliases_before_store_open() {
    for selector in ["", "  ", "default", ".."] {
        let workspace = PathBuf::from(selector);
        let result = dispatch(
            TicketCommandCli::Create(crate::cli::CreateArgs {
                id: None,
                ticket_type: Some("task".to_string()),
                title: Some("Selector validation".to_string()),
                state: None,
                fields: vec![],
                body_file: None,
            }),
            None,
            Some(&workspace),
            None,
            false,
            false,
        );

        let error = result.expect_err("ambient selector must fail");
        assert!(error.to_string().contains("invalid workspace selector"));
    }
}

#[test]
pub(super) fn init_requires_explicit_workspace_or_store_root() {
    let error = dispatch(TicketCommandCli::Init, None, None, None, false, false)
        .expect_err("init must not select an ambient workspace");

    assert!(
        error
            .to_string()
            .contains("store initialization requires explicit")
    );
}

#[test]
pub(super) fn resolve_workspace_root_keeps_explicit_nested_workspace_bounded() {
    let dir = tempdir().unwrap();
    let repo = dir.path().join("repo");
    let child = repo.join("file-combiner");
    std::fs::create_dir_all(repo.join(".workflow-tools").join("ticket")).unwrap();
    std::fs::create_dir_all(&child).unwrap();

    let resolved =
        resolve_workspace_root(&repo.join(".workflow-tools").join("ticket"), Some(&child));

    assert_eq!(resolved, child);
}

#[test]
pub(super) fn resolve_index_root_prefers_explicit_index_root_over_workspace_root() {
    let dir = tempdir().unwrap();
    let repo = dir.path().join("repo");
    let child = repo.join("memory-api");
    std::fs::create_dir_all(repo.join(".workflow-tools").join("ticket")).unwrap();
    std::fs::create_dir_all(child.join(".workflow-tools").join("ticket")).unwrap();

    let resolved = resolve_index_root_from(
        Some(&repo.join(".workflow-tools").join("ticket")),
        Some(&child),
        None,
        Some(&repo),
    )
    .unwrap();

    assert_eq!(resolved, repo.join(".workflow-tools").join("ticket"));
}

#[test]
pub(super) fn dispatch_explicit_index_root_overrides_workspace_root() {
    let dir = tempdir().unwrap();
    let repo = dir.path().join("repo");
    let child = repo.join("memory-api");
    std::fs::create_dir_all(&child).unwrap();
    let root_store = TicketStore::init(&repo).unwrap();
    let child_store = TicketStore::init(&child).unwrap();
    let ticket_id = Uuid::new_v4();
    root_store
        .create(
            Some(ticket_id),
            "tracker-improvement",
            Some("Root ticket"),
            None,
            BTreeMap::new(),
            None,
            None,
        )
        .unwrap();
    child_store
        .create(
            Some(ticket_id),
            "tracker-improvement",
            Some("Child ticket"),
            None,
            BTreeMap::new(),
            None,
            None,
        )
        .unwrap();

    let payload = dispatch(
        TicketCommandCli::Get(IdArgs {
            id: ticket_id.to_string(),
            view: None,
            parts: None,
        }),
        Some(&root_store.index_root),
        Some(&child),
        None,
        true,
        false,
    )
    .unwrap();

    assert_eq!(payload["ticket"]["fields"]["title"], "Root ticket");
}

#[test]
pub(super) fn dispatch_workspace_roots_and_prune_roots_return_machine_output() {
    let dir = tempdir().unwrap();
    let store = TicketStore::init(dir.path()).unwrap();

    let roots = dispatch_store_command(
        TicketCommandCli::Workspace(WorkspaceArgs {
            command: WorkspaceCommand::Roots,
        }),
        store,
        false,
    )
    .unwrap();
    assert_eq!(roots["command"], "workspace_roots");
    assert_eq!(roots["status"], "ok");
    assert!(!roots["roots"].as_array().unwrap().is_empty());

    let store = TicketStore::open(dir.path()).unwrap();
    let pruned = dispatch_store_command(
        TicketCommandCli::Workspace(WorkspaceArgs {
            command: WorkspaceCommand::PruneRoots,
        }),
        store,
        false,
    )
    .unwrap();
    assert_eq!(pruned["command"], "workspace_prune_roots");
    assert_eq!(pruned["pruned"], 0);
}

#[test]
pub(super) fn resolve_index_root_preserves_relative_explicit_index_root() {
    let dir = tempdir().unwrap();
    let repo = dir.path().join("repo");
    let child = repo.join("memory-api");
    std::fs::create_dir_all(repo.join(".workflow-tools").join("ticket")).unwrap();
    std::fs::create_dir_all(child.join(".workflow-tools").join("ticket")).unwrap();

    let resolved = resolve_index_root_from(
        Some(Path::new(".workflow-tools/ticket")),
        Some(&child),
        None,
        Some(&repo),
    )
    .unwrap();

    assert_eq!(resolved, repo.join(".workflow-tools").join("ticket"));
}
