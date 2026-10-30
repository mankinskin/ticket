#[allow(unused_imports)]
use super::*;

#[test]
pub(super) fn ticket_3d952036_review_part_write_leaves_objective_byte_identical() {
    let dir = tempdir().unwrap();
    let store = TicketStore::init(dir.path()).unwrap();

    let id = store
        .create(
            None,
            "tracker-improvement",
            Some("Test ticket"),
            Some("planning"),
            Default::default(),
            None,
            Some("Original objective content, unchanged."),
        )
        .unwrap();
    let path = store.get_indexed(&id).unwrap().unwrap().path;
    let objective_before = fs::read(path.join("description.md")).unwrap();

    // AC3/AC4: a part-addressed write targeting a fresh review part id
    // must never read or write the objective / description.md.
    let review_part_id = Uuid::new_v4();
    store
        .write_part(&id, review_part_id, "review", "Looks good.", None)
        .unwrap();

    let objective_after = fs::read(path.join("description.md")).unwrap();
    assert_eq!(
        objective_before, objective_after,
        "writing a review part must leave the objective bytes byte-identical"
    );

    let manifest = store.get(&id).unwrap();
    let parts = manifest.parts();
    let review = parts
        .iter()
        .find(|p| p.id == review_part_id)
        .expect("review part recorded in manifest");
    assert_eq!(review.kind, "review");
    let review_content = fs::read_to_string(path.join(&review.path)).unwrap();
    assert_eq!(review_content, "Looks good.");
}

#[test]
pub(super) fn ticket_3d952036_per_part_history_and_undo_restores_only_that_part() {
    let dir = tempdir().unwrap();
    let store = TicketStore::init(dir.path()).unwrap();

    let id = store
        .create(
            None,
            "tracker-improvement",
            Some("Test ticket"),
            Some("planning"),
            Default::default(),
            None,
            None,
        )
        .unwrap();

    let review_id = Uuid::new_v4();
    store
        .write_part(&id, review_id, "review", "first pass", None)
        .unwrap();
    let other_id = Uuid::new_v4();
    store
        .write_part(&id, other_id, "validation", "unrelated evidence", None)
        .unwrap();
    store
        .write_part(&id, review_id, "review", "second pass", None)
        .unwrap();

    // AC5: history carries the prior content of only the changed part.
    let revisions = store.get_history(&id).unwrap();
    let last = revisions.last().unwrap();
    assert_eq!(
        last.fields.get(crate::storage::store::PART_HISTORY_ID_KEY),
        Some(&Value::String(review_id.to_string()))
    );
    assert_eq!(
        last.fields
            .get(crate::storage::store::PART_HISTORY_CONTENT_KEY),
        Some(&Value::String("first pass".to_string()))
    );

    store.undo_part(&id, review_id, None).unwrap();

    let path = store.get_indexed(&id).unwrap().unwrap().path;
    let manifest = store.get(&id).unwrap();
    let parts = manifest.parts();
    let review = parts.iter().find(|p| p.id == review_id).unwrap();
    let other = parts.iter().find(|p| p.id == other_id).unwrap();
    assert_eq!(
        fs::read_to_string(path.join(&review.path)).unwrap(),
        "first pass",
        "undo must restore only the addressed part"
    );
    assert_eq!(
        fs::read_to_string(path.join(&other.path)).unwrap(),
        "unrelated evidence",
        "undo of one part must never touch another part's content"
    );
}

#[test]
pub(super) fn ticket_3d952036_legacy_ticket_with_no_parts_table_still_updatable() {
    let dir = tempdir().unwrap();
    let store = TicketStore::init(dir.path()).unwrap();

    let id = store
        .create(
            None,
            "tracker-improvement",
            Some("Legacy ticket"),
            Some("planning"),
            Default::default(),
            None,
            Some("Legacy objective"),
        )
        .unwrap();

    // A legacy ticket carries no `[[parts]]` table.
    let manifest = store.get(&id).unwrap();
    assert!(manifest.parts().is_empty());

    // It must still be updatable through the plain field/description path.
    store
        .update_with_options(
            &id,
            BTreeMap::new(),
            None,
            None,
            Some("Updated legacy objective"),
            Some(DescriptionUpdateMode::Replace),
            None,
            false,
        )
        .unwrap();
    let path = store.get_indexed(&id).unwrap().unwrap().path;
    assert_eq!(
        crate::storage::ticket_fs::TicketFs::read_description(&path)
            .unwrap()
            .as_deref(),
        Some("Updated legacy objective")
    );

    // And it must still be part-addressable via its implicit objective id.
    let report =
        crate::storage::ticket_fs::TicketFs::load_parts(&path, &store.get(&id).unwrap()).unwrap();
    assert_eq!(report.parts.len(), 1);
    let implicit_id = report.parts[0].id;
    store
        .write_part(&id, implicit_id, "objective", "Via part write", None)
        .unwrap();
    assert_eq!(
        crate::storage::ticket_fs::TicketFs::read_description(&path)
            .unwrap()
            .as_deref(),
        Some("Via part write"),
        "writing the implicit objective part must go through description.md"
    );
}
