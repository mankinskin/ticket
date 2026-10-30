#[allow(unused_imports)]
use super::*;

#[test]
pub(super) fn update_without_description_preserves_existing_description() {
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
            Some("Original description"),
        )
        .unwrap();

    // Regression: a field-only update that never intended to touch the
    // description must not clobber the existing description.md content.
    let mut patch = BTreeMap::new();
    patch.insert(
        "custom_field".to_string(),
        Value::String("custom value".to_string()),
    );
    store.update(&id, patch, None, None, None, None).unwrap();

    let path = store.get_indexed(&id).unwrap().unwrap().path;
    let description = crate::storage::ticket_fs::TicketFs::read_description(&path).unwrap();
    assert_eq!(
        description.as_deref(),
        Some("Original description"),
        "an update that omits description must preserve the existing description"
    );
}

#[test]
pub(super) fn update_with_replace_mode_overwrites_description() {
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
            Some("Original description"),
        )
        .unwrap();

    store
        .update_with_options(
            &id,
            BTreeMap::new(),
            None,
            None,
            Some("New description"),
            Some(DescriptionUpdateMode::Replace),
            None,
            false,
        )
        .unwrap();

    let path = store.get_indexed(&id).unwrap().unwrap().path;
    let description = crate::storage::ticket_fs::TicketFs::read_description(&path).unwrap();
    assert_eq!(description.as_deref(), Some("New description"));
}

#[test]
pub(super) fn update_with_append_mode_concatenates_description() {
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
            Some("Original description"),
        )
        .unwrap();

    store
        .update_with_options(
            &id,
            BTreeMap::new(),
            None,
            None,
            Some("Extra note"),
            Some(DescriptionUpdateMode::Append),
            None,
            false,
        )
        .unwrap();

    let path = store.get_indexed(&id).unwrap().unwrap().path;
    let description = crate::storage::ticket_fs::TicketFs::read_description(&path).unwrap();
    assert_eq!(
        description.as_deref(),
        Some("Original description\nExtra note"),
        "append mode should concatenate onto the existing description"
    );
}

#[test]
pub(super) fn update_captures_previous_description_in_history_regardless_of_mode() {
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
            Some("Original description"),
        )
        .unwrap();

    store
        .update_with_options(
            &id,
            BTreeMap::new(),
            None,
            None,
            Some("New description"),
            Some(DescriptionUpdateMode::Replace),
            None,
            false,
        )
        .unwrap();

    let revisions = store.get_history(&id).unwrap();
    let last = revisions.last().expect("history revision recorded");
    assert_eq!(
        last.fields
            .get(crate::storage::store::DESCRIPTION_HISTORY_KEY),
        Some(&Value::String("Original description".to_string())),
        "the pre-update description must be captured in history on every description change"
    );
}

#[test]
pub(super) fn undo_restores_previous_description() {
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
            Some("Original description"),
        )
        .unwrap();

    store
        .update_with_options(
            &id,
            BTreeMap::new(),
            None,
            None,
            Some("New description"),
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
        Some("New description")
    );

    let revisions = store.get_history(&id).unwrap();
    let previous = &revisions[revisions.len() - 2];
    let mut revert_fields = previous.fields.clone();
    if let Some(desc_val) = revisions[revisions.len() - 1]
        .fields
        .get(crate::storage::store::DESCRIPTION_HISTORY_KEY)
    {
        revert_fields.insert(
            crate::storage::store::DESCRIPTION_HISTORY_KEY.to_string(),
            desc_val.clone(),
        );
    }
    store.apply_revert(&id, revert_fields, None).unwrap();

    assert_eq!(
        crate::storage::ticket_fs::TicketFs::read_description(&path)
            .unwrap()
            .as_deref(),
        Some("Original description"),
        "undo must restore the pre-overwrite description, making it recoverable"
    );
}

#[test]
pub(super) fn reverting_to_absent_description_removes_file_instead_of_creating_empty_file() {
    let dir = tempdir().unwrap();
    let store = TicketStore::init(dir.path()).unwrap();
    let id = store
        .create(
            None,
            "tracker-improvement",
            Some("Absent description restore"),
            Some("planning"),
            Default::default(),
            None,
            None,
        )
        .unwrap();
    store
        .update_with_options(
            &id,
            BTreeMap::new(),
            None,
            None,
            Some("temporary description"),
            Some(DescriptionUpdateMode::Replace),
            None,
            false,
        )
        .unwrap();

    let indexed = store.get_indexed(&id).unwrap().unwrap();
    let revisions = store.get_history(&id).unwrap();
    let mut revert_fields = revisions[revisions.len() - 2].fields.clone();
    revert_fields.insert(
        crate::storage::store::DESCRIPTION_HISTORY_KEY.to_string(),
        Value::Null,
    );
    store.apply_revert(&id, revert_fields, None).unwrap();

    assert!(
        !indexed.path.join("description.md").exists(),
        "restoring a missing prior description must remove the file"
    );
}

