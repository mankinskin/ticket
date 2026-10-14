use std::{fs, path::PathBuf};

use tempfile::TempDir;
use uuid::Uuid;

use crate::{
    model::ticket::TicketManifestExt,
    storage::{TicketStore, ticket_fs::TicketFs},
};

use super::*;

type PartSpec<'a> = (&'a str, &'a str, bool, bool);

fn setup_store() -> (TempDir, TicketStore) {
    let directory = tempfile::tempdir().unwrap();
    let store = TicketStore::init(directory.path()).unwrap();
    (directory, store)
}

fn create_ticket_with_parts(
    store: &TicketStore,
    title: &str,
    specs: &[PartSpec<'_>],
) -> (Uuid, PathBuf, Vec<Uuid>, String) {
    let id = store
        .create(
            None,
            "tracker-improvement",
            Some(title),
            Some("planning"),
            Default::default(),
            None,
            None,
        )
        .unwrap();
    let mut ids = Vec::new();
    for (kind, content, _, _) in specs {
        let part_id = Uuid::new_v4();
        store.write_part(&id, part_id, kind, content, None).unwrap();
        ids.push(part_id);
    }

    let ticket_path = store.get_indexed(&id).unwrap().unwrap().path;
    let mut manifest = TicketFs::read(&ticket_path).unwrap();
    let mut parts = manifest.parts();
    for (index, part) in parts.iter_mut().enumerate() {
        let (kind, _, frozen, empty) = specs[index];
        assert_eq!(part.kind, kind);
        let opaque_path = format!("parts/{}.md", part.id);
        fs::rename(ticket_path.join(&part.path), ticket_path.join(&opaque_path)).unwrap();
        part.path = opaque_path.clone();
        part.frozen = frozen;
        if empty {
            fs::write(ticket_path.join(opaque_path), b"").unwrap();
        }
    }
    manifest.set_parts(parts);
    let manifest_contents = crate::model::manifest_format::format_manifest_toml(&manifest);
    fs::write(
        ticket_path.join("ticket.toml"),
        manifest_contents.as_bytes(),
    )
    .unwrap();
    let original_manifest = fs::read_to_string(ticket_path.join("ticket.toml")).unwrap();
    (id, ticket_path, ids, original_manifest)
}

#[test]
fn dry_run_apply_verify_and_undo_preserve_metadata_and_excluded_files() {
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
fn apply_rejects_stale_files_without_publishing_paths() {
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
fn dry_run_reports_foreign_slug_collision_and_never_overwrites_it() {
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

#[test]
fn partial_apply_report_names_completed_and_unapplied_tickets() {
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
fn undo_refuses_to_overwrite_later_part_edits() {
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
fn undo_is_retry_safe_after_the_original_state_was_restored() {
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
fn apply_rejects_a_modified_saved_plan() {
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
fn migration_handles_root_and_nested_workspace_store_layouts() {
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
fn dry_run_does_not_create_locks_or_transaction_artifacts() {
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
