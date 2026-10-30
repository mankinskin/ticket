#[allow(unused_imports)]
use super::*;

#[test]
pub(super) fn migration_plans_and_saved_reports_are_limited_to_the_selected_store() {
    let directory = tempfile::tempdir().unwrap();
    let root_workspace = directory.path().join("repo");
    let nested_workspace = root_workspace.join("workflow-tools").join("ticket");
    fs::create_dir_all(&nested_workspace).unwrap();

    let root_store = TicketStore::init(&root_workspace).unwrap();
    let nested_store = TicketStore::init(&nested_workspace).unwrap();
    let (root_id, root_ticket_path, root_part_ids, _) = create_ticket_with_parts(
        &root_store,
        "Root store ticket",
        &[("design", "root content", false, false)],
    );
    let (nested_id, nested_ticket_path, nested_part_ids, _) = create_ticket_with_parts(
        &nested_store,
        "Nested store ticket",
        &[("design", "nested content", false, false)],
    );

    root_store
        .add_scan_root(ScanRoot {
            path: nested_store.index_root.join("tickets"),
            label: "nested store".to_string(),
        })
        .unwrap();
    nested_store
        .add_scan_root(ScanRoot {
            path: root_store.index_root.join("tickets"),
            label: "root store".to_string(),
        })
        .unwrap();
    root_store.scan(true).unwrap();
    nested_store.scan(true).unwrap();

    assert!(
        root_store
            .list(None, None, None)
            .unwrap()
            .iter()
            .any(|ticket| ticket.id == nested_id)
    );
    assert!(
        nested_store
            .list(None, None, None)
            .unwrap()
            .iter()
            .any(|ticket| ticket.id == root_id)
    );

    let root_manifest_path = root_ticket_path.join("ticket.toml");
    let root_part_path = root_ticket_path
        .join("parts")
        .join(format!("{}.md", root_part_ids[0]));
    let nested_manifest_path = nested_ticket_path.join("ticket.toml");
    let nested_part_path = nested_ticket_path
        .join("parts")
        .join(format!("{}.md", nested_part_ids[0]));
    let root_before = (
        fs::read(&root_manifest_path).unwrap(),
        fs::read(&root_part_path).unwrap(),
    );
    let nested_before = (
        fs::read(&nested_manifest_path).unwrap(),
        fs::read(&nested_part_path).unwrap(),
    );

    let root_plan = root_store.part_layout_dry_run().unwrap();
    let nested_plan = nested_store.part_layout_dry_run().unwrap();
    assert_eq!(root_plan.scanned_tickets, 1);
    assert_eq!(nested_plan.scanned_tickets, 1);
    assert_eq!(root_plan.tickets[0].id, root_id);
    assert_eq!(nested_plan.tickets[0].id, nested_id);

    for (store, plan) in [(&root_store, &root_plan), (&nested_store, &nested_plan)] {
        let tickets_root = fs::canonicalize(store.index_root.join("tickets"))
            .unwrap()
            .to_string_lossy()
            .replace('\\', "/");
        let prefix = format!("{}/", tickets_root.trim_end_matches('/'));
        assert!(
            plan.tickets
                .iter()
                .all(|ticket| ticket.ticket_path.starts_with(&prefix))
        );
    }

    let mut foreign_plan = nested_plan.clone();
    foreign_plan.store_root = root_plan.store_root.clone();
    let apply_error = root_store
        .part_layout_apply(&foreign_plan)
        .unwrap_err()
        .to_string();
    assert!(apply_error.contains("outside selected store tickets directory"));

    let foreign_apply = PartLayoutApplyReport {
        version: foreign_plan.version,
        apply_id: Uuid::new_v4(),
        plan_id: foreign_plan.plan_id,
        store_root: root_plan.store_root,
        completed_tickets: foreign_plan.tickets,
        unapplied_ticket_ids: Vec::new(),
        recovery_required_ticket_ids: Vec::new(),
        failure: None,
    };
    assert!(
        root_store
            .part_layout_verify(&foreign_apply)
            .unwrap_err()
            .to_string()
            .contains("outside selected store tickets directory")
    );
    assert!(
        root_store
            .part_layout_undo(&foreign_apply)
            .unwrap_err()
            .to_string()
            .contains("outside selected store tickets directory")
    );

    assert_eq!(fs::read(root_manifest_path).unwrap(), root_before.0);
    assert_eq!(fs::read(root_part_path).unwrap(), root_before.1);
    assert_eq!(fs::read(nested_manifest_path).unwrap(), nested_before.0);
    assert_eq!(fs::read(nested_part_path).unwrap(), nested_before.1);
}