#[test]
pub(super) fn ticket_3d952036_omitted_description_mode_is_rejected() {
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
            Some("Original description"),
        )
        .unwrap();

    // AC1: `description` with no `description_mode` is a hard error, not a
    // silent default.
    let err = store
        .update_with_options(
            &id,
            BTreeMap::new(),
            None,
            None,
            Some("New description"),
            None,
            None,
            false,
        )
        .unwrap_err();

    // AC2: the error names both modes and states which preserves content.
    let message = err.to_string();
    assert!(
        message.contains("replace"),
        "error must name 'replace': {message}"
    );
    assert!(
        message.contains("append"),
        "error must name 'append': {message}"
    );
    assert!(
        message.to_lowercase().contains("preserv"),
        "error must state which mode preserves existing content: {message}"
    );

    // The rejected write must not have touched the description.
    let path = store.get_indexed(&id).unwrap().unwrap().path;
    assert_eq!(
        crate::storage::ticket_fs::TicketFs::read_description(&path)
            .unwrap()
            .as_deref(),
        Some("Original description"),
        "a rejected write must leave the existing description untouched"
    );
}

/// Rework of ticket 3d952036 (AC5): a prior iteration validated the omitted
/// -mode case only at runtime inside `apply_manifest_update`. This test
/// proves the *boundary* type (`DescriptionUpdate`, threaded through
/// `UpdateTicketBody`/`UpdateArgs`/the MCP handler decode) makes "content
/// without a mode" unrepresentable: there is no `description_mode` field on
/// those types to omit, only a single `description_update: DescriptionUpdate`
/// field whose three variants (`Unchanged`/`Replace(String)`/`Append(String)`)
/// each already encode a complete, valid combination.
///
/// This is an exhaustiveness-dependent unit test, not a compile-fail
/// (trybuild) test — the repository has no trybuild dependency, and this
/// ticket's scope note says to check before introducing a new one. The
/// compile-time guarantee itself is structural: `DescriptionUpdate::as_parts`
/// matches all three variants with no catch-all arm, so adding a fourth
/// variant without updating every match site (including this test) is a
/// compiler error, and no Rust code can construct
/// `DescriptionUpdate::Replace`/`Append` without also supplying content, nor
/// omit a mode when constructing a variant that carries content.
#[test]
pub(super) fn ticket_3d952036_description_update_makes_missing_mode_unrepresentable() {
    // Boundary decode still rejects a raw wire `description` with no
    // `description_mode` string (AC1/AC2), but the value produced by a
    // *successful* decode is always one of three fully-formed variants —
    // there is no fourth "content, no mode" shape to construct.
    let err = DescriptionUpdate::decode(Some("New description".to_string()), None).unwrap_err();
    assert_eq!(err, REQUIRED_DESCRIPTION_MODE_ERROR);

    // Every successful decode round-trips through `as_parts` into exactly
    // the `(content, mode)` pair the two are paired on — Replace/Append
    // always carry both, Unchanged always carries neither.
    let unchanged = DescriptionUpdate::decode(None, None).unwrap();
    assert_eq!(unchanged, DescriptionUpdate::Unchanged);
    assert_eq!(unchanged.as_parts(), (None, None));

    let replace = DescriptionUpdate::decode(Some("R".to_string()), Some("replace")).unwrap();
    assert_eq!(replace, DescriptionUpdate::Replace("R".to_string()));
    assert_eq!(
        replace.as_parts(),
        (Some("R"), Some(DescriptionUpdateMode::Replace))
    );

    let append = DescriptionUpdate::decode(Some("A".to_string()), Some("append")).unwrap();
    assert_eq!(append, DescriptionUpdate::Append("A".to_string()));
    assert_eq!(
        append.as_parts(),
        (Some("A"), Some(DescriptionUpdateMode::Append))
    );

    // An unrecognized mode string is also rejected at the decode boundary,
    // never silently coerced into a variant.
    let bad = DescriptionUpdate::decode(Some("x".to_string()), Some("overwrite")).unwrap_err();
    assert!(
        bad.contains("invalid description_mode"),
        "unrecognized mode must be named in the error: {bad}"
    );
}
