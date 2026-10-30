#[allow(unused_imports)]
use super::*;

impl TicketFs {
    pub(super) fn part_path_for_kind(
        ticket_path: &Path,
        manifest: &TicketManifest,
        kind: &str,
    ) -> Result<String, StorageError> {
        let parts = manifest.parts();
        let base = part_kind_slug(kind);
        let ordinal = parts
            .iter()
            .filter(|part| part_kind_slug(&part.kind) == base)
            .count()
            + 1;
        let filename = part_slug_filename(kind, ordinal);
        let rel_path = format!("{TICKET_PARTS_DIR}/{filename}");

        let parts_dir = ticket_path.join(TICKET_PARTS_DIR);
        match fs::symlink_metadata(&parts_dir) {
            Ok(metadata) if !metadata.file_type().is_dir() || metadata.file_type().is_symlink() => {
                return Err(StorageError::PartPathConflict {
                    ticket: manifest.id,
                    path: TICKET_PARTS_DIR.to_string(),
                });
            }
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(StorageError::Io(error)),
        }
        let file_path = ticket_path.join(&rel_path);
        let occupied = match fs::symlink_metadata(&file_path) {
            Ok(_) => true,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => false,
            Err(error) => return Err(StorageError::Io(error)),
        };
        if parts.iter().any(|part| part.path == rel_path) || occupied {
            return Err(StorageError::PartPathConflict {
                ticket: manifest.id,
                path: rel_path,
            });
        }

        Ok(rel_path)
    }

    /// Load a ticket's content parts: the `[[parts]]` manifest entries
    /// joined with their file content, plus a report of orphan files under
    /// `parts/` not referenced by any manifest entry.
    ///
    /// A legacy ticket with no `[[parts]]` table (AC8) reads back a single
    /// synthetic `objective` part backed by `description.md`, with a
    /// deterministic `id` derived from the ticket id so it stays stable
    /// across reads without requiring a migration write.
    pub fn load_parts(
        ticket_path: &Path,
        manifest: &TicketManifest,
    ) -> Result<PartsLoadReport, StorageError> {
        let lock_path = ticket_path.join(TICKET_LOCK_FILE);
        let lock_file = acquire_lock(&lock_path)?;
        let result = (|| -> Result<PartsLoadReport, StorageError> {
            let current_manifest = Self::read_unlocked(ticket_path)?;
            if manifest.id != current_manifest.id {
                return Err(StorageError::Other(format!(
                    "ticket manifest id changed while loading parts at {}",
                    ticket_path.display()
                )));
            }
            let entries = current_manifest.parts();
            if entries.is_empty() {
                let content = Self::read_description_unlocked(ticket_path)?.unwrap_or_default();
                return Ok(PartsLoadReport {
                    parts: vec![LoadedPart {
                        id: implicit_objective_part_id(current_manifest.id),
                        kind: "objective".to_string(),
                        path: "description.md".to_string(),
                        frozen: false,
                        created_at: current_manifest.created_at,
                        supersedes: None,
                        content,
                        implicit: true,
                    }],
                    orphans: Vec::new(),
                });
            }

            let mut parts = Vec::with_capacity(entries.len());
            let mut referenced_paths: BTreeSet<String> = BTreeSet::new();
            for entry in entries {
                let part_path = ticket_path.join(&entry.path);
                let content = fs::read_to_string(&part_path)
                    .map_err(|error| io_error_at(&part_path, error))?;
                referenced_paths.insert(entry.path.clone());
                parts.push(LoadedPart {
                    id: entry.id,
                    kind: entry.kind,
                    path: entry.path,
                    frozen: entry.frozen,
                    created_at: entry.created_at,
                    supersedes: entry.supersedes,
                    content,
                    implicit: false,
                });
            }

            let orphans = find_orphan_part_files(ticket_path, &referenced_paths)?;
            Ok(PartsLoadReport { parts, orphans })
        })();
        release_lock(&lock_file, result)
    }

