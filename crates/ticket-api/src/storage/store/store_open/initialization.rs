#[allow(unused_imports)]
use super::*;

impl TicketStore {
    pub(in super::super) fn open_internal_profiled(
        index_root: std::path::PathBuf,
        schema_registry: SchemaRegistry,
    ) -> Result<(Self, StoreOpenReport), StorageError> {
        let _span_guard = tracing::debug_span!(
            target: STORE_TRACE_TARGET,
            "ticket_store_open_internal"
        )
        .entered();
        let overall_started = Instant::now();
        let normalize_started = Instant::now();
        let index_root = Self::normalize_existing_path(&index_root);
        let mut report = StoreOpenReport::default();
        report.phase_timings_ms.insert(
            "normalize_existing_path_ms".to_string(),
            elapsed_ms(normalize_started),
        );
        let db_path = index_root.join("tickets.db");
        let search_dir = index_root.join("search_index");

        let sqlite_started = Instant::now();
        let index = RedbIndexStore::open(&db_path)?;
        report.phase_timings_ms.insert(
            "open_sqlite_index_ms".to_string(),
            elapsed_ms(sqlite_started),
        );
        let search_started = Instant::now();
        let search = match TantivySearchIndex::open_or_create(&search_dir) {
            Ok(search) => search,
            Err(_) => {
                let _ = std::fs::remove_dir_all(&search_dir);
                TantivySearchIndex::open_or_create(&search_dir)?
            }
        };
        report.phase_timings_ms.insert(
            "open_search_index_ms".to_string(),
            elapsed_ms(search_started),
        );

        let store = Self {
            index,
            search,
            schema_registry,
            index_root: index_root.clone(),
            hook: OnceLock::new(),
        };
        let add_root_started = Instant::now();
        store.add_scan_root(ScanRoot {
            path: index_root.join("tickets"),
            label: "tickets".to_string(),
        })?;
        report.phase_timings_ms.insert(
            "add_default_scan_root_ms".to_string(),
            elapsed_ms(add_root_started),
        );
        let prune_worktrees_started = Instant::now();
        let pruned_worktree_roots = store.prune_worktree_scan_roots()?;
        report.phase_timings_ms.insert(
            "prune_worktree_scan_roots_ms".to_string(),
            elapsed_ms(prune_worktrees_started),
        );
        if !pruned_worktree_roots.is_empty() {
            let reconcile_started = Instant::now();
            let scan_report = store.scan(true)?;
            report.phase_timings_ms.insert(
                "reconcile_pruned_worktree_roots_ms".to_string(),
                elapsed_ms(reconcile_started),
            );
            report.scan_reports.insert(
                "pruned_worktree_root_reconciliation".to_string(),
                scan_report,
            );
        }
        let bootstrap_started = Instant::now();
        let bootstrap = store.bootstrap_empty_index_from_manifests_profiled()?;
        report.phase_timings_ms.insert(
            "bootstrap_empty_index_ms".to_string(),
            elapsed_ms(bootstrap_started),
        );
        merge_prefixed_timings(
            &mut report.phase_timings_ms,
            "bootstrap",
            bootstrap.phase_timings_ms,
        );
        report.scan_reports.extend(bootstrap.scan_reports);
        report.phase_timings_ms.insert(
            "open_internal_total_ms".to_string(),
            elapsed_ms(overall_started),
        );
        emit_store_open_report("ticket_store_open_internal_complete", &report);
        Ok((store, report))
    }

    pub(super) fn bootstrap_empty_index_from_manifests_profiled(
        &self,
    ) -> Result<StoreOpenReport, StorageError> {
        let _span_guard = tracing::debug_span!(
            target: STORE_TRACE_TARGET,
            "ticket_store_bootstrap_empty_index"
        )
        .entered();
        let mut report = StoreOpenReport::default();
        let count_started = Instant::now();
        if self.count_tickets()? > 0 {
            report
                .phase_timings_ms
                .insert("count_tickets_ms".to_string(), elapsed_ms(count_started));
            let search_probe_started = Instant::now();
            if self
                .search
                .search(&crate::model::query::Expr::And(Vec::new()), 1)
                .map(|results| results.is_empty())
                .unwrap_or(true)
            {
                report.phase_timings_ms.insert(
                    "search_probe_ms".to_string(),
                    elapsed_ms(search_probe_started),
                );
                let bootstrap_scan_started = Instant::now();
                let scan_report = self.scan(true)?;
                report.phase_timings_ms.insert(
                    "scan_existing_index_ms".to_string(),
                    elapsed_ms(bootstrap_scan_started),
                );
                report
                    .scan_reports
                    .insert("bootstrap_existing_index_scan".to_string(), scan_report);
                return Ok(report);
            }
            report.phase_timings_ms.insert(
                "search_probe_ms".to_string(),
                elapsed_ms(search_probe_started),
            );
            return Ok(report);
        }
        report
            .phase_timings_ms
            .insert("count_tickets_ms".to_string(), elapsed_ms(count_started));

        let roots_started = Instant::now();
        for root in self.list_scan_roots()? {
            report
                .phase_timings_ms
                .insert("list_scan_roots_ms".to_string(), elapsed_ms(roots_started));
            let manifest_probe_started = Instant::now();
            if scan_root_has_ticket_manifests(&root.path)? {
                report.phase_timings_ms.insert(
                    "manifest_probe_ms".to_string(),
                    elapsed_ms(manifest_probe_started),
                );
                let bootstrap_scan_started = Instant::now();
                let scan_report = self.scan(true)?;
                report.phase_timings_ms.insert(
                    "scan_manifest_bootstrap_ms".to_string(),
                    elapsed_ms(bootstrap_scan_started),
                );
                report
                    .scan_reports
                    .insert("bootstrap_manifest_scan".to_string(), scan_report);
                break;
            }
            report.phase_timings_ms.insert(
                "manifest_probe_ms".to_string(),
                elapsed_ms(manifest_probe_started),
            );
        }

        if !report.phase_timings_ms.contains_key("list_scan_roots_ms") {
            report
                .phase_timings_ms
                .insert("list_scan_roots_ms".to_string(), elapsed_ms(roots_started));
        }

        Ok(report)
    }
}
