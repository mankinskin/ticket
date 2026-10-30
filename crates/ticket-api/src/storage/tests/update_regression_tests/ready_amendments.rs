#[allow(unused_imports)]
use super::*;

#[test]
pub(super) fn f9e70385_amendment_records_supersedes_and_is_retrievable_alongside_frozen_part() {
    let dir = tempdir().unwrap();
    let store = TicketStore::init(dir.path()).unwrap();

    let id = store
        .create(
            None,
            "tracker-improvement",
            Some("Plan freeze"),
            Some("planning"),
            Default::default(),
            None,
            None,
        )
        .unwrap();

    let requirements_id = Uuid::new_v4();
    store
        .write_part(
            &id,
            requirements_id,
            "requirements",
            "Original requirements",
            None,
        )
        .unwrap();

    let objective_id = Uuid::new_v4();
    store
        .write_part(
            &id,
            objective_id,
            "objective",
            "Frozen objective content",
            None,
        )
        .unwrap();
    store
        .update(&id, BTreeMap::new(), None, Some("ready"), None, None)
        .unwrap();

    let manifest = store.get(&id).unwrap();
    let requirements = manifest
        .parts()
        .into_iter()
        .find(|p| p.kind == "requirements")
        .unwrap();
    assert!(requirements.frozen);

    // AC6: an `amendment` part records `supersedes` and is retrievable
    // alongside the part it supersedes.
    let amendment_id = Uuid::new_v4();
    store
        .write_amendment_part(
            &id,
            amendment_id,
            "Corrected requirement text",
            requirements.id,
            None,
        )
        .unwrap();

    let path = store.get_indexed(&id).unwrap().unwrap().path;
    let manifest = store.get(&id).unwrap();
    let report = crate::storage::ticket_fs::TicketFs::load_parts(&path, &manifest).unwrap();
    let amendment = report
        .parts
        .iter()
        .find(|p| p.id == amendment_id)
        .expect("amendment part retrievable");
    assert_eq!(amendment.kind, "amendment");
    assert_eq!(amendment.supersedes, Some(requirements.id));
    assert_eq!(amendment.content, "Corrected requirement text");

    // The superseded part remains present and byte-identical.
    let still_frozen = report
        .parts
        .iter()
        .find(|p| p.id == requirements.id)
        .unwrap();
    assert!(still_frozen.frozen);
    assert_eq!(still_frozen.content, "Original requirements");
}

#[test]
pub(super) fn f9e70385_undo_part_on_frozen_part_is_also_rejected() {
    let dir = tempdir().unwrap();
    let store = TicketStore::init(dir.path()).unwrap();

    let id = store
        .create(
            None,
            "tracker-improvement",
            Some("Plan freeze"),
            Some("planning"),
            Default::default(),
            None,
            None,
        )
        .unwrap();

    // Write `design` while still unfrozen so there is history to undo.
    let design_id = Uuid::new_v4();
    store
        .write_part(&id, design_id, "design", "design v1", None)
        .unwrap();
    store
        .write_part(&id, design_id, "design", "design v2", None)
        .unwrap();

    store
        .update(&id, BTreeMap::new(), None, Some("ready"), None, None)
        .unwrap();

    // AC7: undo is a write like any other and must not bypass the gate.
    let err = store.undo_part(&id, design_id, None).unwrap_err();
    assert!(matches!(
        err,
        crate::error::StorageError::FrozenPartWrite { .. }
    ));

    let path = store.get_indexed(&id).unwrap().unwrap().path;
    let manifest = store.get(&id).unwrap();
    let design = manifest
        .parts()
        .into_iter()
        .find(|p| p.id == design_id)
        .unwrap();
    assert_eq!(
        fs::read_to_string(path.join(&design.path)).unwrap(),
        "design v2",
        "a rejected undo must leave the frozen part untouched"
    );
}

#[test]
pub(super) fn f9e70385_legacy_description_write_rejected_when_objective_frozen() {
    let dir = tempdir().unwrap();
    let store = TicketStore::init(dir.path()).unwrap();

    let id = store
        .create(
            None,
            "tracker-improvement",
            Some("Plan freeze"),
            Some("planning"),
            Default::default(),
            None,
            Some("Frozen objective content"),
        )
        .unwrap();

    let objective_id = Uuid::new_v4();
    store
        .write_part(
            &id,
            objective_id,
            "objective",
            "Frozen objective content",
            None,
        )
        .unwrap();
    store
        .update(&id, BTreeMap::new(), None, Some("ready"), None, None)
        .unwrap();

    let path = store.get_indexed(&id).unwrap().unwrap().path;
    let manifest = store.get(&id).unwrap();
    let objective = manifest
        .parts()
        .into_iter()
        .find(|p| p.kind == "objective")
        .unwrap();
    assert!(objective.frozen);
    let before_part = fs::read(path.join(&objective.path)).unwrap();
    let before_description = fs::read(path.join("description.md")).unwrap();

    // AC7 (review-reproduced bypass): the legacy `description`/
    // `description_mode` write path in `update_with_options` must reject
    // exactly like a part-addressed write when the ticket's `objective`
    // part is frozen — it is not an alternate, ungated entry point.
    let err = store
        .update_with_options(
            &id,
            BTreeMap::new(),
            None,
            None,
            Some("orphaned legacy write"),
            Some(DescriptionUpdateMode::Replace),
            None,
            false,
        )
        .unwrap_err();
    assert!(matches!(
        err,
        crate::error::StorageError::FrozenPartWrite { .. }
    ));

    let after_part = fs::read(path.join(&objective.path)).unwrap();
    let after_description = fs::read(path.join("description.md")).unwrap();
    assert_eq!(
        before_part, after_part,
        "a rejected legacy write must leave the frozen objective part file untouched"
    );
    assert_eq!(
        before_description, after_description,
        "a rejected legacy write must leave description.md untouched"
    );
}

#[test]
pub(super) fn bc74e91f_combined_freeze_and_description_write_keeps_missing_objective_absent() {
    let dir = tempdir().unwrap();
    let store = TicketStore::init(dir.path()).unwrap();

    let id = store
        .create(
            None,
            "tracker-improvement",
            Some("Combined freeze + description write"),
            Some("planning"),
            Default::default(),
            None,
            None,
        )
        .unwrap();

    let written = "combined write content";

    // A transition to `ready` freezes existing planning parts only. Writing
    // the description in the same update must not materialize an objective.
    store
        .update_with_options(
            &id,
            BTreeMap::new(),
            None,
            Some("ready"),
            Some(written),
            Some(DescriptionUpdateMode::Replace),
            None,
            false,
        )
        .unwrap();

    let path = store.get_indexed(&id).unwrap().unwrap().path;
    let manifest = store.get(&id).unwrap();
    let description_content = fs::read_to_string(path.join("description.md")).unwrap();

    assert_eq!(
        description_content, written,
        "description.md must contain the newly written description"
    );
    assert!(
        manifest.parts().is_empty(),
        "missing planning parts must remain absent"
    );

    let indexed = store.get_indexed(&id).unwrap().unwrap();
    assert_eq!(indexed.state.as_deref(), Some("ready"));
}