    /// Write content to a single content part addressed by its stable
    /// opaque `part_id` — never by `kind` or manifest index.
    ///
    /// If `part_id` matches an existing `[[parts]]` entry, only that
    /// entry's file is overwritten; its `kind` is left unchanged (`kind` is
    /// assigned once at creation and never reclassified by a content
    /// write). If `part_id` matches no existing entry, a new part is
    /// created with `kind` and appended to the manifest; `supersedes` is
    /// set on the new entry (used only by `amendment` parts, `None`
    /// otherwise — ignored when overwriting an existing part).
    ///
    /// A legacy ticket with no `[[parts]]` table addresses its implicit
    /// `objective` part (backed by `description.md`) via
    /// [`implicit_objective_part_id`]; any other `part_id` creates a new
    /// explicit part alongside it without disturbing `description.md`.
    ///
    /// Only the addressed part's file is ever read or written: writing a
    /// `review` or `validation` part never touches `objective` or
    /// `description.md` (AC4 of ticket 3d952036).
    ///
    /// Frozen-part rejection is enforced by the caller
    /// (`TicketStore::enforce_part_write_gate`) before this is reached —
    /// this is a crate-private low-level primitive with no external entry
    /// point (spec 24b3d22b, ticket f9e70385, AC7).
    ///
    /// Returns the updated manifest and the part's prior content (`None`
    /// if the write created the part).
    pub(crate) fn write_part(
        ticket_path: &Path,
        part_id: Uuid,
        kind: &str,
        content: &str,
        supersedes: Option<Uuid>,
    ) -> Result<(TicketManifest, Option<String>), StorageError> {
        ensure_nonblank_content(content, format!("ticket part {part_id}"))?;
        let lock_path = ticket_path.join(TICKET_LOCK_FILE);
        let lock_file = acquire_lock(&lock_path)?;

        let result = (|| -> Result<(TicketManifest, Option<String>), StorageError> {
            let mut manifest = Self::read_unlocked(ticket_path)?;
            let mut parts = manifest.parts();

            if let Some(entry) = parts.iter().find(|p| p.id == part_id) {
                let rel_path = entry.path.clone();
                let prior = read_optional_text(&ticket_path.join(&rel_path))?;
                let manifest_bytes = crate::model::manifest_format::format_manifest_toml(&manifest);
                transaction::publish(
                    ticket_path,
                    TransactionOperation::Overwrite,
                    vec![(rel_path, content.as_bytes().to_vec())],
                    Vec::new(),
                    Some(manifest_bytes.as_bytes()),
                )?;
                return Ok((manifest, prior));
            }

            if parts.is_empty() && part_id == implicit_objective_part_id(manifest.id) {
                let prior = Self::read_description_unlocked(ticket_path)?;
                transaction::publish(
                    ticket_path,
                    TransactionOperation::Overwrite,
                    vec![("description.md".to_string(), content.as_bytes().to_vec())],
                    Vec::new(),
                    None,
                )?;
                return Ok((manifest, prior));
            }

            // New part: create its file under parts/ and append a manifest
            // entry. Never touches any other part's file. Reject a
            // near-miss core-kind typo here — the only point where `kind`
            // is actually persisted (an existing part's `kind` is fixed at
            // creation and ignored above) — spec 24b3d22b AC2.
            classify_part_kind(kind)?;
            let rel_path = Self::part_path_for_kind(ticket_path, &manifest, kind)?;
            parts.push(crate::model::ticket::TicketPart {
                id: part_id,
                kind: kind.to_string(),
                path: rel_path.clone(),
                frozen: false,
                created_at: Utc::now(),
                supersedes,
            });
            manifest.set_parts(parts);
            let manifest_bytes = crate::model::manifest_format::format_manifest_toml(&manifest);
            transaction::publish(
                ticket_path,
                TransactionOperation::Create,
                vec![(rel_path, content.as_bytes().to_vec())],
                Vec::new(),
                Some(manifest_bytes.as_bytes()),
            )?;
            Ok((manifest, None))
        })();

        release_lock(&lock_file, result)
    }

    /// Append a new typed `[[refs]]` entry through the gated manifest write
    /// path, validating `kind` against the closed vocabulary and `urn`
    /// against the shape expected for that `kind` (spec 24b3d22b, ticket
    /// 9d69e93d, AC2/AC3). Rejects before touching the manifest on disk.
    pub(crate) fn write_ref(
        ticket_path: &Path,
        kind: &str,
        urn: &str,
        note: Option<String>,
    ) -> Result<TicketManifest, StorageError> {
        crate::model::refs::validate_new_ref(kind, urn)?;

        let lock_path = ticket_path.join(TICKET_LOCK_FILE);
        let lock_file = acquire_lock(&lock_path)?;

        let result = (|| -> Result<TicketManifest, StorageError> {
            let mut manifest = Self::read_unlocked(ticket_path)?;
            let mut refs = manifest.refs();
            refs.push(crate::model::ticket::TicketRefEntry {
                kind: kind.to_string(),
                urn: urn.to_string(),
                note,
            });
            manifest.set_refs(refs);
            write_manifest_transactional(ticket_path, &manifest)?;
            Ok(manifest)
        })();

        release_lock(&lock_file, result)
    }

    /// Remove a typed `[[refs]]` entry matching `kind` and `urn` exactly,
    /// through the same gated manifest write path as [`Self::write_ref`].
    /// A no-op (returns the unchanged manifest) when no entry matches.
    pub(crate) fn remove_ref(
        ticket_path: &Path,
        kind: &str,
        urn: &str,
    ) -> Result<TicketManifest, StorageError> {
        let lock_path = ticket_path.join(TICKET_LOCK_FILE);
        let lock_file = acquire_lock(&lock_path)?;

        let result = (|| -> Result<TicketManifest, StorageError> {
            let mut manifest = Self::read_unlocked(ticket_path)?;
            let mut refs = manifest.refs();
            refs.retain(|entry| !(entry.kind == kind && entry.urn == urn));
            manifest.set_refs(refs);
            write_manifest_transactional(ticket_path, &manifest)?;
            Ok(manifest)
        })();

        release_lock(&lock_file, result)
    }

