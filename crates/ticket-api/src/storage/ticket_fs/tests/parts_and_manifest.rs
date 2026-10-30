#[allow(unused_imports)]
use super::*;

#[test]
pub(super) fn update_with_null_patch_value_removes_the_key() {
    let (_dir, ticket_path) = make_ticket_dir();
    let mut patch = BTreeMap::new();
    patch.insert(
        "handoff_package".to_string(),
        Value::String("stale data".to_string()),
    );
    TicketFs::update(&ticket_path, &patch, None).unwrap();

    let mut delete_patch = BTreeMap::new();
    delete_patch.insert("handoff_package".to_string(), Value::Null);
    let manifest = TicketFs::update(&ticket_path, &delete_patch, None).unwrap();

    assert!(
        !manifest.extra.contains_key("handoff_package"),
        "null patch value must delete the key, not corrupt it to an empty string"
    );
    let reloaded = TicketFs::read(&ticket_path).unwrap();
    assert!(
        !reloaded.extra.contains_key("handoff_package"),
        "deletion must persist across a reload from disk"
    );
}

#[test]
pub(super) fn update_with_explicit_empty_string_is_not_treated_as_deletion() {
    let (_dir, ticket_path) = make_ticket_dir();
    let mut patch = BTreeMap::new();
    patch.insert("notes".to_string(), Value::String(String::new()));
    let manifest = TicketFs::update(&ticket_path, &patch, None).unwrap();

    assert_eq!(
        manifest.extra.get("notes"),
        Some(&Value::String(String::new())),
        "an explicit empty string must be preserved, not deleted"
    );
}

#[test]
pub(super) fn legacy_ticket_with_no_parts_table_loads_implicit_objective() {
    let (_dir, ticket_path) = make_ticket_dir();
    std::fs::write(
        ticket_path.join("description.md"),
        "Original objective text",
    )
    .unwrap();
    let manifest = TicketFs::read(&ticket_path).unwrap();
    assert!(
        manifest.parts().is_empty(),
        "legacy manifest should have no [[parts]] entries"
    );

    let report = TicketFs::load_parts(&ticket_path, &manifest).unwrap();

    assert_eq!(report.parts.len(), 1);
    let objective = &report.parts[0];
    assert_eq!(objective.kind, "objective");
    assert!(objective.implicit);
    assert_eq!(objective.content, "Original objective text");
    assert!(report.orphans.is_empty());

    // The synthetic id must be stable across independent reads.
    let report_again = TicketFs::load_parts(&ticket_path, &manifest).unwrap();
    assert_eq!(report_again.parts[0].id, objective.id);
}

#[test]
pub(super) fn parts_are_addressed_by_id_not_kind_or_index() {
    let (_dir, ticket_path) = make_ticket_dir();
    let mut manifest = TicketFs::read(&ticket_path).unwrap();

    let review_a = TicketPart::new("review", "parts/review-a.md");
    let review_b = TicketPart::new("review", "parts/review-b.md");
    write_part_file(&ticket_path, &review_a, "first pass");
    write_part_file(&ticket_path, &review_b, "second pass");
    manifest.set_parts(vec![review_a.clone(), review_b.clone()]);
    std::fs::write(
        ticket_path.join("ticket.toml"),
        crate::model::manifest_format::format_manifest_toml(&manifest),
    )
    .unwrap();

    let manifest = TicketFs::read(&ticket_path).unwrap();
    let report = TicketFs::load_parts(&ticket_path, &manifest).unwrap();

    // Two parts of the same kind: distinct ids keep them individually
    // addressable and manifest order is preserved.
    assert_eq!(report.parts.len(), 2);
    assert_eq!(report.parts[0].id, review_a.id);
    assert_eq!(report.parts[1].id, review_b.id);
    assert_eq!(report.find(review_a.id).unwrap().content, "first pass");
    assert_eq!(report.find(review_b.id).unwrap().content, "second pass");
    assert_eq!(report.of_kind("review").count(), 2);
    assert!(report.find(uuid::Uuid::new_v4()).is_none());
}

