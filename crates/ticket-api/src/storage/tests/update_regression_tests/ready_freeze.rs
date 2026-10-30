#[allow(unused_imports)]
use super::*;

#[test]
pub(super) fn f9e70385_ready_freezes_only_existing_planning_parts() {
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
            Some("Legacy objective content"),
        )
        .unwrap();

    let objective_id = Uuid::new_v4();
    store
        .write_part(&id, objective_id, "objective", "objective", None)
        .unwrap();
    let design_id = Uuid::new_v4();
    store
        .write_part(&id, design_id, "design", "design", None)
        .unwrap();
    let notes_id = Uuid::new_v4();
    store
        .write_part(&id, notes_id, "notes", "working notes", None)
        .unwrap();

    store
        .update(&id, BTreeMap::new(), None, Some("ready"), None, None)
        .unwrap();

    let manifest = store.get(&id).unwrap();
    let parts = manifest.parts();

    // Only planning parts that already exist are frozen. Missing kinds stay
    // absent, and non-planning parts remain writable.
    for &kind in PLANNING_KINDS {
        if let Some(part) = parts.iter().find(|p| p.kind == kind) {
            assert!(part.frozen, "planning part '{kind}' must be frozen");
        }
    }
    assert!(parts.iter().any(|p| p.id == objective_id));
    assert!(parts.iter().any(|p| p.id == design_id));
    let notes = parts.iter().find(|p| p.id == notes_id).unwrap();
    assert!(!notes.frozen, "notes part must never be frozen");
    let frozen_count = parts.iter().filter(|p| p.frozen).count();
    assert_eq!(frozen_count, 2, "only existing planning parts are frozen");
}

#[test]
pub(super) fn ready_rejects_new_planning_parts_but_allows_notes_and_amendments() {
    let dir = tempdir().unwrap();
    let store = TicketStore::init(dir.path()).unwrap();
    let id = store
        .create(
            None,
            "tracker-improvement",
            Some("Ready part policy"),
            Some("planning"),
            Default::default(),
            None,
            None,
        )
        .unwrap();
    let objective_id = Uuid::new_v4();
    store
        .write_part(&id, objective_id, "objective", "objective", None)
        .unwrap();
    store
        .update(&id, BTreeMap::new(), None, Some("ready"), None, None)
        .unwrap();

    let requirements_id = Uuid::new_v4();
    assert!(matches!(
        store.write_part(
            &id,
            requirements_id,
            "requirements",
            "late requirements",
            None
        ),
        Err(StorageError::ReadyPlanningPartWrite { .. })
    ));
    let manifest = store.get(&id).unwrap();
    assert!(
        !manifest
            .parts()
            .iter()
            .any(|part| part.id == requirements_id)
    );

    let notes_id = Uuid::new_v4();
    store
        .write_part(&id, notes_id, "notes", "progress note", None)
        .unwrap();
    let amendment_id = Uuid::new_v4();
    store
        .write_amendment_part(&id, amendment_id, "corrected objective", objective_id, None)
        .unwrap();
    let manifest = store.get(&id).unwrap();
    assert!(manifest.parts().iter().any(|part| part.id == notes_id));
    assert!(manifest.parts().iter().any(|part| part.id == amendment_id));
}

#[test]
pub(super) fn f9e70385_write_to_frozen_part_is_rejected_and_file_byte_identical() {
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
            Some("Stable objective"),
        )
        .unwrap();

    let objective_id = Uuid::new_v4();
    store
        .write_part(&id, objective_id, "objective", "Stable objective", None)
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
    let before = fs::read(path.join(&objective.path)).unwrap();

    // AC2: the write is hard-rejected.
    let err = store
        .write_part(&id, objective.id, "objective", "sneaky overwrite", None)
        .unwrap_err();

    let after = fs::read(path.join(&objective.path)).unwrap();
    assert_eq!(
        before, after,
        "the frozen part file must be byte-identical after a rejected write"
    );

    // AC3: the error names the part (kind + id), the freezing state, and
    // both recovery paths (amendment w/ supersedes; transition back to a
    // pre-ready state).
    let message = err.to_string();
    assert!(
        message.contains("objective"),
        "must name the kind: {message}"
    );
    assert!(
        message.contains(&objective.id.to_string()),
        "must name the part id: {message}"
    );
    assert!(
        message.contains("ready"),
        "must name the freezing state: {message}"
    );
    assert!(
        message.contains("amendment") && message.contains("supersedes"),
        "must name the amendment recovery path: {message}"
    );
    assert!(
        message.contains("transition"),
        "must name the state-transition recovery path: {message}"
    );
}

#[test]
pub(super) fn f9e70385_review_write_on_ready_ticket_succeeds() {
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

    let objective_id = Uuid::new_v4();
    store
        .write_part(&id, objective_id, "objective", "Objective v1", None)
        .unwrap();
    store
        .update(&id, BTreeMap::new(), None, Some("ready"), None, None)
        .unwrap();

    // AC4: a write to `review` on a `ready` ticket succeeds.
    let review_id = Uuid::new_v4();
    store
        .write_part(&id, review_id, "review", "Reviewed, looks good.", None)
        .unwrap();

    let path = store.get_indexed(&id).unwrap().unwrap().path;
    let manifest = store.get(&id).unwrap();
    let review = manifest
        .parts()
        .into_iter()
        .find(|p| p.id == review_id)
        .unwrap();
    assert!(!review.frozen);
    assert_eq!(
        fs::read_to_string(path.join(&review.path)).unwrap(),
        "Reviewed, looks good."
    );
}

#[test]
pub(super) fn f9e70385_unfreeze_refreeze_cycle_appends_plan_revision() {
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
            Some("Objective v1"),
        )
        .unwrap();

    let objective_id = Uuid::new_v4();
    store
        .write_part(&id, objective_id, "objective", "Objective v1", None)
        .unwrap();
    store
        .update(&id, BTreeMap::new(), None, Some("ready"), None, None)
        .unwrap();
    let manifest = store.get(&id).unwrap();
    assert!(
        manifest
            .parts()
            .iter()
            .all(|p| p.frozen == (PLANNING_KINDS.contains(&p.kind.as_str())))
    );
    let revision_1 = manifest
        .extra
        .get("plan_revision")
        .and_then(Value::as_u64)
        .unwrap();
    assert_eq!(revision_1, 1);

    // AC5: transitioning back to a pre-`ready` state clears every frozen
    // flag.
    store
        .update(&id, BTreeMap::new(), None, Some("planning"), None, None)
        .unwrap();
    let manifest = store.get(&id).unwrap();
    assert!(
        manifest.parts().iter().all(|p| !p.frozen),
        "every frozen flag must be cleared after unfreezing"
    );

    // The now-unfrozen objective part must accept a direct write.
    let objective = manifest
        .parts()
        .into_iter()
        .find(|p| p.kind == "objective")
        .unwrap();
    store
        .write_part(&id, objective.id, "objective", "Objective v2", None)
        .unwrap();

    // Re-entering `ready` re-freezes and appends a plan revision.
    store
        .update(&id, BTreeMap::new(), None, Some("ready"), None, None)
        .unwrap();
    let manifest = store.get(&id).unwrap();
    assert!(
        manifest
            .parts()
            .iter()
            .all(|p| p.frozen == (PLANNING_KINDS.contains(&p.kind.as_str())))
    );
    let revision_2 = manifest
        .extra
        .get("plan_revision")
        .and_then(Value::as_u64)
        .unwrap();
    assert_eq!(
        revision_2, 2,
        "re-entering ready must append a plan revision"
    );
}
