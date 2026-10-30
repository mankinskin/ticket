#[allow(unused_imports)]
use super::*;

#[test]
pub(super) fn move_rewrites_skip_generated_store_indexes_and_journals() {
    let temp = tempdir().unwrap();
    let repo = temp.path().join("repo");
    let nested_repo = repo.join("nested-repo");
    std::fs::create_dir_all(&repo).unwrap();
    std::fs::create_dir_all(&nested_repo).unwrap();
    run_git(&repo, &["init"]);
    run_git(&nested_repo, &["init"]);

    let source_store = TicketStore::init(&repo).unwrap();
    let _target_store = TicketStore::init(&nested_repo).unwrap();

    let id = source_store
        .create(
            None,
            "tracker-improvement",
            Some("move me"),
            Some("planning"),
            Default::default(),
            None,
            None,
        )
        .unwrap();
    let related = source_store
        .create(
            None,
            "tracker-improvement",
            Some("related ticket"),
            Some("planning"),
            Default::default(),
            None,
            None,
        )
        .unwrap();

    let mut plan = source_store.plan_move_preflight(&id, &nested_repo).unwrap();
    let source_rel = plan
        .source_entity_path
        .strip_prefix(&repo)
        .unwrap()
        .to_string_lossy()
        .replace('\\', "/");
    let source_abs = plan.source_entity_path.to_string_lossy().replace('\\', "/");

    let persistent_doc = source_store
        .index_root
        .join("tickets")
        .join(related.to_string())
        .join("description.md");
    std::fs::create_dir_all(persistent_doc.parent().unwrap()).unwrap();
    std::fs::write(
        &persistent_doc,
        format!("persistent rel ref: {source_rel}\npersistent abs ref: {source_abs}\n"),
    )
    .unwrap();

    let generated_readme = source_store.index_root.join("README.md");
    std::fs::create_dir_all(generated_readme.parent().unwrap()).unwrap();
    std::fs::write(&generated_readme, format!("generated ref: {source_rel}\n")).unwrap();

    let generated_index = source_store.index_root.join("index.toon");
    std::fs::write(&generated_index, format!("source_path: \"{source_rel}\"\n")).unwrap();

    let generated_journal = source_store
        .index_root
        .join("move-journals")
        .join("existing.json");
    std::fs::create_dir_all(generated_journal.parent().unwrap()).unwrap();
    std::fs::write(
        &generated_journal,
        format!("{{\"ref\":\"{source_rel}\"}}\n"),
    )
    .unwrap();

    run_git(&repo, &["config", "user.name", "Move Test"]);
    run_git(&repo, &["config", "user.email", "move-test@example.com"]);
    run_git(
        &repo,
        &[
            "add",
            "--",
            &format!(".workflow-tools/ticket/tickets/{}/description.md", related),
            ".workflow-tools/ticket/README.md",
            ".workflow-tools/ticket/index.toon",
            ".workflow-tools/ticket/move-journals/existing.json",
        ],
    );
    run_git(
        &repo,
        &["commit", "-m", "seed persistent and generated refs"],
    );

    plan = source_store.plan_move_preflight(&id, &nested_repo).unwrap();
    assert!(
        !plan
            .path_reference_files
            .iter()
            .any(|path| path == &generated_readme)
    );
    assert!(
        !plan
            .path_reference_files
            .iter()
            .any(|path| path == &generated_index)
    );
    assert!(
        !plan
            .path_reference_files
            .iter()
            .any(|path| path == &generated_journal)
    );
}
