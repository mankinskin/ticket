#[allow(unused_imports)]
use super::*;

impl TicketStore {
    /// Open an existing ticket store rooted at `index_root` using built-in schemas.
    ///
    /// Returns [`StorageError::WorkspaceNotFound`] if the workspace has not been
    /// initialized yet. Run `ticket init` to create a new workspace first.
    pub fn open(index_root: &Path) -> Result<Self, StorageError> {
        Self::open_with(index_root, SchemaRegistry::with_builtins())
    }

    /// Open an existing ticket store with a custom schema registry.
    ///
    /// Returns [`StorageError::WorkspaceNotFound`] if the workspace has not been
    /// initialized yet. Use [`TicketStore::init_with`] to create a new workspace.
    pub fn open_with(
        index_root: &Path,
        schema_registry: SchemaRegistry,
    ) -> Result<Self, StorageError> {
        let (store, _) = Self::open_with_profiled(index_root, schema_registry)?;
        Ok(store)
    }

    pub fn open_with_profiled(
        index_root: &Path,
        schema_registry: SchemaRegistry,
    ) -> Result<(Self, StoreOpenReport), StorageError> {
        let selected_store = workspace::resolve_store_root_from_with_diagnostics(
            index_root,
            workspace::TICKET_INDEX_DIR,
        );
        let local_workspace = workspace::resolve_workspace_root_from_store_root(
            &selected_store.store_root,
            workspace::TICKET_INDEX_DIR,
        );
        let resolution =
            Self::resolve_workspace_store(&local_workspace, StoreAccessMode::ReadOnly)?;
        Self::open_resolved_with_profiled(&resolution.store_root, schema_registry)
    }

    pub(super) fn open_resolved_with_profiled(
        index_root: &Path,
        schema_registry: SchemaRegistry,
    ) -> Result<(Self, StoreOpenReport), StorageError> {
        let _span_guard = tracing::debug_span!(
            target: STORE_TRACE_TARGET,
            "ticket_store_open_profiled"
        )
        .entered();
        let overall_started = Instant::now();
        let resolve_started = Instant::now();
        let index_root = index_root.to_path_buf();
        let mut report = StoreOpenReport::default();
        report.phase_timings_ms.insert(
            "resolve_store_root_ms".to_string(),
            elapsed_ms(resolve_started),
        );
        let existence_started = Instant::now();
        if !index_root.join("tickets.db").is_file() {
            return Err(StorageError::WorkspaceNotFound { path: index_root });
        }
        report.phase_timings_ms.insert(
            "workspace_exists_check_ms".to_string(),
            elapsed_ms(existence_started),
        );
        let (store, internal_report) = Self::open_internal_profiled(index_root, schema_registry)?;
        merge_timings(
            &mut report.phase_timings_ms,
            internal_report.phase_timings_ms,
        );
        report.scan_reports.extend(internal_report.scan_reports);
        report
            .phase_timings_ms
            .insert("open_total_ms".to_string(), elapsed_ms(overall_started));
        emit_store_open_report("ticket_store_open_profiled_complete", &report);
        Ok((store, report))
    }

    /// Open a Ticket store from the explicitly selected workspace without
    /// searching ancestor workspaces.
    pub fn open_in_workspace(local_workspace: &Path) -> Result<Self, StorageError> {
        Self::open_in_workspace_with(local_workspace, SchemaRegistry::with_builtins())
    }

    pub fn open_in_workspace_with(
        local_workspace: &Path,
        schema_registry: SchemaRegistry,
    ) -> Result<Self, StorageError> {
        let resolution = Self::resolve_workspace_store(local_workspace, StoreAccessMode::ReadOnly)?;
        let (store, _) =
            Self::open_resolved_with_profiled(&resolution.store_root, schema_registry)?;
        Ok(store)
    }

    /// Initialize a Ticket store in the explicitly selected workspace.
    pub fn init_in_workspace(local_workspace: &Path) -> Result<Self, StorageError> {
        Self::init_in_workspace_with(local_workspace, SchemaRegistry::with_builtins())
    }

    pub fn init_in_workspace_with(
        local_workspace: &Path,
        schema_registry: SchemaRegistry,
    ) -> Result<Self, StorageError> {
        let resolution =
            Self::resolve_workspace_store(local_workspace, StoreAccessMode::CreateOrOpen)?;
        let (store, _) = Self::init_with_profiled(&resolution.store_root, schema_registry)?;
        Ok(store)
    }

    /// Open or initialize only the canonical Ticket store belonging to the
    /// explicitly selected workspace.
    pub fn open_or_init_in_workspace(local_workspace: &Path) -> Result<Self, StorageError> {
        Self::open_or_init_in_workspace_with(local_workspace, SchemaRegistry::with_builtins())
    }

    pub fn open_or_init_in_workspace_with(
        local_workspace: &Path,
        schema_registry: SchemaRegistry,
    ) -> Result<Self, StorageError> {
        let resolution =
            Self::resolve_workspace_store(local_workspace, StoreAccessMode::CreateOrOpen)?;
        let canonical_root = resolution.store_root;
        let schema_for_init = schema_registry.clone();
        let opened = memory_kernel::storage::open_or_init(
            || Self::open_resolved_with_profiled(&canonical_root, schema_registry.clone()),
            || Self::init_with_profiled(&canonical_root, schema_for_init),
        )?;
        let was_initialized = opened.was_initialized();
        let (store, mut report) = opened.into_inner();
        if was_initialized {
            let initial_scan_started = Instant::now();
            let initial_scan = store.scan(true)?;
            report.phase_timings_ms.insert(
                "post_init_scan_ms".to_string(),
                elapsed_ms(initial_scan_started),
            );
            report
                .scan_reports
                .insert("post_init_scan".to_string(), initial_scan);
        }
        emit_store_open_report("ticket_store_open_or_init_complete", &report);
        Ok(store)
    }

