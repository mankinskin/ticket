#[allow(unused_imports)]
use super::*;

#[test]
pub(super) fn update_routes_depends_on_patch_to_canonical_edge_ops() {
    let dir = tempdir().unwrap();
    let store = TicketStore::init(dir.path()).unwrap();

    let source = store
        .create(
            None,
            "tracker-improvement",
            Some("Source"),
            Some("planning"),
            Default::default(),
            None,
            None,
        )
        .unwrap();
    let target_a = store
        .create(
            None,
            "tracker-improvement",
            Some("Target A"),
            Some("planning"),
            Default::default(),
            None,
            None,
        )
        .unwrap();
    let target_b = store
        .create(
            None,
            "tracker-improvement",
            Some("Target B"),
            Some("planning"),
            Default::default(),
            None,
            None,
        )
        .unwrap();

    store
        .add_edge(EdgeRecord {
            from: source,
            to: target_a,
            kind: "depends_on".to_string(),
            created_at: Utc::now(),
        })
        .unwrap();

    let mut patch = BTreeMap::new();
    patch.insert(
        "depends_on".to_string(),
        Value::Array(vec![Value::String(target_b.to_string())]),
    );
    store
        .update(&source, patch, None, None, None, None)
        .unwrap();

    let manifest = store.get(&source).unwrap();
    let items = manifest
        .extra
        .get("depends_on")
        .and_then(Value::as_array)
        .unwrap();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].as_str(), Some(target_b.to_string().as_str()));

    let edges = store.edges_from(&source).unwrap();
    assert_eq!(edges.len(), 1);
    assert_eq!(edges[0].to, target_b);
    assert_eq!(edges[0].kind, "depends_on");
}

#[test]
pub(super) fn update_auto_walks_reachable_multi_step_by_default() {
    let dir = tempdir().unwrap();
    let store = TicketStore::init(dir.path()).unwrap();

    let id = store
        .create(
            None,
            "tracker-improvement",
            Some("Reachable multi-step forward"),
            Some("planning"),
            Default::default(),
            None,
            None,
        )
        .unwrap();

    // `new -> in-implementation` is reachable only by traversing `ready`.
    // Without an explicit opt-out, the update auto-walks the path and lands
    // on the requested target state.
    store
        .update(
            &id,
            BTreeMap::new(),
            None,
            Some("in-implementation"),
            None,
            None,
        )
        .unwrap();

    let indexed = store.get_indexed(&id).unwrap().unwrap();
    assert_eq!(indexed.state.as_deref(), Some("in-implementation"));
}

#[test]
pub(super) fn update_blocks_reachable_multi_step_under_single_hop_flag() {
    let dir = tempdir().unwrap();
    let store = TicketStore::init(dir.path()).unwrap();

    let id = store
        .create(
            None,
            "tracker-improvement",
            Some("Reachable multi-step forward"),
            Some("planning"),
            Default::default(),
            None,
            None,
        )
        .unwrap();

    // Under the `single_hop` opt-out, `new -> in-implementation` is rejected
    // with a recovery-oriented error rather than silently walking the path.
    let err = store
        .update_with_options(
            &id,
            BTreeMap::new(),
            None,
            Some("in-implementation"),
            None,
            Some(DescriptionUpdateMode::Replace),
            None,
            true,
        )
        .unwrap_err();

    match err {
        crate::error::StorageError::Validation(
            crate::error::SchemaValidationError::InvalidTransition {
                from,
                to,
                allowed_next,
                intermediate,
            },
        ) => {
            assert_eq!(from, "planning");
            assert_eq!(to, "in-implementation");
            assert!(
                allowed_next.contains(&"ready".to_string()),
                "allowed next states should list the legal single-hop targets: {allowed_next:?}"
            );
            assert!(
                intermediate.contains(&"ready".to_string()),
                "intermediate path should name the mandatory waypoint: {intermediate:?}"
            );
        }
        other => panic!("expected InvalidTransition, got {other:?}"),
    }

    // The blocked update must not have advanced the ticket.
    let indexed = store.get_indexed(&id).unwrap().unwrap();
    assert_eq!(indexed.state.as_deref(), Some("planning"));
}

#[test]
pub(super) fn update_auto_walks_reachable_reverse_multi_step_by_default() {
    let dir = tempdir().unwrap();
    let store = TicketStore::init(dir.path()).unwrap();

    let id = store
        .create(
            None,
            "tracker-improvement",
            Some("Reachable multi-step reverse"),
            Some("in-implementation"),
            Default::default(),
            None,
            None,
        )
        .unwrap();

    store
        .update(&id, BTreeMap::new(), None, Some("planning"), None, None)
        .unwrap();

    let indexed = store.get_indexed(&id).unwrap().unwrap();
    assert_eq!(indexed.state.as_deref(), Some("planning"));
}

#[test]
pub(super) fn update_blocks_reachable_reverse_multi_step_under_single_hop_flag() {
    let dir = tempdir().unwrap();
    let store = TicketStore::init(dir.path()).unwrap();

    let id = store
        .create(
            None,
            "tracker-improvement",
            Some("Reachable multi-step reverse"),
            Some("in-implementation"),
            Default::default(),
            None,
            None,
        )
        .unwrap();

    let err = store
        .update_with_options(
            &id,
            BTreeMap::new(),
            None,
            Some("planning"),
            None,
            Some(DescriptionUpdateMode::Replace),
            None,
            true,
        )
        .unwrap_err();

    let message = err.to_string();
    assert!(
        message.contains("allows next states"),
        "reverse multi-step block should surface allowed next states: {message}"
    );

    let indexed = store.get_indexed(&id).unwrap().unwrap();
    assert_eq!(indexed.state.as_deref(), Some("in-implementation"));
}
