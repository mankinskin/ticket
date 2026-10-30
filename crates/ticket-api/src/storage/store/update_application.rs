#[allow(unused_imports)]
use super::*;

impl TicketStore {
    pub(super) fn apply_manifest_update(
        &self,
        id: &Uuid,
        ticket_path: &Path,
        patch: &BTreeMap<String, Value>,
        new_state: &Option<String>,
        transition_path: &[String],
        type_id: &str,
        description: Option<&str>,
        description_mode: Option<DescriptionUpdateMode>,
    ) -> Result<(TicketManifest, Option<String>), StorageError> {
        // Gate on the ticket's state as of the start of this call, before
        // any transition below applies this same call's freeze/unfreeze.
        // A single call that both transitions into `ready` and sets the
        // description is the freeze taking effect, not a write to a part
        // already frozen by a prior call — AC7 targets the latter only
        // (proven by `f9e70385_legacy_description_write_rejected_when_objective_frozen`,
        // which freezes in one call and writes in a separate later one).
        if description.is_some() {
            self.enforce_description_write_gate(id)?;
        }

        // Apply description changes before transitions, but do not create
        // manifest parts as a side effect of freezing an incomplete plan.
        let mut previous_description = None;
        if let Some(desc) = description {
            // AC1/AC2: an omitted description_mode is a hard error, not a
            // silent default — the silent `Replace` default was the direct
            // cause of destructive description overwrites.
            let mode = description_mode
                .ok_or_else(|| StorageError::Other(REQUIRED_DESCRIPTION_MODE_ERROR.to_string()))?;
            let existing = TicketFs::read_description(ticket_path)?;
            previous_description = existing.clone();
            let final_text = match mode {
                DescriptionUpdateMode::Replace => desc.to_string(),
                DescriptionUpdateMode::Append => match existing {
                    Some(existing) if !existing.is_empty() => {
                        format!("{existing}\n{desc}")
                    }
                    _ => desc.to_string(),
                },
            };
            // AC7: the legacy `description` write is not a privileged
            // bypass of plan freezing — it was already gated above against
            // this call's pre-transition state.
            TicketFs::write_description(ticket_path, &final_text)?;
        }

        let updated_manifest = if transition_path.is_empty() {
            let mut manifest = TicketFs::update(ticket_path, patch, new_state.as_deref())?;
            let planned_rank = self.state_rank_for_type(type_id, Some("planning"));
            if let Some(final_state) = new_state.as_deref() {
                if final_state == "ready" {
                    manifest = TicketFs::apply_plan_freeze(ticket_path, true)?;
                } else if self.state_rank_for_type(type_id, Some(final_state)) < planned_rank {
                    manifest = TicketFs::apply_plan_freeze(ticket_path, false)?;
                }
            }
            manifest
        } else {
            // Plan freezing (spec 24b3d22b, ticket f9e70385, AC1/AC5): every
            // state visited along the transition path is evaluated. Entering
            // `ready` freezes existing planning parts, leaves missing kinds
            // absent, and cuts a plan revision; landing below `ready` clears
            // every frozen flag. States ranked at or above `ready` otherwise
            // leave frozen flags untouched.
            let planned_rank = self.state_rank_for_type(type_id, Some("ready"));
            let mut manifest = None;
            for (index, state) in transition_path.iter().enumerate() {
                let step_patch = if index + 1 == transition_path.len() {
                    patch.clone()
                } else {
                    BTreeMap::new()
                };
                let mut step_manifest =
                    TicketFs::update(ticket_path, &step_patch, Some(state.as_str()))?;
                if state.as_str() == "ready" {
                    step_manifest = TicketFs::apply_plan_freeze(ticket_path, true)?;
                } else if self.state_rank_for_type(type_id, Some(state.as_str())) < planned_rank {
                    step_manifest = TicketFs::apply_plan_freeze(ticket_path, false)?;
                }
                manifest = Some(step_manifest);
            }
            manifest.expect("transition path produces at least one manifest")
        };

        Ok((updated_manifest, previous_description))
    }

    pub(super) fn refresh_index_and_search(
        &self,
        id: &Uuid,
        patch: &BTreeMap<String, Value>,
        new_state: &Option<String>,
        updated_manifest: &TicketManifest,
        indexed: &mut IndexedTicket,
        now: DateTime<Utc>,
    ) -> Result<(), StorageError> {
        indexed.updated_at = now;
        if let Some(s) = new_state {
            indexed.state = Some(s.clone());
        }
        if let Some(title_val) = patch.get("title").and_then(|v| v.as_str()) {
            indexed.title = Some(title_val.to_string());
        }
        self.index.insert_ticket(indexed)?;

        let body = TicketFs::read_description(&indexed.path)?;
        let created_at_str = indexed.created_at.to_rfc3339();
        let effort_str = updated_manifest.extra.get("effort").and_then(|v| match v {
            serde_json::Value::String(s) => Some(s.clone()),
            serde_json::Value::Number(n) => Some(n.to_string()),
            _ => None,
        });
        self.with_search_repair(|| {
            Ok(self.search.upsert(
                id,
                indexed.title.as_deref(),
                body.as_deref(),
                indexed.state.as_deref(),
                Some(indexed.type_id.as_str()),
                Some(&created_at_str),
                effort_str.as_deref(),
            )?)
        })?;

        Ok(())
    }
}
