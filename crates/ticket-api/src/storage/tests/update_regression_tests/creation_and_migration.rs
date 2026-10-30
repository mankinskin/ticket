#[allow(unused_imports)]
use super::*;

#[test]
pub(super) fn create_defaults_to_planning_state() {
    let dir = tempdir().unwrap();
    let store = TicketStore::init(dir.path()).unwrap();

    let id = store
        .create(
            None,
            "tracker-improvement",
            Some("Planning default"),
            None,
            Default::default(),
            None,
            None,
        )
        .unwrap();

    let indexed = store.get_indexed(&id).unwrap().unwrap();
    assert_eq!(indexed.state.as_deref(), Some("planning"));
}

#[test]
pub(super) fn blank_ticket_content_is_rejected_without_mutation() {
    let dir = tempdir().unwrap();
    let store = TicketStore::init(dir.path()).unwrap();
    let rejected_id = Uuid::new_v4();

    assert!(matches!(
        store.create(
            Some(rejected_id),
            "tracker-improvement",
            Some("Blank description"),
            Some("planning"),
            Default::default(),
            None,
            Some(" \n\t "),
        ),
        Err(StorageError::EmptyPartContent { .. })
    ));
    assert!(store.get_indexed(&rejected_id).unwrap().is_none());

    let id = store
        .create(
            None,
            "tracker-improvement",
            Some("Content validation"),
            Some("planning"),
            Default::default(),
            None,
            Some("existing description"),
        )
        .unwrap();
    let indexed = store.get_indexed(&id).unwrap().unwrap();
    let history_before = store.get_history(&id).unwrap().len();
    let original_manifest = store.get(&id).unwrap();

    assert!(matches!(
        store.write_part(&id, Uuid::new_v4(), "review", " \n ", None),
        Err(StorageError::EmptyPartContent { .. })
    ));
    assert!(matches!(
        store.write_amendment_part(&id, Uuid::new_v4(), "\t", Uuid::new_v4(), None),
        Err(StorageError::EmptyPartContent { .. })
    ));
    assert!(matches!(
        store.update_with_options(
            &id,
            BTreeMap::new(),
            None,
            None,
            Some(" \t\n"),
            Some(DescriptionUpdateMode::Replace),
            None,
            false,
        ),
        Err(StorageError::EmptyPartContent { .. })
    ));

    let manifest_after = store.get(&id).unwrap();
    assert_eq!(manifest_after.parts(), original_manifest.parts());
    assert_eq!(
        fs::read_to_string(indexed.path.join("description.md")).unwrap(),
        "existing description"
    );
    assert_eq!(store.get_history(&id).unwrap().len(), history_before);
}

#[test]
pub(super) fn migration_prevalidates_all_segments_before_writing_any_ticket() {
    use crate::storage::store::{MigrationDryRunReport, MigrationSegment, TicketMigrationPlan};

    let dir = tempdir().unwrap();
    let store = TicketStore::init(dir.path()).unwrap();
    let first_id = store
        .create(
            None,
            "tracker-improvement",
            Some("Valid migration"),
            Some("in-implementation"),
            Default::default(),
            None,
            Some("legacy description"),
        )
        .unwrap();
    let second_id = store
        .create(
            None,
            "tracker-improvement",
            Some("Invalid migration"),
            Some("in-implementation"),
            Default::default(),
            None,
            Some("legacy description"),
        )
        .unwrap();

    let migration_plan = |id, content: &str| TicketMigrationPlan {
        id,
        title: None,
        segments: vec![MigrationSegment {
            kind: "review".to_string(),
            heading: Some("## Review".to_string()),
            content: content.to_string(),
        }],
        matched_counts: BTreeMap::from([("review".to_string(), 1)]),
        objective_lines: 0,
        lossless: true,
    };
    let dry_run = MigrationDryRunReport {
        migratable: vec![
            migration_plan(first_id, "## Review\nvalid"),
            migration_plan(second_id, " \n\t"),
        ],
        ..MigrationDryRunReport::default()
    };

    assert!(matches!(
        store.migration_apply(&dry_run, None),
        Err(StorageError::EmptyPartContent { .. })
    ));
    assert!(store.get(&first_id).unwrap().parts().is_empty());
    assert!(store.get(&second_id).unwrap().parts().is_empty());
}

#[test]
pub(super) fn part_files_use_normalized_kind_slugs_and_manifest_order_ordinals() {
    let dir = tempdir().unwrap();
    let store = TicketStore::init(dir.path()).unwrap();
    let id = store
        .create(
            None,
            "tracker-improvement",
            Some("Slug filenames"),
            Some("planning"),
            Default::default(),
            None,
            None,
        )
        .unwrap();

    let first_id = Uuid::new_v4();
    store
        .write_part(&id, first_id, "review", "first review", None)
        .unwrap();
    let second_id = Uuid::new_v4();
    store
        .write_part(&id, second_id, "REVIEW", "second review", None)
        .unwrap();
    let unicode_id = Uuid::new_v4();
    store
        .write_part(&id, unicode_id, "Straße", "unicode attachment", None)
        .unwrap();

    let manifest = store.get(&id).unwrap();
    let paths: Vec<_> = manifest
        .parts()
        .into_iter()
        .map(|part| (part.id, part.path))
        .collect();
    assert_eq!(
        paths,
        vec![
            (first_id, "parts/review.md".to_string()),
            (second_id, "parts/review-2.md".to_string()),
            (unicode_id, "parts/strasse.md".to_string()),
        ]
    );
}

#[test]
pub(super) fn occupied_slug_part_path_is_rejected_without_manifest_mutation() {
    let dir = tempdir().unwrap();
    let store = TicketStore::init(dir.path()).unwrap();
    let id = store
        .create(
            None,
            "tracker-improvement",
            Some("Path collision"),
            Some("planning"),
            Default::default(),
            None,
            None,
        )
        .unwrap();
    let indexed = store.get_indexed(&id).unwrap().unwrap();
    let parts_dir = indexed.path.join("parts");
    fs::create_dir_all(&parts_dir).unwrap();
    fs::write(parts_dir.join("review.md"), "pre-existing").unwrap();
    let before = store.get(&id).unwrap();

    assert!(matches!(
        store.write_part(&id, Uuid::new_v4(), "review", "new review", None),
        Err(StorageError::PartPathConflict { .. })
    ));

    assert_eq!(store.get(&id).unwrap().parts(), before.parts());
    assert_eq!(
        fs::read_to_string(parts_dir.join("review.md")).unwrap(),
        "pre-existing"
    );
}
