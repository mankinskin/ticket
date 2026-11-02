use std::collections::BTreeMap;
use ticket_api::storage::store::TicketStore;
use uuid::Uuid;

#[test]
fn audited_reopen_is_idempotent_and_preserves_every_previous_revision() {
    let root = tempfile::tempdir_in(".").unwrap();
    let store = TicketStore::init(root.path()).unwrap();
    let id = Uuid::new_v4();
    store
        .create(
            Some(id),
            "tracker-improvement",
            Some("done work"),
            Some("ready"),
            BTreeMap::new(),
            None,
            Some("body"),
        )
        .unwrap();
    store.close(&id, "done", Some("test")).unwrap();
    let before = store.get_history(&id).unwrap();
    let revision = before.last().unwrap().rev;
    let operation = Uuid::new_v4();
    assert!(
        store
            .reopen_done(&id, revision, "   ", operation, None)
            .is_err()
    );
    store
        .reopen_done(
            &id,
            revision,
            "correct reviewed result",
            operation,
            Some("test"),
        )
        .unwrap();
    store
        .reopen_done(
            &id,
            revision,
            "correct reviewed result",
            operation,
            Some("test"),
        )
        .unwrap();
    let after = store.get_history(&id).unwrap();
    assert_eq!(after.len(), before.len() + 1);
    for (a, b) in before.iter().zip(&after) {
        assert_eq!(a.fields, b.fields);
    }
    assert_eq!(store.get(&id).unwrap().extra["state"], "ready");
    assert!(
        store
            .reopen_done(&id, revision, "different reason", operation, None)
            .is_err()
    );
    assert!(
        store
            .reopen_done(&id, revision, "another operation", Uuid::new_v4(), None)
            .is_err()
    );
}

#[test]
fn stale_done_revision_and_cancelled_ticket_never_reopen() {
    let root = tempfile::tempdir_in(".").unwrap();
    let store = TicketStore::init(root.path()).unwrap();
    let id = Uuid::new_v4();
    store
        .create(
            Some(id),
            "tracker-improvement",
            Some("done work"),
            Some("ready"),
            BTreeMap::new(),
            None,
            Some("body"),
        )
        .unwrap();
    store.close(&id, "done", None).unwrap();
    assert!(
        store
            .reopen_done(&id, 0, "stale", Uuid::new_v4(), None)
            .is_err()
    );
    assert_eq!(store.get(&id).unwrap().extra["state"], "done");
    let cancelled = Uuid::new_v4();
    store
        .create(
            Some(cancelled),
            "tracker-improvement",
            Some("cancelled work"),
            Some("cancelled"),
            BTreeMap::new(),
            None,
            Some("body"),
        )
        .unwrap();
    assert!(
        store
            .reopen_done(&cancelled, 0, "unsupported", Uuid::new_v4(), None)
            .is_err()
    );
    assert_eq!(store.get(&cancelled).unwrap().extra["state"], "cancelled");
}