#[test]
pub(super) fn dry_run_apply_verify_and_undo_preserve_metadata_and_excluded_files() {
    let (_directory, store) = setup_store();
    let (id, ticket_path, ids, original_manifest) = create_ticket_with_parts(
        &store,
        "Part layout migration",
        &[
            ("design", "design payload", true, false),
            ("design", "empty eligible part", true, true),
            ("design", "second design payload", true, false),
            ("review", "empty non-planning part", true, true),
            ("examples", "empty unfrozen part", false, true),
        ],
    );
    let orphan_path = ticket_path.join("parts/orphan.md");
    fs::write(&orphan_path, b"orphan payload").unwrap();
    let original_parts = TicketFs::read(&ticket_path).unwrap().parts();

    let plan = store.part_layout_dry_run().unwrap();
    let ticket_plan = plan.tickets.iter().find(|ticket| ticket.id == id).unwrap();
    assert_eq!(ticket_plan.status, PartLayoutTicketStatus::Planned);
    assert_eq!(ticket_plan.cleanup_candidates.len(), 1);
    assert_eq!(ticket_plan.cleanup_candidates[0].part.id, ids[1]);
    assert_eq!(ticket_plan.path_changes.len(), 4);
    assert_eq!(
        ticket_plan
            .path_changes
            .iter()
            .map(|change| change.to.as_str())
            .collect::<Vec<_>>(),
        vec![
            "parts/design.md",
            "parts/design-2.md",
            "parts/review.md",
            "parts/examples.md",
        ],
        "{:#?}",
        ticket_plan.path_changes
    );
    assert_eq!(ticket_plan.normalized_base_collisions.len(), 1);
    assert_eq!(
        ticket_plan.normalized_base_collisions[0].normalized_base,
        "design"
    );
    assert_eq!(ticket_plan.preservation.len(), 6);
    assert!(
        ticket_plan
            .preservation
            .iter()
            .all(|file| file.size.is_some() && file.sha256.is_some())
    );
    assert!(
        ticket_plan
            .skipped_parts
            .iter()
            .any(|skip| skip.part_id == ids[3] && skip.reason.contains("not a planning kind"))
    );
    assert!(
        ticket_plan
            .skipped_parts
            .iter()
            .any(|skip| skip.part_id == ids[4] && skip.reason.contains("not frozen"))
    );
    assert_eq!(
        fs::read_to_string(ticket_path.join("ticket.toml")).unwrap(),
        original_manifest
    );
    assert_eq!(fs::read(&orphan_path).unwrap(), b"orphan payload");

    let apply = store.part_layout_apply(&plan).unwrap();
    assert!(apply.failure.is_none(), "{:?}", apply.failure);
    assert_eq!(apply.completed_tickets.len(), 1);
    assert!(store.part_layout_verify(&apply).unwrap().passed);

    let migrated = TicketFs::read(&ticket_path).unwrap();
    let migrated_parts = migrated.parts();
    assert_eq!(migrated_parts, ticket_plan.parts_after);
    assert_eq!(
        migrated_parts
            .iter()
            .map(|part| part.id)
            .collect::<Vec<_>>(),
        vec![ids[0], ids[2], ids[3], ids[4]]
    );
    assert_eq!(migrated_parts[0].path, "parts/design.md");
    assert_eq!(migrated_parts[1].path, "parts/design-2.md");
    assert_eq!(migrated_parts[2].path, "parts/review.md");
    assert_eq!(migrated_parts[3].path, "parts/examples.md");
    assert!(migrated_parts[0].frozen);
    assert!(migrated_parts[1].frozen);
    assert!(migrated_parts[2].frozen);
    assert!(!migrated_parts[3].frozen);
    assert!(!ticket_path.join(format!("parts/{}.md", ids[1])).exists());
    assert_eq!(
        fs::read(ticket_path.join(&migrated_parts[0].path)).unwrap(),
        b"design payload"
    );
    assert_eq!(
        fs::read(ticket_path.join(&migrated_parts[1].path)).unwrap(),
        b"second design payload"
    );
    assert_eq!(
        fs::read(ticket_path.join(&migrated_parts[2].path)).unwrap(),
        b""
    );
    assert_eq!(fs::read(&orphan_path).unwrap(), b"orphan payload");

    let undo = store.part_layout_undo(&apply).unwrap();
    assert!(undo.failure.is_none(), "{:?}", undo.failure);
    assert_eq!(undo.restored_ticket_ids, vec![id]);
    assert_eq!(
        fs::read_to_string(ticket_path.join("ticket.toml")).unwrap(),
        original_manifest
    );
    assert_eq!(
        TicketFs::read(&ticket_path).unwrap().parts(),
        original_parts
    );
    for part in original_parts {
        let expected = match part.id {
            part_id if part_id == ids[0] => b"design payload".as_slice(),
            part_id if part_id == ids[2] => b"second design payload".as_slice(),
            _ => b"".as_slice(),
        };
        assert_eq!(fs::read(ticket_path.join(part.path)).unwrap(), expected);
    }
    assert_eq!(fs::read(&orphan_path).unwrap(), b"orphan payload");
}

