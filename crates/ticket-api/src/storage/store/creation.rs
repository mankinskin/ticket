#[allow(unused_imports)]
use super::*;

impl TicketStore {
    // ── ticket CRUD ──────────────────────────────────────────────────────────

    /// Create a new ticket.
    ///
    /// `target_root`: a registered scan root, workspace root, store root, or
    /// path inside a local `.ticket/` store. If `None`, the first registered
    /// scan root is used (error if none exist).
    pub fn create(
        &self,
        id: Option<Uuid>,
        type_id: &str,
        title: Option<&str>,
        initial_state: Option<&str>,
        extra: BTreeMap<String, Value>,
        target_root: Option<&Path>,
        body: Option<&str>,
    ) -> Result<TicketId, StorageError> {
        if let Some(body) = body {
            ensure_nonblank_content(body, "ticket description")?;
        }
        let id = id.unwrap_or_else(Uuid::new_v4);
        let now = Utc::now();

        // Resolve target scan root.
        let root = self.resolve_target_root(target_root)?;
        std::fs::create_dir_all(&root)?;

        // Resolve the schema before selecting the conventional entry state so
        // creation and off-schema recovery use the same rule.
        let schema = self.schema_registry.get(type_id).ok_or_else(|| {
            StorageError::Validation(crate::error::SchemaValidationError::UnknownType {
                type_id: type_id.to_string(),
                registered: self
                    .schema_registry
                    .type_ids()
                    .map(str::to_string)
                    .collect(),
            })
        })?;

        let mut manifest = TicketManifest::new(id, now);
        manifest
            .extra
            .insert("type".to_string(), Value::String(type_id.to_string()));
        if let Some(t) = title {
            manifest
                .extra
                .insert("title".to_string(), Value::String(t.to_string()));
        }
        let state = initial_state
            .or_else(|| schema.entry_state())
            .unwrap_or("planning")
            .to_string();
        manifest
            .extra
            .insert("state".to_string(), Value::String(state.clone()));
        for (k, v) in extra {
            manifest.extra.insert(k, v);
        }

        // Validate against the registered type schema; an unregistered type
        // must fail here rather than silently persist an unvalidated ticket
        // that later explodes at transition resolution.
        schema.validate_manifest(&manifest)?;

        let ticket_path = Self::normalize_existing_path(&TicketFs::create(&manifest, &root, body)?);

        let indexed = IndexedTicket {
            id,
            path: ticket_path,
            type_id: type_id.to_string(),
            title: title.map(str::to_string),
            state: Some(state.clone()),
            created_at: now,
            updated_at: now,
        };
        self.index.insert_ticket(&indexed)?;

        // Use the provided body directly (already written to disk); fall back to
        // reading the file for scan-integrated tickets that may have existing content.
        let body_for_index = match body {
            Some(body) => Some(body.to_string()),
            None => TicketFs::read_description(&indexed.path)?,
        };
        let created_at_str = indexed.created_at.to_rfc3339();
        let effort_str = manifest.extra.get("effort").and_then(|v| match v {
            serde_json::Value::String(s) => Some(s.clone()),
            serde_json::Value::Number(n) => Some(n.to_string()),
            _ => None,
        });
        self.with_search_repair(|| {
            Ok(self.search.upsert(
                &id,
                title,
                body_for_index.as_deref(),
                Some(&state),
                Some(type_id),
                Some(&created_at_str),
                effort_str.as_deref(),
            )?)
        })?;

        // Append initial history snapshot (rev 1).
        if let Err(error) = TicketFs::append_history(&indexed.path, manifest.extra.clone(), None) {
            tracing::error!(
                ticket_id = %id,
                path = %indexed.path.display(),
                %error,
                "failed to append history revision; manifest write succeeded but undo history is now incomplete"
            );
        }

        // Emit SSE hook event.
        if let Some(h) = self.hook() {
            h.ticket_upsert(
                id,
                Some(state),
                title.map(str::to_string),
                indexed.updated_at,
            );
        }

        self.refresh_workflow_facts_for_roots(&[id], false, now)?;

        Ok(id)
    }

    pub(super) fn resolve_target_root(
        &self,
        target_root: Option<&Path>,
    ) -> Result<PathBuf, StorageError> {
        let Some(target_root) = target_root else {
            // Canonical: write into the workspace's own .workflow-tools/ticket/tickets/
            // directory (resolved via the index_root), ignoring any registered
            // scan roots. Callers that want to place tickets elsewhere must
            // pass an explicit `target_root`.
            return Ok(self.index_root.join("tickets"));
        };

        let roots = self.list_scan_roots()?;

        let requested = if target_root.is_dir() {
            target_root.to_path_buf()
        } else {
            target_root.parent().unwrap_or(target_root).to_path_buf()
        };
        let requested = self.resolve_scan_root_path(&requested);

        if let Some(root) = roots
            .iter()
            .find(|root| root.path == requested)
            .map(|root| root.path.clone())
        {
            return Ok(root);
        }

        let store_root =
            workspace::resolve_store_root_from(target_root, workspace::TICKET_INDEX_DIR);
        if store_root.exists() {
            return Ok(self.resolve_scan_root_path(&store_root.join("tickets")));
        }

        Err(StorageError::Other(format!(
            "invalid ticket root '{}': expected a registered scan root, a workspace root containing .workflow-tools/ticket, the canonical ticket store itself, or a path inside that store",
            target_root.display()
        )))
    }
}