#[test]
pub(super) fn orphan_part_file_is_reported_not_silently_adopted() {
    let (_dir, ticket_path) = make_ticket_dir();
    let mut manifest = TicketFs::read(&ticket_path).unwrap();

    let objective = TicketPart::new("objective", "parts/objective.md");
    write_part_file(&ticket_path, &objective, "the objective");
    manifest.set_parts(vec![objective.clone()]);
    std::fs::write(
        ticket_path.join("ticket.toml"),
        crate::model::manifest_format::format_manifest_toml(&manifest),
    )
    .unwrap();

    // A file dropped into parts/ with no manifest entry.
    std::fs::write(
        ticket_path.join("parts").join("stray.md"),
        "not indexed anywhere",
    )
    .unwrap();

    let manifest = TicketFs::read(&ticket_path).unwrap();
    let report = TicketFs::load_parts(&ticket_path, &manifest).unwrap();

    assert_eq!(report.parts.len(), 1, "only the indexed part loads");
    assert_eq!(report.orphans.len(), 1);
    assert_eq!(
        report.orphans[0].file_name().unwrap().to_str().unwrap(),
        "stray.md"
    );
}

#[test]
pub(super) fn unknown_kind_part_round_trips_as_opaque_attachment() {
    let (_dir, ticket_path) = make_ticket_dir();
    let mut manifest = TicketFs::read(&ticket_path).unwrap();

    let attachment = TicketPart::new("handoff_package", "parts/handoff.md");
    write_part_file(&ticket_path, &attachment, "opaque payload");
    manifest.set_parts(vec![attachment.clone()]);
    std::fs::write(
        ticket_path.join("ticket.toml"),
        crate::model::manifest_format::format_manifest_toml(&manifest),
    )
    .unwrap();

    let manifest = TicketFs::read(&ticket_path).unwrap();
    let report = TicketFs::load_parts(&ticket_path, &manifest).unwrap();

    assert_eq!(report.parts.len(), 1);
    let loaded = &report.parts[0];
    assert_eq!(loaded.kind, "handoff_package");
    assert_eq!(loaded.content, "opaque payload");
    assert!(
        !crate::model::parts::is_core_part_kind(&loaded.kind),
        "unrecognised kinds must not be treated as core"
    );
}

#[test]
pub(super) fn manifest_parts_table_round_trips_through_write_manifest_and_read() {
    let (_dir, ticket_path) = make_ticket_dir();
    let mut manifest = TicketFs::read(&ticket_path).unwrap();

    let objective = TicketPart::new("objective", "parts/objective.md");
    let mut amendment = TicketPart::new("amendment", "parts/amend-1.md");
    amendment.supersedes = Some(objective.id);
    amendment.frozen = false;
    manifest.set_parts(vec![objective.clone(), amendment.clone()]);

    // Exercise the exact write path `TicketFs` uses: format then write.
    std::fs::write(
        ticket_path.join("ticket.toml"),
        crate::model::manifest_format::format_manifest_toml(&manifest),
    )
    .unwrap();

    let reloaded = TicketFs::read(&ticket_path).unwrap();
    let parts = reloaded.parts();
    assert_eq!(parts.len(), 2, "the [[parts]] table must not be dropped");
    assert_eq!(parts[0], objective);
    assert_eq!(parts[1], amendment);
    assert_eq!(parts[1].supersedes, Some(objective.id));
}

#[test]
pub(super) fn ticket_fs_update_preserves_parts_table_when_patching_other_fields() {
    let (_dir, ticket_path) = make_ticket_dir();
    let mut manifest = TicketFs::read(&ticket_path).unwrap();
    let objective = TicketPart::new("objective", "parts/objective.md");
    manifest.set_parts(vec![objective.clone()]);
    std::fs::write(
        ticket_path.join("ticket.toml"),
        crate::model::manifest_format::format_manifest_toml(&manifest),
    )
    .unwrap();

    let mut patch = BTreeMap::new();
    patch.insert("priority".to_string(), Value::String("high".to_string()));
    let updated = TicketFs::update(&ticket_path, &patch, None).unwrap();

    assert_eq!(
        updated.parts(),
        vec![objective],
        "an unrelated field patch must not drop the [[parts]] table"
    );
}
