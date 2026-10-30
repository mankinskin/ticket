use std::{collections::BTreeMap, fs};

use memory_kernel::workspace_policy::WORKSPACE_POLICY_FILE;
use tempfile::tempdir;

use crate::{storage::store::TicketStore, workflow::WorkflowModel};

fn extra_with_effort(value: &str) -> BTreeMap<String, serde_json::Value> {
    let mut extra = BTreeMap::new();
    extra.insert(
        "effort".to_string(),
        serde_json::Value::String(value.to_string()),
    );
    extra
}

fn open_store() -> (tempfile::TempDir, TicketStore) {
    let dir = tempdir().unwrap();
    let store = TicketStore::init(dir.path()).unwrap();
    (dir, store)
}

#[test]
fn no_findings_for_ticket_with_good_description() {
    let (_dir, store) = open_store();
    let linked_parent = store
        .create(
            None,
            "tracker-improvement",
            Some("My well-described ticket"),
            Some("planning"),
            extra_with_effort("1200"),
            None,
            Some("This description is definitely long enough to pass the 50-character threshold."),
        )
        .unwrap();
    let id = store
        .create(
            None,
            "tracker-improvement",
            Some("My well-described dependent ticket"),
            Some("planning"),
            extra_with_effort("800"),
            None,
            Some("This description is definitely long enough to pass the 50-character threshold."),
        )
        .unwrap();
    store
        .add_edge(crate::model::edge::EdgeRecord {
            from: linked_parent,
            to: id,
            kind: "depends_on".to_string(),
            created_at: chrono::Utc::now(),
        })
        .unwrap();

    let tickets = store.list(None, None, None).unwrap();
    let edges = store.list_all_edges().unwrap();
    let workflow = WorkflowModel::build(&store, tickets.clone(), edges.clone()).unwrap();
    let report = super::collect_findings(&store, &tickets, &edges, &workflow);

    let ticket_findings: Vec<_> = report
        .findings
        .iter()
        .filter(|f| f.ticket_id == id)
        .collect();
    assert!(
        ticket_findings.is_empty(),
        "expected no findings, got {ticket_findings:?}"
    );
}

#[test]
fn unknown_type_produces_error_finding() {
    let (_dir, store) = open_store();
    let id = store
        .create(
            None,
            "tracker-improvement",
            Some("Ticket with an unregistered type"),
            Some("planning"),
            extra_with_effort("500"),
            None,
            Some("This description is definitely long enough to pass the 50-character threshold."),
        )
        .unwrap();

    // `create` now rejects unregistered types outright, so simulate
    // legacy data whose type predates schema registration by editing the
    // manifest on disk directly and reindexing, mirroring how existing
    // unregistered-type tickets already on disk came to exist.
    let mut manifest = store.get(&id).unwrap();
    manifest.extra.insert(
        "type".to_string(),
        serde_json::Value::String("made-up-type".to_string()),
    );
    let ticket_path = store.get_indexed(&id).unwrap().unwrap().path;
    let toml_str = memory_kernel::model::manifest_format::format_manifest_toml(&manifest);
    fs::write(
        ticket_path.join(crate::model::filesystem::TICKET_MANIFEST_FILE),
        toml_str,
    )
    .unwrap();
    store.scan(true).unwrap();

    let tickets = store.list(None, None, None).unwrap();
    let edges = store.list_all_edges().unwrap();
    let workflow = WorkflowModel::build(&store, tickets.clone(), edges.clone()).unwrap();
    let report = super::collect_findings(&store, &tickets, &edges, &workflow);

    let finding = report
        .findings
        .iter()
        .find(|f| f.ticket_id == id && f.check == "unknown_type")
        .expect("expected unknown_type finding");
    assert_eq!(finding.severity, "error");
    assert!(finding.message.contains("made-up-type"));
    assert_eq!(*report.summary.get("unknown_type").unwrap_or(&0), 1);
}

#[test]
fn off_schema_state_produces_finding() {
    let (_dir, store) = open_store();
    // Reproduce legacy persisted data, which predates create-time state
    // validation, by patching a valid manifest and reindexing it.
    let id = store
        .create(
            None,
            "tracker-improvement",
            Some("Ticket frozen in an off-schema state"),
            Some("planning"),
            extra_with_effort("500"),
            None,
            Some("This description is definitely long enough to pass the 50-character threshold."),
        )
        .unwrap();
    let mut manifest = store.get(&id).unwrap();
    manifest.extra.insert(
        "state".to_string(),
        serde_json::Value::String("archived".to_string()),
    );
    let ticket_path = store.get_indexed(&id).unwrap().unwrap().path;
    let toml_str = memory_kernel::model::manifest_format::format_manifest_toml(&manifest);
    fs::write(
        ticket_path.join(crate::model::filesystem::TICKET_MANIFEST_FILE),
        toml_str,
    )
    .unwrap();
    store.scan(true).unwrap();

    let tickets = store.list(None, None, None).unwrap();
    let edges = store.list_all_edges().unwrap();
    let workflow = WorkflowModel::build(&store, tickets.clone(), edges.clone()).unwrap();
    let report = super::collect_findings(&store, &tickets, &edges, &workflow);

    let finding = report
        .findings
        .iter()
        .find(|f| f.ticket_id == id && f.check == "off_schema_state")
        .expect("expected off_schema_state finding");
    assert_eq!(finding.severity, "error");
    assert!(finding.message.contains("archived"));
    assert_eq!(finding.ticket_id, id);
    assert_eq!(*report.summary.get("off_schema_state").unwrap_or(&0), 1);
}

