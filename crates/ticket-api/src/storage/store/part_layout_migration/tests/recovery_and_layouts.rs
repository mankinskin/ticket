#[allow(unused_imports)]
use super::*;

#[test]
pub(super) fn partial_apply_report_names_completed_and_unapplied_tickets() {
    let (_directory, store) = setup_store();
    let (first_id, _, _, _) = create_ticket_with_parts(
        &store,
        "First partial ticket",
        &[("design", "first", false, false)],
    );
    let (second_id, second_path, _, _) = create_ticket_with_parts(
        &store,
        "Second partial ticket",
        &[("review", "second", false, false)],
    );
    let plan = store.part_layout_dry_run().unwrap();
    let last = plan
        .tickets
        .iter()
        .filter(|ticket| ticket.status == PartLayoutTicketStatus::Planned)
        .last()
        .unwrap();
    let stale_id = last.id;
    let stale_path = if stale_id == first_id {
        store.get_indexed(&first_id).unwrap().unwrap().path
    } else {
        second_path.clone()
    };
    fs::write(
        stale_path.join(&last.parts[0].part.path),
        b"changed after dry-run",
    )
    .unwrap();

    let apply = store.part_layout_apply(&plan).unwrap();
    assert!(apply.failure.is_some());
    assert_eq!(apply.completed_tickets.len(), 1);
    assert_eq!(apply.unapplied_ticket_ids, vec![stale_id]);
    assert_ne!(apply.completed_tickets[0].id, stale_id);

    let original_content = if stale_id == first_id {
        "first"
    } else {
        assert_eq!(stale_id, second_id);
        "second"
    };
    fs::write(
        stale_path.join(&last.parts[0].part.path),
        original_content.as_bytes(),
    )
    .unwrap();
    let resumed = store.part_layout_apply(&plan).unwrap();
    assert!(resumed.failure.is_none(), "{:?}", resumed.failure);
    assert_eq!(resumed.completed_tickets.len(), 2);
    assert!(resumed.unapplied_ticket_ids.is_empty());
    assert!(store.part_layout_verify(&resumed).unwrap().passed);
}

#[test]
pub(super) fn undo_refuses_to_overwrite_later_part_edits() {
    let (_directory, store) = setup_store();
    let (id, ticket_path, _, _) = create_ticket_with_parts(
        &store,
        "Undo conflict",
        &[("design", "payload", false, false)],
    );
    let plan = store.part_layout_dry_run().unwrap();
    let apply = store.part_layout_apply(&plan).unwrap();
    assert!(apply.failure.is_none());
    fs::write(ticket_path.join("parts/design.md"), b"later edit").unwrap();

    let undo = store.part_layout_undo(&apply).unwrap();
    assert!(undo.failure.as_deref().unwrap().contains("refusing undo"));
    assert!(undo.restored_ticket_ids.is_empty());
    assert_eq!(undo.not_restored_ticket_ids, vec![id]);
    assert_eq!(
        fs::read(ticket_path.join("parts/design.md")).unwrap(),
        b"later edit"
    );
}

#[test]
pub(super) fn undo_is_retry_safe_after_the_original_state_was_restored() {
    let (_directory, store) = setup_store();
    let (id, ticket_path, _, original_manifest) = create_ticket_with_parts(
        &store,
        "Retry-safe undo",
        &[("design", "payload", false, false)],
    );
    let plan = store.part_layout_dry_run().unwrap();
    let apply = store.part_layout_apply(&plan).unwrap();
    assert!(apply.failure.is_none());

    let first_undo = store.part_layout_undo(&apply).unwrap();
    assert!(first_undo.failure.is_none());
    assert_eq!(first_undo.restored_ticket_ids, vec![id]);

    let retried_undo = store.part_layout_undo(&apply).unwrap();
    assert!(retried_undo.failure.is_none());
    assert_eq!(retried_undo.restored_ticket_ids, vec![id]);
    assert!(retried_undo.not_restored_ticket_ids.is_empty());
    assert!(retried_undo.recovery_required_ticket_ids.is_empty());
    assert_eq!(
        fs::read_to_string(ticket_path.join("ticket.toml")).unwrap(),
        original_manifest
    );
}

#[test]
pub(super) fn apply_rejects_a_modified_saved_plan() {
    let (_directory, store) = setup_store();
    let (id, ticket_path, _, _) = create_ticket_with_parts(
        &store,
        "Modified plan",
        &[("design", "payload", false, false)],
    );
    let mut plan = store.part_layout_dry_run().unwrap();
    let ticket_plan = plan
        .tickets
        .iter_mut()
        .find(|ticket| ticket.id == id)
        .unwrap();
    ticket_plan.path_changes[0].to = "parts/attacker-selected.md".to_string();

    let apply = store.part_layout_apply(&plan).unwrap();
    assert!(apply.failure.as_deref().unwrap().contains("modified"));
    assert!(apply.completed_tickets.is_empty());
    assert!(
        ticket_path
            .join(&plan.tickets[0].parts[0].part.path)
            .exists()
    );
    assert!(!ticket_path.join("parts/attacker-selected.md").exists());
}

#[test]
pub(super) fn migration_handles_root_and_nested_workspace_store_layouts() {
    let directory = tempfile::tempdir().unwrap();
    let root_workspace = directory.path();
    let nested_workspace = root_workspace.join("workflow-tools").join("ticket");
    fs::create_dir_all(&nested_workspace).unwrap();

    for (workspace, title) in [
        (root_workspace, "Root-layout ticket"),
        (nested_workspace.as_path(), "Nested-layout ticket"),
    ] {
        let store = TicketStore::init_in_workspace(workspace).unwrap();
        let (id, _, _, _) =
            create_ticket_with_parts(&store, title, &[("design", "payload", false, false)]);
        let plan = store.part_layout_dry_run().unwrap();
        assert_eq!(plan.planned_tickets, 1);
        assert_eq!(plan.tickets.len(), 1);
        assert_eq!(plan.tickets[0].id, id);
        let apply = store.part_layout_apply(&plan).unwrap();
        assert!(apply.failure.is_none());
        assert!(store.part_layout_verify(&apply).unwrap().passed);
        assert!(store.part_layout_undo(&apply).unwrap().failure.is_none());
    }
}

#[test]
pub(super) fn dry_run_does_not_create_locks_or_transaction_artifacts() {
    let (_directory, store) = setup_store();
    let id = store
        .create(
            None,
            "tracker-improvement",
            Some("Read-only dry-run"),
            Some("planning"),
            Default::default(),
            None,
            None,
        )
        .unwrap();
    let ticket_path = store.get_indexed(&id).unwrap().unwrap().path;
    let manifest_before = fs::read(ticket_path.join("ticket.toml")).unwrap();
    let lock_before = ticket_path.join(".ticket-lock").exists();
    let generation_before = ticket_path.join(".ticket-generation").exists();
    let transaction_before = ticket_path.join(".ticket-transaction.json").exists();

    let report = store.part_layout_dry_run().unwrap();

    assert_eq!(report.scanned_tickets, 1);
    assert_eq!(
        fs::read(ticket_path.join("ticket.toml")).unwrap(),
        manifest_before
    );
    assert_eq!(ticket_path.join(".ticket-lock").exists(), lock_before);
    assert_eq!(
        ticket_path.join(".ticket-generation").exists(),
        generation_before
    );
    assert_eq!(
        ticket_path.join(".ticket-transaction.json").exists(),
        transaction_before
    );
}
