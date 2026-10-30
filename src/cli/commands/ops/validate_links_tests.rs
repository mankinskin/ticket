use std::collections::BTreeMap;

use spec_api::{SpecManifest, SpecStore, TicketRef};
use tempfile::TempDir;
use ticket_api::model::ticket::SpecRef;

use super::{
    TicketStore, cmd_validate_links, resolve_reference_owner_root, resolve_referenced_spec_root,
    try_get_spec,
};

/// Reproduces the nested-store bug: a ticket's `related_specs` entry
/// carries a `store_root` that does not resolve to any store, while the
/// referenced spec actually exists under the workspace's canonical
/// `.spec` store. Before structured `SpecRef` carried an explicit store
/// identifier, a relative-path prose link like this could silently
/// resolve against the wrong (or a nonexistent) store instead of being
/// flagged. `validate-links` must detect it as `wrong_store_ref`.
#[test]
fn detects_wrong_store_ref_for_nested_store_bug_scenario() {
    let workspace = TempDir::new().unwrap();
    let workspace_root = workspace.path();

    let ticket_store = TicketStore::init(workspace_root).unwrap();
    let mut spec_store = SpecStore::init(workspace_root).unwrap();

    let spec_manifest = SpecManifest::new(
        "traceability/nested-store-bug",
        "Nested store bug spec",
        "ticket-api",
    );
    let spec_id = spec_manifest.id();
    spec_store.create(&spec_manifest, "body", None).unwrap();

    let ticket_id = ticket_store
        .create(
            None,
            "task",
            Some("Nested store bug ticket"),
            None,
            BTreeMap::new(),
            None,
            None,
        )
        .unwrap();

    // Wrong: the spec actually lives under the canonical `.spec` store,
    // but this ref claims a nonexistent nested store.
    let wrong_spec_ref = SpecRef {
        spec_id,
        workspace: "default".to_string(),
        store_root: "nested/.spec".to_string(),
    };
    let mut patch = BTreeMap::new();
    patch.insert(
        "related_specs".to_string(),
        serde_json::to_value(vec![wrong_spec_ref]).unwrap(),
    );
    ticket_store
        .update(&ticket_id, patch, None, None, None, None)
        .unwrap();

    let result = cmd_validate_links(&ticket_store, workspace_root).unwrap();

    assert_eq!(result["valid"], false);
    assert_eq!(result["checked"], 1);
    let findings = result["findings"].as_array().unwrap();
    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0]["kind"], "wrong_store_ref");
    assert_eq!(findings[0]["spec_id"], spec_id.to_string());
}

#[test]
fn resolves_reference_owner_independently_of_invocation_root() {
    let aggregate = TempDir::new().unwrap();
    let aggregate_root = aggregate.path();
    let owner_root = aggregate_root.join("workflow-tools");
    std::fs::create_dir_all(&owner_root).unwrap();

    let aggregate_ticket_store = TicketStore::init(aggregate_root).unwrap();
    let owner_ticket_store = TicketStore::init(&owner_root).unwrap();
    let aggregate_ticket_id = aggregate_ticket_store
        .create(
            None,
            "task",
            Some("Aggregate invocation ticket"),
            None,
            BTreeMap::new(),
            None,
            None,
        )
        .unwrap();
    let owner_ticket_id = owner_ticket_store
        .create(
            None,
            "task",
            Some("Nested invocation ticket"),
            None,
            BTreeMap::new(),
            None,
            None,
        )
        .unwrap();

    let mut spec_manifest = SpecManifest::new(
        "traceability/owner-root-resolution",
        "Owner root resolution spec",
        "ticket-api",
    );
    let spec_id = spec_manifest.id();
    spec_manifest.set_related_tickets(vec![
        TicketRef {
            ticket_id: aggregate_ticket_id,
            workspace: "default".to_string(),
            store_root: ".workflow-tools/ticket".to_string(),
        },
        TicketRef {
            ticket_id: owner_ticket_id,
            workspace: "workflow-tools".to_string(),
            store_root: ".workflow-tools/ticket".to_string(),
        },
    ]);
    let mut spec_store = SpecStore::init(&owner_root).unwrap();
    spec_store.create(&spec_manifest, "body", None).unwrap();

    let spec_ref = SpecRef {
        spec_id,
        workspace: "workflow-tools".to_string(),
        store_root: ".workflow-tools/spec".to_string(),
    };
    let resolved_owner_root =
        resolve_reference_owner_root(aggregate_root, &spec_ref.workspace).unwrap();
    let resolved_spec_root =
        resolve_referenced_spec_root(&resolved_owner_root, &spec_ref.store_root).unwrap();
    assert_eq!(
        resolved_spec_root,
        owner_root
            .canonicalize()
            .unwrap()
            .join(".workflow-tools/spec")
    );
    assert_eq!(
        try_get_spec(&resolved_spec_root, spec_id).unwrap().id(),
        spec_id
    );

    for (ticket_store, ticket_id) in [
        (&aggregate_ticket_store, aggregate_ticket_id),
        (&owner_ticket_store, owner_ticket_id),
    ] {
        let mut patch = BTreeMap::new();
        patch.insert(
            "related_specs".to_string(),
            serde_json::to_value(vec![spec_ref.clone()]).unwrap(),
        );
        ticket_store
            .update(&ticket_id, patch, None, None, None, None)
            .unwrap();
    }

    let aggregate_result = cmd_validate_links(&aggregate_ticket_store, aggregate_root).unwrap();
    let nested_result = cmd_validate_links(&owner_ticket_store, aggregate_root).unwrap();

    for result in [&aggregate_result, &nested_result] {
        assert_eq!(result["checked"], 1);
        assert_eq!(result["valid"], true);
        assert_eq!(result["findings"].as_array().unwrap().len(), 0);
    }
}

