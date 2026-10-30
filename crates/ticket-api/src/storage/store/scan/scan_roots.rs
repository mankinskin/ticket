#[allow(unused_imports)]
use super::*;

impl TicketStore {
    pub(in super::super) fn is_external_worktree_path(&self, path: &Path) -> bool {
        match (
            Self::worktree_scope(path),
            Self::worktree_scope(&self.index_root),
        ) {
            (Some(path_scope), Some(store_scope)) => path_scope != store_scope,
            (Some(_), None) => true,
            (None, _) => false,
        }
    }

    pub(super) fn worktree_scope(path: &Path) -> Option<std::path::PathBuf> {
        let normalized = Self::normalize_path(path.to_path_buf());
        let mut scope = std::path::PathBuf::new();
        let mut found_worktrees = false;

        for component in normalized.components() {
            scope.push(component.as_os_str());
            if found_worktrees {
                return Some(scope);
            }
            found_worktrees = component.as_os_str() == ".worktrees";
        }

        None
    }

    pub fn add_scan_root(&self, root: ScanRoot) -> Result<(), StorageError> {
        let path = self.resolve_scan_root_path(&root.path);
        if self.is_external_worktree_path(&path) {
            return Err(StorageError::Other(format!(
                "refusing scan root under .worktrees outside store root: {}",
                path.display()
            )));
        }
        self.index.add_scan_root(&ScanRoot {
            path,
            label: root.label,
        })?;
        Ok(())
    }

    /// Persist a scan root together with explicit auditability metadata.
    pub fn add_scan_root_with_metadata(
        &self,
        root: ScanRoot,
        metadata: ScanRootMetadata,
    ) -> Result<(), StorageError> {
        let path = self.resolve_scan_root_path(&root.path);
        if self.is_external_worktree_path(&path) {
            return Err(StorageError::Other(format!(
                "refusing scan root under .worktrees outside store root: {}",
                path.display()
            )));
        }
        self.index.add_scan_root_with_metadata(
            &ScanRoot {
                path,
                label: root.label,
            },
            &metadata,
        )?;
        Ok(())
    }

    /// Delete persisted sibling-worktree scan roots that can outlive the
    /// worktree and leave stale indexed ticket paths behind.
    pub fn prune_worktree_scan_roots(&self) -> Result<Vec<ScanRoot>, StorageError> {
        let persisted = self.index.list_scan_roots()?;
        let paths: Vec<_> = persisted
            .iter()
            .filter_map(|root| {
                let path = self.resolve_scan_root_path(&root.path);
                self.is_external_worktree_path(&path)
                    .then_some(root.path.clone())
            })
            .collect();
        self.index.remove_scan_roots(&paths)?;

        Ok(persisted
            .into_iter()
            .filter(|root| paths.iter().any(|path| path == &root.path))
            .map(|root| ScanRoot {
                path: self.resolve_scan_root_path(&root.path),
                label: root.label,
            })
            .collect())
    }

    pub fn list_scan_roots(&self) -> Result<Vec<ScanRoot>, StorageError> {
        let mut seen = HashSet::new();
        let mut roots = Vec::new();

        for root in self.index.list_scan_roots()? {
            let path = self.resolve_scan_root_path(&root.path);
            if seen.insert(path.clone()) {
                roots.push(ScanRoot {
                    path,
                    label: root.label,
                });
            }
        }

        Ok(roots)
    }

    /// List persisted scan roots together with their auditability metadata.
    ///
    /// Paths are resolved and de-duplicated exactly like [`list_scan_roots`].
    pub fn list_scan_roots_with_metadata(&self) -> Result<Vec<PersistedScanRoot>, StorageError> {
        let mut seen = HashSet::new();
        let mut roots = Vec::new();

        for persisted in self.index.list_scan_roots_with_metadata()? {
            let path = self.resolve_scan_root_path(&persisted.root.path);
            if seen.insert(path.clone()) {
                roots.push(PersistedScanRoot {
                    root: ScanRoot {
                        path,
                        label: persisted.root.label,
                    },
                    metadata: persisted.metadata,
                });
            }
        }

        Ok(roots)
    }

    /// Re-apply the workspace policy to the registered scan roots.
    ///
    /// Discovery is re-run under the resolved policy: every discovered allowed
    /// root is (re-)registered with `policy_decision = included`, and any
    /// previously-registered root that the policy no longer allows is flipped to
    /// `policy_decision = ignored` (so scan-time skipping and the query-time
    /// guard exclude it). A forced rescan then re-indexes, and the returned
    /// [`ScanReport`] surfaces the skipped roots.
    pub fn reapply_workspace_policy(
        &self,
        workspace_root: &Path,
    ) -> Result<ScanReport, StorageError> {
        let policy = crate::workspace_policy::load_workspace_policy(workspace_root);

        let allowed = crate::workspace::discover_workspace_scan_roots_with_policy(
            workspace_root,
            crate::workspace::TICKET_INDEX_DIR,
            "tickets",
            &policy,
        );
        let allowed_paths: HashSet<std::path::PathBuf> = allowed
            .iter()
            .map(|root| self.resolve_scan_root_path(&root.path))
            .collect();

        // Flip previously-registered roots the policy no longer allows.
        for persisted in self.list_scan_roots_with_metadata()? {
            if !allowed_paths.contains(&persisted.root.path) {
                self.add_scan_root_with_metadata(
                    persisted.root,
                    ScanRootMetadata {
                        source: ScanRootSource::Policy,
                        policy_decision: PolicyDecision::Ignored,
                        workspace_root: Some(workspace_root.to_path_buf()),
                    },
                )?;
            }
        }

        // (Re-)register every allowed root as policy-included.
        for root in allowed {
            self.add_scan_root_with_metadata(
                root,
                ScanRootMetadata {
                    source: ScanRootSource::Policy,
                    policy_decision: PolicyDecision::Included,
                    workspace_root: Some(workspace_root.to_path_buf()),
                },
            )?;
        }

        self.scan(true)
    }
}
