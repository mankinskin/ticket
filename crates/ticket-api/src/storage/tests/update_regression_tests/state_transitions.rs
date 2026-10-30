#[allow(unused_imports)]
use super::*;

#[test]
pub(super) fn scan_migrates_legacy_new_planned_and_open_states_to_planning() {
    let dir = tempdir().unwrap();
    let store = TicketStore::init(dir.path()).unwrap();

    for legacy_state in ["new", "planned", "open"] {
        let id = store
            .create(
                None,
                "tracker-improvement",
                Some("Legacy state"),
                Some("planning"),
                Default::default(),
                None,
                None,
            )
            .unwrap();
        let mut manifest = store.get(&id).unwrap();
        manifest
            .extra
            .insert("state".to_string(), Value::String(legacy_state.to_string()));
        let path = store.get_indexed(&id).unwrap().unwrap().path;
        let toml = memory_kernel::model::manifest_format::format_manifest_toml(&manifest);
        std::fs::write(
            path.join(crate::model::filesystem::TICKET_MANIFEST_FILE),
            toml,
        )
        .unwrap();
    }

    store.scan(true).unwrap();

    for ticket in store.list(None, None, None).unwrap() {
        assert_eq!(ticket.state.as_deref(), Some("planning"));
        assert_eq!(
            store
                .get(&ticket.id)
                .unwrap()
                .extra
                .get("state")
                .and_then(Value::as_str),
            Some("planning")
        );
    }
}

#[test]
pub(super) fn create_rejects_off_schema_state() {
    let dir = tempdir().unwrap();
    let store = TicketStore::init(dir.path()).unwrap();

    let err = store
        .create(
            None,
            "tracker-improvement",
            Some("Invalid creation state"),
            Some("archived"),
            Default::default(),
            None,
            None,
        )
        .expect_err("off-schema state must be rejected before persistence");

    match err {
        crate::error::StorageError::Validation(
            crate::error::SchemaValidationError::OffSchemaState { state, allowed },
        ) => {
            assert_eq!(state, "archived");
            assert!(allowed.contains(&"planning".to_string()));
        }
        other => panic!("expected OffSchemaState, got {other:?}"),
    }
    assert!(store.list(None, None, None).unwrap().is_empty());
}

#[test]
pub(super) fn off_schema_state_recovers_only_to_entry_state_then_transitions_normally() {
    let dir = tempdir().unwrap();
    let store = TicketStore::init(dir.path()).unwrap();
    let id = store
        .create(
            None,
            "tracker-improvement",
            Some("Legacy off-schema state"),
            Some("planning"),
            Default::default(),
            None,
            None,
        )
        .unwrap();

    let mut manifest = store.get(&id).unwrap();
    manifest
        .extra
        .insert("state".to_string(), Value::String("archived".to_string()));
    let ticket_path = store.get_indexed(&id).unwrap().unwrap().path;
    let toml_str = memory_kernel::model::manifest_format::format_manifest_toml(&manifest);
    std::fs::write(
        ticket_path.join(crate::model::filesystem::TICKET_MANIFEST_FILE),
        toml_str,
    )
    .unwrap();
    store.scan(true).unwrap();

    let err = store
        .update(&id, BTreeMap::new(), None, Some("ready"), None, None)
        .expect_err("recovery must not jump to a non-entry state");
    assert!(err.to_string().contains("'archived' -> 'ready'"));

    store
        .update(&id, BTreeMap::new(), None, Some("planning"), None, None)
        .expect("off-schema state should recover to the entry state");
    store
        .update(&id, BTreeMap::new(), None, Some("planning"), None, None)
        .expect("recovered ticket should transition normally");

    let indexed = store.get_indexed(&id).unwrap().unwrap();
    assert_eq!(indexed.state.as_deref(), Some("planning"));
}

#[test]
pub(super) fn ticket_29a56eef_state_in_field_patch_errors_instead_of_silently_dropping() {
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

    // A `state` key sent through a plain field/field_map patch (no
    // `to_state`) must never be silently dropped: it must either apply or
    // return an explicit error naming the field.
    let mut patch = BTreeMap::new();
    patch.insert("state".to_string(), Value::String("planning".to_string()));

    let result = store.update(&id, patch, None, None, None, None);

    match result {
        Ok(_) => {
            let indexed = store.get_indexed(&id).unwrap().unwrap();
            assert_eq!(
                indexed.state.as_deref(),
                Some("planning"),
                "if the write is accepted, it must actually apply"
            );
        }
        Err(err) => {
            let message = err.to_string();
            assert!(
                message.contains("state"),
                "error must name the rejected field, got: {message}"
            );
        }
    }
}

#[test]
pub(super) fn bug_7f4aaa05_state_preserved_on_field_patch_without_to_state() {
    let dir = tempdir().unwrap();
    let store = TicketStore::init(dir.path()).unwrap();

    // Create ticket
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

    // Advance to ready
    store
        .update(&id, BTreeMap::new(), Some(&[]), Some("ready"), None, None)
        .unwrap();

    let indexed = store.get_indexed(&id).unwrap().unwrap();
    assert_eq!(indexed.state.as_deref(), Some("ready"));

    // BUG: Update description WITHOUT to_state - state should be preserved
    let mut patch = BTreeMap::new();
    patch.insert(
        "custom_field".to_string(),
        Value::String("custom value".to_string()),
    );

    store.update(&id, patch, None, None, None, None).unwrap();

    let indexed = store.get_indexed(&id).unwrap().unwrap();
    assert_eq!(
        indexed.state.as_deref(),
        Some("ready"),
        "State should be preserved when patching fields without to_state"
    );
}

#[test]
pub(super) fn bug_7f4aaa05_description_patch_with_to_state_transition() {
    let dir = tempdir().unwrap();
    let store = TicketStore::init(dir.path()).unwrap();

    // Create ticket
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

    // Advance to ready
    store
        .update(&id, BTreeMap::new(), Some(&[]), Some("ready"), None, None)
        .unwrap();

    let indexed = store.get_indexed(&id).unwrap().unwrap();
    assert_eq!(indexed.state.as_deref(), Some("ready"));

    // Combined: patch fields AND transition in one call
    let mut patch = BTreeMap::new();
    patch.insert(
        "custom_field".to_string(),
        Value::String("custom value".to_string()),
    );

    store
        .update(&id, patch, None, Some("in-implementation"), None, None)
        .unwrap();

    let indexed = store.get_indexed(&id).unwrap().unwrap();
    assert_eq!(
        indexed.state.as_deref(),
        Some("in-implementation"),
        "State should transition to in-implementation"
    );
    assert_eq!(
        indexed.title.as_deref(),
        Some("Test ticket"),
        "Title should be preserved"
    );
}

#[test]
pub(super) fn bug_7f4aaa05_transition_states_multi_step_path() {
    let dir = tempdir().unwrap();
    let store = TicketStore::init(dir.path()).unwrap();

    // Create ticket
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

    // Multi-step transition: new -> ready
    let transition_states = vec!["ready".to_string()];
    store
        .update(
            &id,
            BTreeMap::new(),
            Some(transition_states.as_slice()),
            None, // NO to_state
            None,
            None,
        )
        .unwrap();

    let indexed = store.get_indexed(&id).unwrap().unwrap();
    assert_eq!(
        indexed.state.as_deref(),
        Some("ready"),
        "transition_states should apply the final state from the path"
    );
}
