#[allow(unused_imports)]
use super::*;

// ── typed [[refs]] (spec 24b3d22b, ticket 9d69e93d) ──────────────────────────

#[test]
pub(super) fn write_ref_appends_and_round_trips_all_six_kinds() {
    let (_dir, ticket_path) = make_ticket_dir();

    let entries = vec![
        (
            "spec",
            format!("ce://default/spec/{}", uuid::Uuid::new_v4()),
            Some("contract".to_string()),
        ),
        (
            "test_execution",
            "ce://default/test-execution/7f2c1a04".to_string(),
            None,
        ),
        ("log", "ce://default/log/build-2026-07-30".to_string(), None),
        (
            "rule",
            format!("ce://default/rule/{}", uuid::Uuid::new_v4()),
            None,
        ),
        (
            "file",
            "memory-api/crates/ticket-api/src/storage/store.rs".to_string(),
            Some("write path".to_string()),
        ),
        ("commit", "abc1234".to_string(), None),
    ];

    for (kind, urn, note) in &entries {
        TicketFs::write_ref(&ticket_path, kind, urn, note.clone()).unwrap();
    }

    let manifest = TicketFs::read(&ticket_path).unwrap();
    let refs = manifest.refs();
    assert_eq!(
        refs.len(),
        6,
        "all six ref kinds must round-trip without loss"
    );
    for (kind, urn, note) in &entries {
        let found = refs
            .iter()
            .find(|r| r.kind == *kind && r.urn == *urn)
            .unwrap_or_else(|| panic!("missing ref kind={kind} urn={urn}"));
        assert_eq!(found.note, *note);
    }
}

#[test]
pub(super) fn write_ref_rejects_unknown_kind_with_vocabulary_in_error() {
    let (_dir, ticket_path) = make_ticket_dir();
    let err = TicketFs::write_ref(&ticket_path, "doc", "anything", None)
        .expect_err("unknown ref kind must be rejected");
    let message = err.to_string();
    assert!(message.contains("doc"));
    for kind in crate::model::refs::REF_KINDS {
        assert!(message.contains(kind), "{message}");
    }
    // Rejected before touching the manifest on disk.
    let manifest = TicketFs::read(&ticket_path).unwrap();
    assert!(manifest.refs().is_empty());
}

#[test]
pub(super) fn write_ref_rejects_malformed_urn_for_kind() {
    let (_dir, ticket_path) = make_ticket_dir();

    assert!(TicketFs::write_ref(&ticket_path, "spec", "not-a-urn", None).is_err());
    assert!(TicketFs::write_ref(&ticket_path, "commit", "zz", None).is_err());
    assert!(TicketFs::write_ref(&ticket_path, "file", "/etc/passwd", None).is_err());

    let manifest = TicketFs::read(&ticket_path).unwrap();
    assert!(
        manifest.refs().is_empty(),
        "no malformed ref should persist"
    );
}

#[test]
pub(super) fn foreign_ref_kind_already_in_manifest_round_trips_unchanged() {
    let (_dir, ticket_path) = make_ticket_dir();
    let mut manifest = TicketFs::read(&ticket_path).unwrap();

    // Simulates a manifest written by a future/foreign tool with a ref kind
    // outside today's closed vocabulary. Reading must never drop or fail on
    // it, even though writing a *new* ref of this kind would be rejected.
    manifest.set_refs(vec![TicketRefEntry {
        kind: "future_kind".to_string(),
        urn: "ce://default/future_kind/whatever".to_string(),
        note: Some("preserved, not dropped".to_string()),
    }]);
    std::fs::write(
        ticket_path.join("ticket.toml"),
        crate::model::manifest_format::format_manifest_toml(&manifest),
    )
    .unwrap();

    let reloaded = TicketFs::read(&ticket_path).unwrap();
    let refs = reloaded.refs();
    assert_eq!(refs.len(), 1);
    assert_eq!(refs[0].kind, "future_kind");
    assert_eq!(refs[0].note, Some("preserved, not dropped".to_string()));
}

#[test]
pub(super) fn legacy_related_specs_extra_bridges_to_refs_with_identical_spec_identity() {
    let (_dir, ticket_path) = make_ticket_dir();
    let mut manifest = TicketFs::read(&ticket_path).unwrap();

    let spec_id = uuid::Uuid::new_v4();
    manifest.set_related_specs(vec![SpecRef {
        spec_id,
        workspace: "default".to_string(),
        store_root: ".spec".to_string(),
    }]);
    std::fs::write(
        ticket_path.join("ticket.toml"),
        crate::model::manifest_format::format_manifest_toml(&manifest),
    )
    .unwrap();

    // No [[refs]] table was ever written for this ticket.
    let reloaded = TicketFs::read(&ticket_path).unwrap();
    assert!(reloaded.extra.get("refs").is_none());

    let refs = reloaded.refs();
    assert_eq!(refs.len(), 1);
    assert_eq!(refs[0].kind, "spec");
    assert_eq!(refs[0].urn, format!("ce://default/spec/{spec_id}"));

    // Diff of resolved spec ids before (legacy `related_specs()`) and after
    // (typed `refs()`) must be empty.
    let legacy_ids: Vec<uuid::Uuid> = reloaded.related_specs().iter().map(|r| r.spec_id).collect();
    let refs_ids: Vec<uuid::Uuid> = refs
        .iter()
        .filter(|r| r.kind == "spec")
        .map(|r| {
            r.urn
                .rsplit('/')
                .next()
                .unwrap()
                .parse::<uuid::Uuid>()
                .unwrap()
        })
        .collect();
    assert_eq!(legacy_ids, refs_ids);
}

#[test]
pub(super) fn remove_ref_deletes_matching_entry_and_leaves_others() {
    let (_dir, ticket_path) = make_ticket_dir();
    TicketFs::write_ref(&ticket_path, "commit", "abc1234", None).unwrap();
    TicketFs::write_ref(&ticket_path, "commit", "def5678", None).unwrap();

    TicketFs::remove_ref(&ticket_path, "commit", "abc1234").unwrap();

    let manifest = TicketFs::read(&ticket_path).unwrap();
    let refs = manifest.refs();
    assert_eq!(refs.len(), 1);
    assert_eq!(refs[0].urn, "def5678");
}

#[test]
pub(super) fn ticket_fs_update_preserves_refs_table_when_patching_other_fields() {
    let (_dir, ticket_path) = make_ticket_dir();
    TicketFs::write_ref(&ticket_path, "commit", "abc1234", None).unwrap();

    let mut patch = BTreeMap::new();
    patch.insert("priority".to_string(), Value::String("high".to_string()));
    let updated = TicketFs::update(&ticket_path, &patch, None).unwrap();

    assert_eq!(
        updated.refs().len(),
        1,
        "an unrelated field patch must not drop [[refs]]"
    );
}