#[test]
fn task_type_has_registered_schema_and_no_unknown_type_finding() {
    let (_dir, store) = open_store();
    let id = store
        .create(
            None,
            "task",
            Some("A task-typed ticket"),
            Some("planning"),
            extra_with_effort("500"),
            None,
            Some("This description is definitely long enough to pass the 50-character threshold."),
        )
        .unwrap();

    let tickets = store.list(None, None, None).unwrap();
    let edges = store.list_all_edges().unwrap();
    let workflow = WorkflowModel::build(&store, tickets.clone(), edges.clone()).unwrap();
    let report = super::collect_findings(&store, &tickets, &edges, &workflow);

    assert!(
        !report
            .findings
            .iter()
            .any(|f| f.ticket_id == id && f.check == "unknown_type"),
        "task type must resolve to a schema"
    );
}

#[test]
fn missing_description_produces_warning_not_error() {
    let (_dir, store) = open_store();
    let id = store
        .create(
            None,
            "tracker-improvement",
            Some("Ticket with no description"),
            Some("planning"),
            extra_with_effort("900"),
            None,
            None,
        )
        .unwrap();

    let tickets = store.list(None, None, None).unwrap();
    let edges = store.list_all_edges().unwrap();
    let workflow = WorkflowModel::build(&store, tickets.clone(), edges.clone()).unwrap();
    let report = super::collect_findings(&store, &tickets, &edges, &workflow);

    let finding = report
        .findings
        .iter()
        .find(|f| f.ticket_id == id && f.check == "missing_description")
        .expect("expected missing_description finding");
    assert_eq!(
        finding.severity, "warning",
        "severity must be 'warning', not 'error'"
    );
    assert!(
        finding.message.contains("description.md"),
        "message must mention description.md"
    );
    assert_eq!(*report.summary.get("missing_description").unwrap_or(&0), 1);
}

#[test]
fn short_description_produces_info_finding() {
    let (_dir, store) = open_store();
    let id = store
        .create(
            None,
            "tracker-improvement",
            Some("Ticket with terse description"),
            Some("planning"),
            extra_with_effort("600"),
            None,
            Some("Short."),
        )
        .unwrap();

    let tickets = store.list(None, None, None).unwrap();
    let edges = store.list_all_edges().unwrap();
    let workflow = WorkflowModel::build(&store, tickets.clone(), edges.clone()).unwrap();
    let report = super::collect_findings(&store, &tickets, &edges, &workflow);

    let finding = report
        .findings
        .iter()
        .find(|f| f.ticket_id == id && f.check == "short_description")
        .expect("expected short_description finding");
    assert_eq!(finding.severity, "info");
}

#[test]
fn done_ticket_skips_non_topology_checks() {
    let (_dir, store) = open_store();
    let done = store
        .create(
            None,
            "tracker-improvement",
            Some("Finished ticket"),
            Some("done"),
            BTreeMap::new(),
            None,
            None,
        )
        .unwrap();
    let linked = store
        .create(
            None,
            "tracker-improvement",
            Some("Linked ticket"),
            Some("done"),
            BTreeMap::new(),
            None,
            None,
        )
        .unwrap();
    store
        .add_edge(crate::model::edge::EdgeRecord {
            from: done,
            to: linked,
            kind: "depends_on".to_string(),
            created_at: chrono::Utc::now(),
        })
        .unwrap();

    let tickets = store.list(None, None, None).unwrap();
    let edges = store.list_all_edges().unwrap();
    let workflow = WorkflowModel::build(&store, tickets.clone(), edges.clone()).unwrap();
    let report = super::collect_findings(&store, &tickets, &edges, &workflow);

    let ticket_findings: Vec<_> = report
        .findings
        .iter()
        .filter(|f| f.ticket_id == done)
        .collect();
    assert!(
        ticket_findings.is_empty(),
        "done ticket should skip non-topology checks, got {ticket_findings:?}"
    );
}

