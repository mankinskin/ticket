#[allow(unused_imports)]
use super::*;

#[test]
pub(super) fn workflow_facts_set_became_actionable_at_when_blockers_resolve() {
    let dir = tempdir().unwrap();
    let store = TicketStore::init(dir.path()).unwrap();
    let blocker = store
        .create(
            None,
            "tracker-improvement",
            Some("Blocking prerequisite"),
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
            Some("Blocked dependent"),
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

    let initial = store.get_workflow_facts(&dependent).unwrap().unwrap();
    assert_eq!(initial.unresolved_dependency_count, 1);
    assert!(initial.became_actionable_at.is_none());

    store.close(&blocker, "done", None).unwrap();

    let updated = store.get_workflow_facts(&dependent).unwrap().unwrap();
    assert_eq!(updated.unresolved_dependency_count, 0);
    assert!(updated.became_actionable_at.is_some());
    assert!(updated.last_blocker_progress_at.is_none());
}

#[test]
pub(super) fn workflow_facts_set_last_blocker_progress_at_while_ticket_remains_blocked() {
    let dir = tempdir().unwrap();
    let store = TicketStore::init(dir.path()).unwrap();
    let progressing_blocker = store
        .create(
            None,
            "tracker-improvement",
            Some("Progressing blocker"),
            Some("planning"),
            Default::default(),
            None,
            None,
        )
        .unwrap();
    let persistent_blocker = store
        .create(
            None,
            "tracker-improvement",
            Some("Persistent blocker"),
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
            Some("Still blocked dependent"),
            Some("planning"),
            Default::default(),
            None,
            None,
        )
        .unwrap();

    for blocker in [progressing_blocker, persistent_blocker] {
        store
            .add_edge(EdgeRecord {
                from: dependent,
                to: blocker,
                kind: "depends_on".to_string(),
                created_at: Utc::now(),
            })
            .unwrap();
    }

    let initial = store.get_workflow_facts(&dependent).unwrap().unwrap();
    assert_eq!(initial.unresolved_dependency_count, 2);
    assert!(initial.last_blocker_progress_at.is_none());

    store
        .update(
            &progressing_blocker,
            BTreeMap::new(),
            Some(&[]),
            Some("planning"),
            None,
            None,
        )
        .unwrap();

    let updated = store.get_workflow_facts(&dependent).unwrap().unwrap();
    assert_eq!(updated.unresolved_dependency_count, 2);
    assert!(updated.last_blocker_progress_at.is_some());
    assert!(updated.became_actionable_at.is_none());
}

#[test]
pub(super) fn workflow_facts_follow_depends_on_edge_removal() {
    let dir = tempdir().unwrap();
    let store = TicketStore::init(dir.path()).unwrap();
    let blocker = store
        .create(
            None,
            "tracker-improvement",
            Some("Transient blocker"),
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
            Some("Edge-driven dependent"),
            Some("planning"),
            Default::default(),
            None,
            None,
        )
        .unwrap();
    let edge = EdgeRecord {
        from: dependent,
        to: blocker,
        kind: "depends_on".to_string(),
        created_at: Utc::now(),
    };

    store.add_edge(edge.clone()).unwrap();
    assert_eq!(
        store
            .get_workflow_facts(&dependent)
            .unwrap()
            .unwrap()
            .unresolved_dependency_count,
        1
    );

    store.remove_edge(edge).unwrap();

    let updated = store.get_workflow_facts(&dependent).unwrap().unwrap();
    assert_eq!(updated.unresolved_dependency_count, 0);
    assert!(updated.became_actionable_at.is_some());
    assert!(updated.last_blocker_progress_at.is_none());
}
