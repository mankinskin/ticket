#[allow(unused_imports)]
use super::*;

#[test]
pub(super) fn health_all_e2e_reports_timings_on_large_fixture() {
    init_perf_test_tracing();
    let perf = materialize_fixture_with_ticket_perf_load(TicketPerfFixtureOptions {
        root_generated_ticket_count: 96,
        submodule_generated_ticket_count: 24,
        tracked_reference_file_count: 4,
        references_per_file: 10,
    })
    .expect("perf fixture should materialize");

    let run_id = format!("perf-run-{}", Uuid::new_v4());
    let fixture_profile = "root96_sub24_refs4x10";
    let workspace_scope = "ticket-root";
    let reindex_mode = "mixed";
    let change_count = 1 + 10 + 100;

    let root_store = perf
        .fixture
        .store_root("ticket-root")
        .expect("root ticket store path")
        .to_path_buf();

    let (store, open_report) =
        TicketStore::open_or_init_profiled(&root_store).expect("open root store");

    let scan_report = store.scan(true).expect("scan root store");

    append_incremental_fixture_tickets(&root_store, 1, 1);
    let incremental_scan_1 = store.scan(false).expect("incremental scan root store (1)");

    append_incremental_fixture_tickets(&root_store, 2, 10);
    let incremental_scan_10 = store.scan(false).expect("incremental scan root store (10)");

    append_incremental_fixture_tickets(&root_store, 3, 100);
    let incremental_scan_100 = store
        .scan(false)
        .expect("incremental scan root store (100)");

    let ids = parse_ids(&perf.root_ticket_ids);
    add_perf_edges(&store, &ids);

    let list_started = Instant::now();
    let tickets = store.list(None, None, None).expect("list tickets");
    let all_edges = store.list_all_edges().expect("list edges");
    let list_elapsed = list_started.elapsed();

    let workflow_started = Instant::now();
    let workflow =
        WorkflowModel::build(&store, tickets.clone(), all_edges.clone()).expect("build workflow");
    let workflow_elapsed = workflow_started.elapsed();

    let health_started = Instant::now();
    let report = collect_findings(&store, &tickets, &all_edges, &workflow);
    let health_elapsed = health_started.elapsed();

    assert!(!report.findings.is_empty());
    assert!(
        report.summary.contains_key("graph_participation")
            || report.summary.contains_key("missing_effort_estimation")
    );
    assert!(
        open_report
            .phase_timings_ms
            .contains_key("open_or_init_total_ms")
    );
    assert!(
        open_report
            .phase_timings_ms
            .contains_key("open_sqlite_index_ms")
    );
    assert!(scan_report.phase_timings_ms.contains_key("scan_total_ms"));
    assert!(
        scan_report
            .phase_timings_ms
            .contains_key("rebuild_workflow_facts_ms")
    );
    assert!(
        scan_report
            .phase_timings_ms
            .contains_key("integration.manifest_parse_ms")
    );
    assert!(
        scan_report
            .phase_timings_ms
            .contains_key("integration.index_upsert_ms")
    );
    assert!(
        scan_report
            .phase_timings_ms
            .contains_key("integration.edge_write_ms")
    );
    assert!(
        scan_report
            .phase_timings_ms
            .contains_key("integration.description_read_ms")
    );
    assert!(
        scan_report
            .phase_timings_ms
            .contains_key("integration.search_upsert_ms")
    );
    assert!(
        scan_report
            .phase_timings_ms
            .contains_key("workflow.fetch_dependency_edges_ms")
    );
    assert!(
        scan_report
            .phase_timings_ms
            .contains_key("workflow.fetch_dependency_tickets_ms")
    );
    assert!(
        scan_report
            .phase_timings_ms
            .contains_key("workflow.compute_unresolved_ms")
    );
    assert!(
        scan_report
            .phase_timings_ms
            .contains_key("workflow.write_facts_ms")
    );
    assert!(
        scan_report
            .phase_timings_ms
            .keys()
            .any(|key| key.starts_with("scan_root_"))
    );
    assert!(
        incremental_scan_1
            .phase_timings_ms
            .contains_key("scan_total_ms")
    );
    assert!(
        incremental_scan_10
            .phase_timings_ms
            .contains_key("scan_total_ms")
    );
    assert!(
        incremental_scan_100
            .phase_timings_ms
            .contains_key("scan_total_ms")
    );

    let scan_total_samples_ms = vec![
        *scan_report
            .phase_timings_ms
            .get("scan_total_ms")
            .expect("scan_total_ms present for full scan"),
        *incremental_scan_1
            .phase_timings_ms
            .get("scan_total_ms")
            .expect("scan_total_ms present for incremental scan(1)"),
        *incremental_scan_10
            .phase_timings_ms
            .get("scan_total_ms")
            .expect("scan_total_ms present for incremental scan(10)"),
        *incremental_scan_100
            .phase_timings_ms
            .get("scan_total_ms")
            .expect("scan_total_ms present for incremental scan(100)"),
    ];
    let (scan_total_p50_ms, scan_total_p95_ms, scan_total_p99_ms) =
        percentile_summary(&scan_total_samples_ms);
    assert!(scan_total_p50_ms <= scan_total_p95_ms);
    assert!(scan_total_p95_ms <= scan_total_p99_ms);

    eprintln!(
        "health_perf run_id={} fixture_profile={} workspace_scope={} change_count={} reindex_mode={} scan_total_p50_ms={} scan_total_p95_ms={} scan_total_p99_ms={} open_phases={:?} bootstrap_scans={:?} scan_true_phases={:?} scan_false_1_phases={:?} scan_false_10_phases={:?} scan_false_100_phases={:?} list_ms={} workflow_ms={} collect_ms={} tickets={} edges={} findings={}",
        run_id,
        fixture_profile,
        workspace_scope,
        change_count,
        reindex_mode,
        scan_total_p50_ms,
        scan_total_p95_ms,
        scan_total_p99_ms,
        open_report.phase_timings_ms,
        open_report.scan_reports.keys().cloned().collect::<Vec<_>>(),
        scan_report.phase_timings_ms,
        incremental_scan_1.phase_timings_ms,
        incremental_scan_10.phase_timings_ms,
        incremental_scan_100.phase_timings_ms,
        list_elapsed.as_millis(),
        workflow_elapsed.as_millis(),
        health_elapsed.as_millis(),
        tickets.len(),
        all_edges.len(),
        report.findings.len(),
    );
    tracing::info!(
        target: PERF_TRACE_TARGET,
        run = "health_all_e2e_reports_timings_on_large_fixture",
        run_id = %run_id,
        fixture_profile,
        workspace_scope,
        change_count,
        reindex_mode,
        scan_total_p50_ms,
        scan_total_p95_ms,
        scan_total_p99_ms,
        tickets = tickets.len(),
        edges = all_edges.len(),
        findings = report.findings.len(),
        list_ms = list_elapsed.as_millis() as u64,
        workflow_ms = workflow_elapsed.as_millis() as u64,
        collect_ms = health_elapsed.as_millis() as u64,
        "ticket_api_perf_test_complete"
    );
}
