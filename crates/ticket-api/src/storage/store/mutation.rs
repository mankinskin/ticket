#[allow(unused_imports)]
use super::*;

impl TicketStore {
    /// Update a ticket: apply field patches, optional state transition, and optional description.
    pub fn update(
        &self,
        id: &Uuid,
        patch: BTreeMap<String, Value>,
        transition_states: Option<&[String]>,
        to_state: Option<&str>,
        description: Option<&str>,
        author: Option<&str>,
    ) -> Result<TicketManifest, StorageError> {
        self.update_with_options(
            id,
            patch,
            transition_states,
            to_state,
            description,
            None,
            author,
            false,
        )
    }

    /// Same as [`update`](Self::update) but with an explicit `single_hop`
    /// opt-out. When `single_hop` is `false` (the default used by [`update`]),
    /// a reachable multi-hop `to_state` is auto-walked through its required
    /// intermediate states. When `single_hop` is `true`, a transition that
    /// would skip a required waypoint is rejected with a recovery-oriented
    /// [`SchemaValidationError::InvalidTransition`] instead of being walked.
    #[allow(clippy::too_many_arguments)]
    pub fn update_with_options(
        &self,
        id: &Uuid,
        patch: BTreeMap<String, Value>,
        transition_states: Option<&[String]>,
        to_state: Option<&str>,
        description: Option<&str>,
        description_mode: Option<DescriptionUpdateMode>,
        author: Option<&str>,
        single_hop: bool,
    ) -> Result<TicketManifest, StorageError> {
        if let Some(description) = description {
            ensure_nonblank_content(description, format!("ticket {id} description"))?;
        }
        let mut patch = patch;

        // `state` must never be applied as a plain field patch: doing so
        // would either bypass transition validation (if silently applied)
        // or be silently dropped (as it previously was, when the
        // to_state-derived `new_state` overwrote the patched value with the
        // ticket's unchanged current state). Reject it explicitly so callers
        // route state changes through `to_state`/`transition_states`, which
        // validate against the schema's allowed transitions.
        if patch.contains_key("state") {
            return Err(StorageError::Other(format!(
                "field 'state' cannot be set via a field/field_map patch (rejected value: {:?}); use to_state or transition_states to change ticket state",
                patch.get("state")
            )));
        }

        let mut indexed = self.get_indexed(id)?.ok_or(StorageError::NotFound(*id))?;
        let current_manifest = TicketFs::read(&indexed.path)?;
        let edge_patch_plans = edge_patch_plans(&patch, &current_manifest.extra)?;
        strip_file_backed_edge_fields(&mut patch);

        // Determine the target state and transition path.
        // Priority: to_state > last element of transition_states > current state (no change)
        let (new_state, transition_path) =
            self.resolve_update_target(&indexed, transition_states, to_state, single_hop)?;
        let previous_state = indexed.state.clone();
        let (updated_manifest, previous_description) = self.apply_manifest_update(
            id,
            &indexed.path,
            &patch,
            &new_state,
            &transition_path,
            &indexed.type_id,
            description,
            description_mode,
        )?;

        // Route edge-field updates through canonical graph APIs.
        apply_edge_patch_plans(self, *id, edge_patch_plans)?;

        // Refresh indexed metadata.
        let now = Utc::now();
        self.refresh_index_and_search(
            id,
            &patch,
            &new_state,
            &updated_manifest,
            &mut indexed,
            now,
        )?;

        // Append history snapshot after successful write. Always capture the
        // pre-update description alongside the manifest fields whenever a
        // description change was applied, regardless of mode, so it is
        // recoverable via undo.
        //
        // A failure here is logged loudly rather than propagated: the
        // manifest/description write above already succeeded and is the
        // system of record, so failing the whole update would report
        // failure for a mutation that actually committed (and could prompt
        // a caller retry that double-applies the patch). The history file
        // is a best-effort undo trail; losing an entry must be visible, not
        // silent, but must not mask a successful write as an error.
        let mut history_fields = updated_manifest.extra.clone();
        if description.is_some() {
            history_fields.insert(
                DESCRIPTION_HISTORY_KEY.to_string(),
                previous_description
                    .map(Value::String)
                    .unwrap_or(Value::Null),
            );
        }
        if let Err(error) =
            TicketFs::append_history(&indexed.path, history_fields, author.map(str::to_string))
        {
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
                *id,
                indexed.state.clone(),
                indexed.title.clone(),
                indexed.updated_at,
            );
        }

        // Reconcile board: mark completed on terminal states.
        self.board_reconcile(id, false);

        let state_progressed = previous_state.as_deref() != new_state.as_deref()
            && self.state_rank_for_type(&indexed.type_id, new_state.as_deref())
                > self.state_rank_for_type(&indexed.type_id, previous_state.as_deref());
        let workflow_progress = previous_state.as_deref() != new_state.as_deref()
            || new_state.as_deref() == Some("planning");
        if workflow_progress {
            self.refresh_workflow_facts_for_roots(
                &[*id],
                state_progressed || new_state.as_deref() == Some("planning"),
                now,
            )?;
        }

        TicketFs::read(&indexed.path)
    }

    pub(super) fn resolve_update_target(
        &self,
        indexed: &IndexedTicket,
        transition_states: Option<&[String]>,
        to_state: Option<&str>,
        single_hop: bool,
    ) -> Result<(Option<String>, Vec<String>), StorageError> {
        if let Some(to) = to_state {
            let path = self.resolve_transition_path(
                indexed,
                transition_states.unwrap_or(&[]),
                to,
                single_hop,
            )?;
            let final_state = path.last().cloned().unwrap_or_else(|| to.to_string());
            return Ok((Some(final_state), path));
        }

        if let Some(transition_states_slice) = transition_states {
            if let Some(final_target) = transition_states_slice.last() {
                let intermediate_steps =
                    &transition_states_slice[..transition_states_slice.len() - 1];
                let path = self.resolve_transition_path(
                    indexed,
                    intermediate_steps,
                    final_target,
                    single_hop,
                )?;
                return Ok((Some(final_target.clone()), path));
            }
            return Ok((indexed.state.clone(), Vec::new()));
        }

        Ok((indexed.state.clone(), Vec::new()))
    }
}