#[test]
fn detects_dangling_spec_ref() {
    let workspace = TempDir::new().unwrap();
    let workspace_root = workspace.path();

    let ticket_store = TicketStore::init(workspace_root).unwrap();
    SpecStore::init(workspace_root).unwrap();

    let ticket_id = ticket_store
        .create(
            None,
            "task",
            Some("Dangling spec ref ticket"),
            None,
            BTreeMap::new(),
            None,
            None,
        )
        .unwrap();

    let dangling_ref = SpecRef {
        spec_id: uuid::Uuid::new_v4(),
        workspace: "default".to_string(),
        store_root: ".spec".to_string(),
    };
    let mut patch = BTreeMap::new();
    patch.insert(
        "related_specs".to_string(),
        serde_json::to_value(vec![dangling_ref]).unwrap(),
    );
    ticket_store
        .update(&ticket_id, patch, None, None, None, None)
        .unwrap();

    let result = cmd_validate_links(&ticket_store, workspace_root).unwrap();

    assert_eq!(result["valid"], false);
    let findings = result["findings"].as_array().unwrap();
    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0]["kind"], "dangling_spec_ref");
}

#[test]
fn detects_bidirectional_inconsistency() {
    let workspace = TempDir::new().unwrap();
    let workspace_root = workspace.path();

    let ticket_store = TicketStore::init(workspace_root).unwrap();
    let mut spec_store = SpecStore::init(workspace_root).unwrap();

    let spec_manifest =
        SpecManifest::new("traceability/no-back-ref", "No back ref spec", "ticket-api");
    let spec_id = spec_manifest.id();
    spec_store.create(&spec_manifest, "body", None).unwrap();
    // Deliberately do not set the spec's related_tickets back-reference.

    let ticket_id = ticket_store
        .create(
            None,
            "task",
            Some("One-way link ticket"),
            None,
            BTreeMap::new(),
            None,
            None,
        )
        .unwrap();

    let spec_ref = SpecRef {
        spec_id,
        workspace: "default".to_string(),
        store_root: ".workflow-tools/spec".to_string(),
    };
    let mut patch = BTreeMap::new();
    patch.insert(
        "related_specs".to_string(),
        serde_json::to_value(vec![spec_ref]).unwrap(),
    );
    ticket_store
        .update(&ticket_id, patch, None, None, None, None)
        .unwrap();

    let result = cmd_validate_links(&ticket_store, workspace_root).unwrap();

    assert_eq!(result["valid"], false);
    let findings = result["findings"].as_array().unwrap();
    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0]["kind"], "bidirectional_inconsistency");
}

#[test]
fn valid_when_bidirectional_link_is_consistent() {
    let workspace = TempDir::new().unwrap();
    let workspace_root = workspace.path();

    let ticket_store = TicketStore::init(workspace_root).unwrap();
    let mut spec_store = SpecStore::init(workspace_root).unwrap();

    let mut spec_manifest = SpecManifest::new(
        "traceability/consistent-link",
        "Consistent link spec",
        "ticket-api",
    );
    let spec_id = spec_manifest.id();

    let ticket_id = ticket_store
        .create(
            None,
            "task",
            Some("Consistent link ticket"),
            None,
            BTreeMap::new(),
            None,
            None,
        )
        .unwrap();

    spec_manifest.set_related_tickets(vec![TicketRef {
        ticket_id,
        workspace: "default".to_string(),
        store_root: ".ticket".to_string(),
    }]);
    spec_store.create(&spec_manifest, "body", None).unwrap();

    let spec_ref = SpecRef {
        spec_id,
        workspace: "default".to_string(),
        store_root: ".workflow-tools/spec".to_string(),
    };
    let mut patch = BTreeMap::new();
    patch.insert(
        "related_specs".to_string(),
        serde_json::to_value(vec![spec_ref]).unwrap(),
    );
    ticket_store
        .update(&ticket_id, patch, None, None, None, None)
        .unwrap();

    let result = cmd_validate_links(&ticket_store, workspace_root).unwrap();

    assert_eq!(result["valid"], true);
    assert_eq!(result["findings"].as_array().unwrap().len(), 0);
}
