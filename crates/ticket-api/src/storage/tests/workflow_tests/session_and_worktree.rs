#[allow(unused_imports)]
use super::*;

#[test]
pub(super) fn release_lease_enforces_owner_and_stale_rules() {
    use crate::error::StorageError;

    let dir = tempdir().unwrap();
    let store = TicketStore::init(dir.path()).unwrap();
    let ticket = store
        .create(
            None,
            "tracker-improvement",
            Some("Leased ticket"),
            Some("planning"),
            Default::default(),
            None,
            None,
        )
        .unwrap();

    // The holder may release its own live lease.
    store.claim(&ticket, "agent-a", 3600, Some("work")).unwrap();
    store.release_lease(&ticket, "agent-a").unwrap();
    assert!(store.list_leases().unwrap().is_empty());

    // A different agent may not release a live lease held by someone else.
    store.claim(&ticket, "agent-a", 3600, Some("work")).unwrap();
    let err = store.release_lease(&ticket, "agent-b").unwrap_err();
    assert!(
        matches!(err, StorageError::LeaseConflict { .. }),
        "expected LeaseConflict, got {err:?}"
    );
    store.release_lease(&ticket, "agent-a").unwrap();

    // Any agent may release a stale (expired) lease.
    store.claim(&ticket, "agent-a", 0, Some("work")).unwrap();
    store.release_lease(&ticket, "agent-b").unwrap();
    assert!(store.list_leases().unwrap().is_empty());

    // Releasing a ticket with no active lease is a no-op.
    store.release_lease(&ticket, "agent-b").unwrap();
}

#[test]
pub(super) fn board_check_out_releases_orphaned_lease_when_entry_is_missing() {
    use crate::storage::board::BoardError;

    let dir = tempdir().unwrap();
    let store = TicketStore::init(dir.path()).unwrap();
    let ticket = store
        .create(
            None,
            "tracker-improvement",
            Some("Orphaned lease ticket"),
            Some("planning"),
            Default::default(),
            None,
            None,
        )
        .unwrap();

    store
        .board_check_in(&ticket, "agent-a", 0, "work", vec![], None, None, None)
        .unwrap();
    assert_eq!(store.list_leases().unwrap().len(), 1);

    let preview = store.board_clean_preview(true).unwrap();
    assert_eq!(preview.entry_ids.len(), 1);
    store.board_clean_apply(&preview.token, true).unwrap();

    let err = store
        .board_check_out(&ticket, "agent-a", Some("cleanup orphan"))
        .unwrap_err();
    assert!(
        matches!(err, BoardError::NotCheckedIn { .. }),
        "expected NotCheckedIn, got {err:?}"
    );
    assert!(store.list_leases().unwrap().is_empty());
}

#[test]
pub(super) fn board_check_in_round_trips_session_and_worktree_metadata() {
    let dir = tempdir().unwrap();
    let store = TicketStore::init(dir.path()).unwrap();
    let ticket = store
        .create(
            None,
            "tracker-improvement",
            Some("Metadata ticket"),
            Some("planning"),
            Default::default(),
            None,
            None,
        )
        .unwrap();

    let entry = store
        .board_check_in(
            &ticket,
            "agent-a",
            3600,
            "work",
            vec![],
            Some("session-a".to_string()),
            Some("/tmp/worktree-a".to_string()),
            Some("agent/metadata".to_string()),
        )
        .unwrap();
    let snapshot = store.board_show(None).unwrap();
    let stored = snapshot
        .entries
        .iter()
        .find(|candidate| candidate.entry_id == entry.entry_id)
        .unwrap();

    assert_eq!(stored.session_id.as_deref(), Some("session-a"));
    assert_eq!(stored.worktree_path.as_deref(), Some("/tmp/worktree-a"));
    assert_eq!(stored.branch.as_deref(), Some("agent/metadata"));
}

