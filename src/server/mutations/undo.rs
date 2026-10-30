#[allow(unused_imports)]
use super::*;

impl TicketServer {
    pub(super) async fn undo_ticket_update(
        &self,
        input: UpdateTicketInput,
    ) -> Result<CallToolResult, McpError> {
        let has_fields = input
            .fields
            .as_ref()
            .is_some_and(|fields| !fields.is_empty());
        let has_field_map = input
            .field_map
            .as_ref()
            .is_some_and(|fields| !fields.is_empty());
        if input.to_state.is_some()
            || !input.transition_states.is_empty()
            || has_fields
            || has_field_map
        {
            return Err(McpError::invalid_params(
                "undo cannot be combined with to_state, transition_states, fields, or field_map",
                None,
            ));
        }

        let workspace = input.workspace;
        let id_str = input.id;
        let (previous_rev, new_rev, updated, path) = self
            .with_store_ext(&workspace.clone(), move |store| {
                let id = Self::resolve_uuid_with(store, &id_str)?;
                let revisions = store.get_history(&id).map_err(Self::store_err)?;
                if revisions.len() < 2 {
                    return Err(Self::store_err(ticket_api::error::StorageError::Database(
                        "cannot undo: not enough history revisions".into(),
                    )));
                }
                let previous = &revisions[revisions.len() - 2];
                let mut revert_fields = previous.fields.clone();
                if let Some(desc_val) = revisions[revisions.len() - 1]
                    .fields
                    .get(ticket_api::storage::DESCRIPTION_HISTORY_KEY)
                {
                    revert_fields.insert(
                        ticket_api::storage::DESCRIPTION_HISTORY_KEY.to_string(),
                        desc_val.clone(),
                    );
                }
                let new_rev = store
                    .apply_revert(&id, revert_fields, None)
                    .map_err(Self::store_err)?;
                let updated = store.get(&id).map_err(Self::store_err)?;
                let path = indexed_ticket_path(store, &id)?;
                Ok((previous.rev, new_rev, updated, path))
            })
            .await?;

        Self::json_result(&serde_json::json!({
            "workspace": workspace,
            "status": "ok",
            "undo": true,
            "reverted_to": previous_rev,
            "new_rev": new_rev,
            "ticket": detail_from_manifest(updated, path),
        }))
    }
}
