#[allow(unused_imports)]
use super::*;

#[test]
pub(super) fn update_allows_reverse_transitions_from_terminal_states() {
    let dir = tempdir().unwrap();
    let store = TicketStore::init(dir.path()).unwrap();

    let done_ticket = store
        .create(
            None,
            "tracker-improvement",
            Some("Reopen done ticket"),
            Some("done"),
            Default::default(),
            None,
            None,
        )
        .unwrap();
    store
        .update(
            &done_ticket,
            BTreeMap::new(),
            Some(&[]),
            Some("in-review"),
            None,
            None,
        )
        .unwrap();
    assert_eq!(
        store
            .get_indexed(&done_ticket)
            .unwrap()
            .unwrap()
            .state
            .as_deref(),
        Some("in-review")
    );

    let cancelled_ticket = store
        .create(
            None,
            "tracker-improvement",
            Some("Reopen cancelled ticket"),
            Some("cancelled"),
            Default::default(),
            None,
            None,
        )
        .unwrap();
    store
        .update(
            &cancelled_ticket,
            BTreeMap::new(),
            Some(&[]),
            Some("planning"),
            None,
            None,
        )
        .unwrap();
    assert_eq!(
        store
            .get_indexed(&cancelled_ticket)
            .unwrap()
            .unwrap()
            .state
            .as_deref(),
        Some("planning")
    );
}

#[test]
pub(super) fn update_guards_transition_ahead_of_dependency_state() {
    use crate::error::StorageError;

    let dir = tempdir().unwrap();
    let store = TicketStore::init(dir.path()).unwrap();
    let blocker = store
        .create(
            None,
            "tracker-improvement",
            Some("Guard blocker"),
            Some("planning"),
            Default::default(),
            None,
            None,
        )
        .unwrap();
    let dependent = store
        .create(
            None,
            "tracker-improvement",
            Some("Guard dependent"),
            Some("planning"),
            Default::default(),
            None,
            None,
        )
        .unwrap();
    store
        .add_edge(EdgeRecord {
            from: dependent,
            to: blocker,
            kind: "depends_on".to_string(),
            created_at: Utc::now(),
        })
        .unwrap();

    // ready -> ready (equal rank) is allowed.
    store
        .update(
            &dependent,
            BTreeMap::new(),
            Some(&[]),
            Some("planning"),
            None,
            None,
        )
        .unwrap();

    // Advancing the dependent past the blocker (still 'ready') is rejected.
    let err = store
        .update(
            &dependent,
            BTreeMap::new(),
            Some(&[]),
            Some("in-implementation"),
            None,
            None,
        )
        .unwrap_err();
    assert!(
        matches!(err, StorageError::DependencyNotProgressed { .. }),
        "expected DependencyNotProgressed, got {err:?}"
    );

    // Once the blocker advances, the dependent may match its progress.
    store
        .update(
            &blocker,
            BTreeMap::new(),
            Some(&[]),
            Some("in-implementation"),
            None,
            None,
        )
        .unwrap();
    store
        .update(
            &dependent,
            BTreeMap::new(),
            Some(&[]),
            Some("in-implementation"),
            None,
            None,
        )
        .unwrap();

    // A terminal (done) dependency no longer caps the dependent's progress.
    store.close(&blocker, "done", None).unwrap();
    store
        .update(
            &dependent,
            BTreeMap::new(),
            Some(&[]),
            Some("in-review"),
            None,
            None,
        )
        .unwrap();

    // Cancelling is always permitted regardless of dependency progress.
    let gate = store
        .create(
            None,
            "tracker-improvement",
            Some("Cancel gate"),
            Some("planning"),
            Default::default(),
            None,
            None,
        )
        .unwrap();
    let abandoned = store
        .create(
            None,
            "tracker-improvement",
            Some("Abandoned dependent"),
            Some("planning"),
            Default::default(),
            None,
            None,
        )
        .unwrap();
    store
        .add_edge(EdgeRecord {
            from: abandoned,
            to: gate,
            kind: "depends_on".to_string(),
            created_at: Utc::now(),
        })
        .unwrap();
    store
        .update(
            &abandoned,
            BTreeMap::new(),
            Some(&[]),
            Some("cancelled"),
            None,
            None,
        )
        .unwrap();
}

#[test]
pub(super) fn update_allows_demotion_and_parking_despite_lagging_dependency() {
    let dir = tempdir().unwrap();
    let store = TicketStore::init(dir.path()).unwrap();
    let blocker = store
        .create(
            None,
            "tracker-improvement",
            Some("Guard blocker"),
            Some("planning"),
            Default::default(),
            None,
            None,
        )
        .unwrap();
    let dependent = store
        .create(
            None,
            "tracker-improvement",
            Some("Guard dependent"),
            Some("in-implementation"),
            Default::default(),
            None,
            None,
        )
        .unwrap();
    store
        .add_edge(EdgeRecord {
            from: dependent,
            to: blocker,
            kind: "depends_on".to_string(),
            created_at: Utc::now(),
        })
        .unwrap();

    // Demoting the dependent below its own current rank is allowed even though
    // the dependency (open) has not progressed.
    store
        .update(
            &dependent,
            BTreeMap::new(),
            Some(&[]),
            Some("planning"),
            None,
            None,
        )
        .unwrap();

    // Parking a separate in-implementation ticket is also allowed.
    let parked = store
        .create(
            None,
            "tracker-improvement",
            Some("Guard parked"),
            Some("in-implementation"),
            Default::default(),
            None,
            None,
        )
        .unwrap();
    store
        .add_edge(EdgeRecord {
            from: parked,
            to: blocker,
            kind: "depends_on".to_string(),
            created_at: Utc::now(),
        })
        .unwrap();
    store
        .update(
            &parked,
            BTreeMap::new(),
            Some(&[]),
            Some("on-hold"),
            None,
            None,
        )
        .unwrap();
}

#[test]
pub(super) fn update_still_guards_forward_transition_past_lagging_dependency() {
    use crate::error::StorageError;

    let dir = tempdir().unwrap();
    let store = TicketStore::init(dir.path()).unwrap();
    let blocker = store
        .create(
            None,
            "tracker-improvement",
            Some("Guard blocker"),
            Some("planning"),
            Default::default(),
            None,
            None,
        )
        .unwrap();
    let dependent = store
        .create(
            None,
            "tracker-improvement",
            Some("Guard dependent"),
            Some("in-implementation"),
            Default::default(),
            None,
            None,
        )
        .unwrap();
    store
        .add_edge(EdgeRecord {
            from: dependent,
            to: blocker,
            kind: "depends_on".to_string(),
            created_at: Utc::now(),
        })
        .unwrap();

    let err = store
        .update(
            &dependent,
            BTreeMap::new(),
            Some(&[]),
            Some("in-review"),
            None,
            None,
        )
        .unwrap_err();
    assert!(
        matches!(err, StorageError::DependencyNotProgressed { .. }),
        "expected DependencyNotProgressed, got {err:?}"
    );
}