    /// Remove `[[parts]]` entries matching any of `part_ids` and delete
    /// their backing files, used by the description migration's undo path
    /// (`TicketStore::migration_undo`) to restore the pre-migration layout.
    /// Ids with no matching entry are ignored. `description.md` itself is
    /// never touched — migration never overwrites it, so it already holds
    /// the pre-migration content.
    pub(crate) fn remove_parts(
        ticket_path: &Path,
        part_ids: &[Uuid],
    ) -> Result<TicketManifest, StorageError> {
        let lock_path = ticket_path.join(TICKET_LOCK_FILE);
        let lock_file = acquire_lock(&lock_path)?;

        let result = (|| -> Result<TicketManifest, StorageError> {
            let mut manifest = Self::read_unlocked(ticket_path)?;
            let mut parts = manifest.parts();
            let mut removed_paths = Vec::new();
            parts.retain(|p| {
                if part_ids.contains(&p.id) {
                    removed_paths.push(p.path.clone());
                    false
                } else {
                    true
                }
            });
            manifest.set_parts(parts);
            let manifest_bytes = crate::model::manifest_format::format_manifest_toml(&manifest);
            transaction::publish(
                ticket_path,
                TransactionOperation::RemoveParts,
                Vec::new(),
                removed_paths,
                Some(manifest_bytes.as_bytes()),
            )?;
            Ok(manifest)
        })();

        release_lock(&lock_file, result)
    }

    /// f9e70385, AC1/AC5), invoked exclusively from the state-transition
    /// path (`TicketStore::update_with_options`) whenever a ticket enters
    /// or leaves `ready`. This is the sanctioned freeze/unfreeze
    /// mechanism itself — not a content write — so it does not go through
    /// `TicketStore::enforce_part_write_gate`; there is no other privileged
    /// bypass.
    ///
    /// `freeze = true`: existing entries whose kinds are in
    /// [`crate::model::parts::PLANNING_PART_KINDS`] are set `frozen = true`.
    /// Missing kinds remain absent. `plan_revision` is incremented, which
    /// appends a plan revision to history via the caller's normal snapshot.
    ///
    /// `freeze = false`: every part's `frozen` flag is cleared, regardless
    /// of kind.
    pub(crate) fn apply_plan_freeze(
        ticket_path: &Path,
        freeze: bool,
    ) -> Result<TicketManifest, StorageError> {
        let lock_path = ticket_path.join(TICKET_LOCK_FILE);
        let lock_file = acquire_lock(&lock_path)?;

        let result = (|| -> Result<TicketManifest, StorageError> {
            let mut manifest = Self::read_unlocked(ticket_path)?;
            let mut parts = manifest.parts();

            if freeze {
                for part in &mut parts {
                    if crate::model::parts::is_planning_part_kind(&part.kind) {
                        part.frozen = true;
                    }
                }
                let revision = manifest
                    .extra
                    .get("plan_revision")
                    .and_then(Value::as_u64)
                    .unwrap_or(0)
                    + 1;
                manifest
                    .extra
                    .insert("plan_revision".to_string(), Value::Number(revision.into()));
            } else {
                for part in parts.iter_mut() {
                    part.frozen = false;
                }
            }

            manifest.set_parts(parts);
            write_manifest_transactional(ticket_path, &manifest)?;
            Ok(manifest)
        })();

        release_lock(&lock_file, result)
    }

    pub fn update_edge_field(
        ticket_path: &Path,
        edge_kind: &str,
        target: Uuid,
        present: bool,
    ) -> Result<(TicketManifest, bool), StorageError> {
        let lock_path = ticket_path.join(TICKET_LOCK_FILE);
        let lock_file = acquire_lock(&lock_path)?;

        let result = (|| -> Result<(TicketManifest, bool), StorageError> {
            let mut manifest = Self::read_unlocked(ticket_path)?;
            let mut targets = parse_edge_targets(manifest.extra.get(edge_kind), edge_kind)?;

            let changed = if present {
                targets.insert(target.to_string())
            } else {
                targets.remove(&target.to_string())
            };

            if changed {
                if targets.is_empty() {
                    manifest.extra.remove(edge_kind);
                } else {
                    manifest.extra.insert(
                        edge_kind.to_string(),
                        Value::Array(targets.into_iter().map(Value::String).collect()),
                    );
                }
                write_manifest_transactional(ticket_path, &manifest)?;
            }

            Ok((manifest, changed))
        })();

        release_lock(&lock_file, result)
    }
}
