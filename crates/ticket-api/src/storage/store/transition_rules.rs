#[allow(unused_imports)]
use super::*;

impl TicketStore {
    pub(super) fn resolve_transition_path(
        &self,
        indexed: &IndexedTicket,
        transition_states: &[String],
        target_state: &str,
        single_hop: bool,
    ) -> Result<Vec<String>, StorageError> {
        let schema = self.schema_registry.get(&indexed.type_id).ok_or_else(|| {
            StorageError::Other(format!("no schema for type '{}'", indexed.type_id))
        })?;
        let current_state = indexed.state.as_deref().unwrap_or("planning");
        if current_state == target_state && transition_states.is_empty() {
            return Ok(vec![]);
        }

        // Ticket-domain compatibility: an off-schema persisted state may only
        // recover directly to the schema entry state before normal transitions
        // resume.
        if !schema.states.iter().any(|state| state == current_state) {
            let entry_state = schema.entry_state().unwrap_or("planning");
            if transition_states.is_empty() && target_state == entry_state {
                return Ok(Vec::new());
            }
            return Err(StorageError::Validation(
                crate::error::SchemaValidationError::InvalidTransition {
                    from: current_state.to_string(),
                    to: target_state.to_string(),
                    allowed_next: vec![entry_state.to_string()],
                    intermediate: if target_state == entry_state {
                        vec![entry_state.to_string()]
                    } else {
                        Vec::new()
                    },
                },
            ));
        }

        self.enforce_dependency_progress(indexed, target_state)?;

        let mut path = Vec::new();
        let mut from = current_state.to_string();
        let mut checkpoints: Vec<String> = transition_states.to_vec();
        checkpoints.push(target_state.to_string());

        for checkpoint in checkpoints {
            if checkpoint == from {
                continue;
            }

            match schema.find_path(&from, &checkpoint) {
                // Reachable path. By default auto-walk it, visiting every
                // required intermediate state. Under the `single_hop` opt-out,
                // a multi-hop path that would skip a required waypoint is
                // rejected with recovery guidance instead of being walked.
                Some(mut segment) => {
                    if segment.last().map(String::as_str) != Some(checkpoint.as_str()) {
                        segment.push(checkpoint.clone());
                    }
                    if single_hop && segment.len() > 1 {
                        return Err(StorageError::Validation(
                            crate::error::SchemaValidationError::InvalidTransition {
                                from: from.clone(),
                                to: checkpoint.clone(),
                                allowed_next: schema.allowed_next_states(&from),
                                intermediate: segment,
                            },
                        ));
                    }
                    path.extend(segment);
                    from = checkpoint;
                }
                // No path exists at all: the target is unreachable from here.
                None => {
                    return Err(StorageError::Validation(
                        schema.invalid_transition_error(&from, &checkpoint).into(),
                    ));
                }
            }
        }

        if !schema.required_states.is_empty()
            && schema.terminal_states.contains(&target_state.to_string())
        {
            let history = TicketFs::read_history(&indexed.path).unwrap_or_default();
            let mut visited: Vec<String> = history
                .iter()
                .filter_map(|r| {
                    r.fields
                        .get("state")
                        .and_then(|v| v.as_str())
                        .map(String::from)
                })
                .collect();
            visited.push(current_state.to_string());
            visited.extend(path.iter().cloned());
            schema.validate_workflow(target_state, &visited)?;
        }

        Ok(path)
    }

    /// Guard: a ticket may not advance further along the state schema than any
    /// of its unresolved `depends_on` targets. Terminal (`done`/`cancelled`)
    /// dependencies are treated as satisfied, cross-store dependencies that are
    /// not locally resolvable are skipped, cancelling a ticket is always
    /// permitted regardless of dependency progress, and parking (`on-hold`) or
    /// demoting a ticket never violates the invariant since neither is forward
    /// progress relative to unresolved dependencies.
    pub(super) fn enforce_dependency_progress(
        &self,
        indexed: &IndexedTicket,
        target_state: &str,
    ) -> Result<(), StorageError> {
        if target_state == "cancelled" || target_state == "on-hold" {
            return Ok(());
        }
        let target_rank = self.state_rank_for_type(&indexed.type_id, Some(target_state));
        // Demotions/no-ops can never violate dependency ordering, only advances can.
        let current_rank = self.state_rank_for_type(&indexed.type_id, indexed.state.as_deref());
        if target_rank <= current_rank {
            return Ok(());
        }
        for edge in self.edges_from(&indexed.id)? {
            if edge.kind != "depends_on" {
                continue;
            }
            let Some(dependency) = self.get_indexed(&edge.to)? else {
                continue;
            };
            let dependency_state = dependency.state.as_deref();
            if matches!(dependency_state, Some("done") | Some("cancelled")) {
                continue;
            }
            let dependency_rank = self.state_rank_for_type(&dependency.type_id, dependency_state);
            if target_rank > dependency_rank {
                return Err(StorageError::DependencyNotProgressed {
                    ticket: indexed.id,
                    target_state: target_state.to_string(),
                    dependency: edge.to,
                    dependency_state: dependency_state.unwrap_or("planning").to_string(),
                });
            }
        }
        Ok(())
    }
}