#[test]
fn orphan_ticket_produces_graph_participation_finding() {
    let (_dir, store) = open_store();
    let id = store
        .create(
            None,
            "tracker-improvement",
            Some("Orphan ticket"),
            Some("in-implementation"),
            extra_with_effort("1500"),
            None,
            Some("This description is definitely long enough to pass the 50-character threshold."),
        )
        .unwrap();

    let tickets = store.list(None, None, None).unwrap();
    let edges = store.list_all_edges().unwrap();
    let workflow = WorkflowModel::build(&store, tickets.clone(), edges.clone()).unwrap();
    let report = super::collect_findings(&store, &tickets, &edges, &workflow);

    let finding = report
        .findings
        .iter()
        .find(|f| f.ticket_id == id && f.check == "graph_participation")
        .expect("expected graph_participation finding");
    assert_eq!(finding.severity, "warning");
    assert!(
        finding.message.contains("depends_on graph"),
        "message must mention depends_on graph"
    );
}

#[test]
fn missing_effort_estimation_produces_warning() {
    let (_dir, store) = open_store();
    let parent = store
        .create(
            None,
            "tracker-improvement",
            Some("Parent"),
            Some("planning"),
            extra_with_effort("500"),
            None,
            Some("This description is definitely long enough to pass the 50-character threshold."),
        )
        .unwrap();
    let id = store
        .create(
            None,
            "tracker-improvement",
            Some("Missing effort"),
            Some("planning"),
            BTreeMap::new(),
            None,
            Some("This description is definitely long enough to pass the 50-character threshold."),
        )
        .unwrap();
    store
        .add_edge(crate::model::edge::EdgeRecord {
            from: parent,
            to: id,
            kind: "depends_on".to_string(),
            created_at: chrono::Utc::now(),
        })
        .unwrap();

    let tickets = store.list(None, None, None).unwrap();
    let edges = store.list_all_edges().unwrap();
    let workflow = WorkflowModel::build(&store, tickets.clone(), edges.clone()).unwrap();
    let report = super::collect_findings(&store, &tickets, &edges, &workflow);

    let finding = report
        .findings
        .iter()
        .find(|f| f.ticket_id == id && f.check == "missing_effort_estimation")
        .expect("expected missing_effort_estimation finding");
    assert_eq!(finding.severity, "warning");
    assert!(
        finding.message.contains("effort estimation"),
        "message must mention effort estimation"
    );
}

#[test]
fn cross_workspace_dependency_is_warning_not_dangling_error() {
    let dir = tempdir().unwrap();
    let root_repo = dir.path().join("repo");
    let child_repo = root_repo.join("child");
    fs::create_dir_all(&child_repo).unwrap();

    let parent_store = TicketStore::init(&root_repo).unwrap();
    let child_store = TicketStore::init(&child_repo).unwrap();

    fs::create_dir_all(&child_store.index_root).unwrap();
    fs::write(
        child_store.index_root.join(WORKSPACE_POLICY_FILE),
        "include_descendants = true\ninclude_ancestors = true\ndeny_external_paths = true\n",
    )
    .unwrap();

    let parent_id = parent_store
        .create(
            None,
            "tracker-improvement",
            Some("Parent prerequisite"),
            Some("planning"),
            extra_with_effort("1200"),
            None,
            Some("This description is definitely long enough to pass the 50-character threshold."),
        )
        .unwrap();
    assert!(
        parent_store
            .get_indexed(&parent_id)
            .unwrap()
            .unwrap()
            .path
            .join("ticket.toml")
            .is_file()
    );

    let child_id = child_store
        .create(
            None,
            "tracker-improvement",
            Some("Child ticket"),
            Some("planning"),
            extra_with_effort("800"),
            None,
            Some("This description is definitely long enough to pass the 50-character threshold."),
        )
        .unwrap();

    let classifier = memory_kernel::cross_store_edges::CrossStoreEdgeClassifier::for_store(
        &child_store.index_root,
        memory_kernel::ContentKind::Ticket,
        crate::workspace_policy::load_workspace_policy(&child_repo),
    )
    .unwrap();
    assert!(matches!(
        classifier.classify(parent_id),
        memory_kernel::cross_store_edges::EdgeReferenceResolution::CrossWorkspaceEdge { .. }
    ));

    let tickets = child_store.list(None, None, None).unwrap();
    let edges = vec![crate::model::edge::EdgeRecord {
        from: child_id,
        to: parent_id,
        kind: "depends_on".to_string(),
        created_at: chrono::Utc::now(),
    }];
    let workflow = WorkflowModel::build(&child_store, tickets.clone(), Vec::new()).unwrap();
    let report = super::collect_findings(&child_store, &tickets, &edges, &workflow);

    assert!(
        report
            .findings
            .iter()
            .any(|f| { f.ticket_id == child_id && f.check == "cross_workspace_edge" })
    );
    assert!(
        !report
            .findings
            .iter()
            .any(|f| { f.ticket_id == child_id && f.check == "dangling_edge" })
    );
}