    /// Initialize a new ticket store rooted at `index_root` using built-in schemas.
    ///
    /// Creates the workspace directory and all required index files. Idempotent:
    /// if the workspace already exists it is opened without error.
    pub fn init(index_root: &Path) -> Result<Self, StorageError> {
        Self::init_with(index_root, SchemaRegistry::with_builtins())
    }

    /// Open an existing ticket store, or initialize and rebuild it when the
    /// local derived index artifacts do not exist yet.
    pub fn open_or_init(index_root: &Path) -> Result<Self, StorageError> {
        Self::open_or_init_with(index_root, SchemaRegistry::with_builtins())
    }

    /// Initialize a new ticket store with a custom schema registry.
    ///
    /// Creates the workspace directory and all required index files. Idempotent:
    /// if the workspace already exists it is opened without error.
    pub fn init_with(
        index_root: &Path,
        schema_registry: SchemaRegistry,
    ) -> Result<Self, StorageError> {
        let (store, _) = Self::init_with_profiled(index_root, schema_registry)?;
        Ok(store)
    }

    pub(super) fn init_with_profiled(
        index_root: &Path,
        schema_registry: SchemaRegistry,
    ) -> Result<(Self, StoreOpenReport), StorageError> {
        let _span_guard = tracing::debug_span!(
            target: STORE_TRACE_TARGET,
            "ticket_store_init_profiled"
        )
        .entered();
        let overall_started = Instant::now();
        let resolve_started = Instant::now();
        let resolution = Self::resolve_workspace_store(index_root, StoreAccessMode::CreateOrOpen)?;
        let index_root = resolution.store_root;
        let mut report = StoreOpenReport {
            initialized_store: true,
            ..Default::default()
        };
        report.phase_timings_ms.insert(
            "resolve_store_root_ms".to_string(),
            elapsed_ms(resolve_started),
        );
        let ensure_started = Instant::now();
        ensure_sqlite_index_root(&index_root, "tickets.db", &["search_index/"])?;
        report.phase_timings_ms.insert(
            "ensure_index_root_ms".to_string(),
            elapsed_ms(ensure_started),
        );
        let (store, internal_report) = Self::open_internal_profiled(index_root, schema_registry)?;
        merge_timings(
            &mut report.phase_timings_ms,
            internal_report.phase_timings_ms,
        );
        report.scan_reports.extend(internal_report.scan_reports);
        report
            .phase_timings_ms
            .insert("init_total_ms".to_string(), elapsed_ms(overall_started));
        emit_store_open_report("ticket_store_init_profiled_complete", &report);
        Ok((store, report))
    }

    /// Open an existing ticket store with a custom schema registry, or
    /// initialize and rebuild it when the local derived index artifacts do not
    /// exist yet.
    pub fn open_or_init_with(
        index_root: &Path,
        schema_registry: SchemaRegistry,
    ) -> Result<Self, StorageError> {
        let (store, _) = Self::open_or_init_with_profiled(index_root, schema_registry)?;
        Ok(store)
    }

    pub fn open_or_init_profiled(
        index_root: &Path,
    ) -> Result<(Self, StoreOpenReport), StorageError> {
        Self::open_or_init_with_profiled(index_root, SchemaRegistry::with_builtins())
    }

    pub fn open_or_init_with_profiled(
        index_root: &Path,
        schema_registry: SchemaRegistry,
    ) -> Result<(Self, StoreOpenReport), StorageError> {
        let span = tracing::debug_span!(
            target: STORE_TRACE_TARGET,
            "ticket_store_open_or_init",
            initialized_store = Empty,
        );
        let _span_guard = span.enter();
        let overall_started = Instant::now();
        let selected_store = workspace::resolve_store_root_from_with_diagnostics(
            index_root,
            workspace::TICKET_INDEX_DIR,
        );
        let local_workspace = workspace::resolve_workspace_root_from_store_root(
            &selected_store.store_root,
            workspace::TICKET_INDEX_DIR,
        );
        let resolution =
            Self::resolve_workspace_store(&local_workspace, StoreAccessMode::CreateOrOpen)?;
        let canonical_root = resolution.store_root;
        let schema_for_init = schema_registry.clone();
        let opened = memory_kernel::storage::open_or_init(
            || Self::open_resolved_with_profiled(&canonical_root, schema_registry.clone()),
            || Self::init_with_profiled(&canonical_root, schema_for_init),
        )?;
        let was_initialized = opened.was_initialized();
        span.record("initialized_store", was_initialized);
        let (store, mut report) = opened.into_inner();
        if was_initialized {
            let initial_scan_started = Instant::now();
            let initial_scan = store.scan(true)?;
            report.phase_timings_ms.insert(
                "post_init_scan_ms".to_string(),
                elapsed_ms(initial_scan_started),
            );
            report
                .scan_reports
                .insert("post_init_scan".to_string(), initial_scan);
        }
        report.phase_timings_ms.insert(
            "open_or_init_total_ms".to_string(),
            elapsed_ms(overall_started),
        );
        emit_store_open_report("ticket_store_open_or_init_complete", &report);
        Ok((store, report))
    }
}