#[test]
pub(super) fn board_worktrees_groups_entries_by_path() {
    let dir = tempdir().unwrap();
    let store = TicketStore::init(dir.path()).unwrap();
    let create_ticket = |title| {
        store
            .create(
                None,
                "tracker-improvement",
                Some(title),
                Some("planning"),
                Default::default(),
                None,
                None,
            )
            .unwrap()
    };
    let first = create_ticket("First worktree ticket");
    let second = create_ticket("Second worktree ticket");
    let third = create_ticket("Third worktree ticket");

    for (ticket, agent, path) in [
        (first, "agent-a", "/tmp/worktree-a"),
        (second, "agent-b", "/tmp/worktree-a"),
        (third, "agent-c", "/tmp/worktree-b"),
    ] {
        store
            .board_check_in(
                &ticket,
                agent,
                3600,
                "work",
                vec![],
                Some("session-a".to_string()),
                Some(path.to_string()),
                None,
            )
            .unwrap();
    }

    let snapshot = store.board_show(None).unwrap();
    assert_eq!(snapshot.active_worktrees.len(), 2);
    let grouped = snapshot
        .active_worktrees
        .iter()
        .find(|worktree| worktree.worktree_path == "/tmp/worktree-a")
        .unwrap();
    assert_eq!(grouped.ticket_ids, vec![first, second]);
}

#[test]
pub(super) fn board_check_in_rejects_worktree_owned_by_another_session() {
    use crate::storage::board::BoardError;

    let dir = tempdir().unwrap();
    let store = TicketStore::init(dir.path()).unwrap();
    let first = store
        .create(
            None,
            "tracker-improvement",
            Some("First"),
            Some("planning"),
            Default::default(),
            None,
            None,
        )
        .unwrap();
    let second = store
        .create(
            None,
            "tracker-improvement",
            Some("Second"),
            Some("planning"),
            Default::default(),
            None,
            None,
        )
        .unwrap();

    store
        .board_check_in(
            &first,
            "agent-a",
            3600,
            "work",
            vec![],
            Some("session-a".to_string()),
            Some("/tmp/worktree-a".to_string()),
            None,
        )
        .unwrap();
    store
        .board_check_in(
            &second,
            "agent-b",
            3600,
            "work",
            vec![],
            Some("session-a".to_string()),
            Some("/tmp/worktree-a".to_string()),
            None,
        )
        .unwrap();

    let third = store
        .create(
            None,
            "tracker-improvement",
            Some("Third"),
            Some("planning"),
            Default::default(),
            None,
            None,
        )
        .unwrap();
    let error = store
        .board_check_in(
            &third,
            "agent-c",
            3600,
            "work",
            vec![],
            Some("session-b".to_string()),
            Some("/tmp/worktree-a".to_string()),
            None,
        )
        .unwrap_err();
    assert!(matches!(error, BoardError::WorktreeConflict { .. }));
}

#[test]
pub(super) fn board_check_in_requires_session_for_worktree_and_allows_unbound_entry() {
    use crate::storage::board::BoardError;

    let dir = tempdir().unwrap();
    let store = TicketStore::init(dir.path()).unwrap();
    let ticket = store
        .create(
            None,
            "tracker-improvement",
            Some("Worktree binding"),
            Some("planning"),
            Default::default(),
            None,
            None,
        )
        .unwrap();

    let error = store
        .board_check_in(
            &ticket,
            "agent-a",
            3600,
            "work",
            vec![],
            None,
            Some("/tmp/worktree-a".to_string()),
            None,
        )
        .unwrap_err();
    assert!(matches!(error, BoardError::WorktreeRequiresSession { .. }));

    let entry = store
        .board_check_in(&ticket, "agent-a", 3600, "work", vec![], None, None, None)
        .unwrap();
    assert!(entry.session_id.is_none());
    assert!(entry.worktree_path.is_none());
    assert!(entry.branch.is_none());
}