#[test]
pub(super) fn apply_rejects_stale_files_without_publishing_paths() {
    let (_directory, store) = setup_store();
    let (id, ticket_path, ids, _) =
        create_ticket_with_parts(&store, "Stale plan", &[("design", "payload", false, false)]);
    let orphan_path = ticket_path.join("parts/orphan.md");
    fs::write(&orphan_path, b"orphan before").unwrap();
    let plan = store.part_layout_dry_run().unwrap();
    let ticket_plan = plan.tickets.iter().find(|ticket| ticket.id == id).unwrap();
    fs::write(
        ticket_path.join(&ticket_plan.parts[0].part.path),
        b"changed",
    )
    .unwrap();
    fs::write(&orphan_path, b"orphan changed").unwrap();

    let apply = store.part_layout_apply(&plan).unwrap();
    assert!(apply.failure.as_deref().unwrap().contains("stale"));
    assert!(apply.completed_tickets.is_empty());
    assert_eq!(apply.unapplied_ticket_ids, vec![id]);
    assert!(ticket_path.join(&ticket_plan.parts[0].part.path).exists());
    assert!(!ticket_path.join("parts/design.md").exists());
    assert_eq!(fs::read(&orphan_path).unwrap(), b"orphan changed");
    assert_eq!(ids.len(), 1);
}

#[test]
pub(super) fn dry_run_reports_foreign_slug_collision_and_never_overwrites_it() {
    let (_directory, store) = setup_store();
    let (id, ticket_path, _, _) = create_ticket_with_parts(
        &store,
        "Foreign collision",
        &[("design", "payload", false, false)],
    );
    let orphan_path = ticket_path.join("parts/design.md");
    fs::write(&orphan_path, b"foreign payload").unwrap();

    let plan = store.part_layout_dry_run().unwrap();
    let ticket_plan = plan.tickets.iter().find(|ticket| ticket.id == id).unwrap();
    assert_eq!(ticket_plan.status, PartLayoutTicketStatus::Conflict);
    assert!(
        ticket_plan
            .conflicts
            .iter()
            .any(|conflict| conflict.contains("orphan"))
    );

    let apply = store.part_layout_apply(&plan).unwrap();
    assert!(apply.failure.is_some());
    assert!(apply.completed_tickets.is_empty());
    assert_eq!(fs::read(orphan_path).unwrap(), b"foreign payload");
}
