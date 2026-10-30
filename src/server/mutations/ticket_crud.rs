#[allow(unused_imports)]
use super::*;

impl TicketServer {
    pub(crate) async fn update_ticket_tool(
        &self,
        input: UpdateTicketInput,
    ) -> Result<CallToolResult, McpError> {
        if input.undo {
            return self.undo_ticket_update(input).await;
        }

        let workspace = input.workspace;
        let id_str = input.id;
        let transition_states = input.transition_states;
        let to_state = input.to_state;
        let patch = parse_field_patch(input.fields, input.field_map)?;
        // The wire `description` + `description_mode` pair is decoded into
        // `input.description_update` at deserialization time by
        // `UpdateTicketInput`'s `TryFrom<UpdateTicketInputWire>` (AC5 of
        // ticket 3d952036); no runtime decode happens here.
        let description_update = input.description_update;
        let (description, description_mode) = description_update.as_parts();
        let description = description.map(str::to_string);
        let author = input.author;
        let single_hop = input.single_hop;
        let changed_fields = patch.clone();
        let state_transition_requested = to_state.clone();
        let description_updated = description.is_some();
        let (manifest, path, previous_state) = self
            .with_store_ext(&workspace.clone(), move |store| {
                let id = Self::resolve_uuid_with(store, &id_str)?;
                let previous_state = store
                    .get_indexed(&id)
                    .map_err(Self::store_err)?
                    .and_then(|ticket| ticket.state);
                let manifest = store
                    .update_with_options(
                        &id,
                        patch,
                        Some(transition_states.as_slice()),
                        to_state.as_deref(),
                        description.as_deref(),
                        description_mode,
                        author.as_deref(),
                        single_hop,
                    )
                    .map_err(Self::store_err)?;
                let path = indexed_ticket_path(store, &id)?;
                Ok((manifest, path, previous_state))
            })
            .await?;

        let mut response = serde_json::Map::from_iter([
            ("status".to_string(), Value::String("ok".to_string())),
            ("id".to_string(), Value::String(manifest.id.to_string())),
        ]);
        if let Some(path) = path {
            response.insert("path".to_string(), Value::String(path));
        }
        if !changed_fields.is_empty() {
            response.insert(
                "changed_fields".to_string(),
                Value::Object(changed_fields.into_iter().collect()),
            );
        }
        if let Some(to_state) = state_transition_requested {
            response.insert(
                "state_transition".to_string(),
                serde_json::json!({
                    "from": previous_state,
                    "to": to_state,
                }),
            );
        }
        if description_updated {
            response.insert("description_updated".to_string(), Value::Bool(true));
        }

        Self::json_result(&Value::Object(response))
    }

    pub(crate) async fn close_ticket_tool(
        &self,
        input: CloseTicketInput,
    ) -> Result<CallToolResult, McpError> {
        let workspace = input.workspace;
        let id_str = input.id;
        let author = input.author;
        let target_state = input.to_state.clone();
        let (manifest, path) = self
            .with_store_ext(&workspace.clone(), move |store| {
                let id = Self::resolve_uuid_with(store, &id_str)?;
                store
                    .close(&id, &input.to_state, author.as_deref())
                    .map_err(Self::store_err)
            })
            .await?;

        Self::json_result(&serde_json::json!({
            "workspace": workspace,
            "status": "ok",
            "id": manifest.id.to_string(),
            "target_state": target_state,
            "traversed_states": path,
        }))
    }

    pub(crate) async fn cancel_ticket_tool(
        &self,
        input: CancelTicketInput,
    ) -> Result<CallToolResult, McpError> {
        let workspace = input.workspace;
        let id_str = input.id;
        let author = input.author;
        let (manifest, path) = self
            .with_store_ext(&workspace.clone(), move |store| {
                let id = Self::resolve_uuid_with(store, &id_str)?;
                store
                    .close(&id, "cancelled", author.as_deref())
                    .map_err(Self::store_err)
            })
            .await?;

        Self::json_result(&serde_json::json!({
            "workspace": workspace,
            "status": "ok",
            "id": manifest.id.to_string(),
            "traversed_states": path,
        }))
    }

    pub(crate) async fn create_ticket_tool(
        &self,
        input: CreateTicketInput,
    ) -> Result<CallToolResult, McpError> {
        let extra = parse_field_patch(Some(input.fields.clone()), None)?;
        let workspace =
            ticket_api::workspace::normalize_explicit_workspace_selector(Some(&input.workspace))
                .map_err(|err| McpError::invalid_params(err.to_string(), None))?
                .to_string_lossy()
                .into_owned();
        let type_id = input.type_id;
        let title = input.title;
        let state = input.state;
        let description = input.description;
        let (ticket_id, manifest, path) = self
            .with_store_ext(&workspace.clone(), move |store| {
                let id = store
                    .create(
                        None,
                        &type_id,
                        title.as_deref(),
                        state.as_deref(),
                        extra,
                        None,
                        description.as_deref(),
                    )
                    .map_err(Self::store_err)?;
                let manifest = store.get(&id).map_err(Self::store_err)?;
                let path = indexed_ticket_path(store, &id)?;
                Ok((id, manifest, path))
            })
            .await?;

        Self::json_result(&serde_json::json!({
            "workspace": workspace,
            "status": "ok",
            "id": ticket_id.to_string(),
            "ticket": detail_from_manifest(manifest, path),
        }))
    }

    pub(crate) async fn delete_ticket_tool(
        &self,
        input: DeleteTicketInput,
    ) -> Result<CallToolResult, McpError> {
        let workspace = input.workspace;
        let id_str = input.id;
        let id = self
            .with_store_ext(&workspace.clone(), move |store| {
                let id = Self::resolve_uuid_with(store, &id_str)?;
                store.delete(&id).map_err(Self::store_err)?;
                Ok(id)
            })
            .await?;

        Self::json_result(&serde_json::json!({
            "workspace": workspace,
            "status": "ok",
            "id": id.to_string(),
        }))
    }
}
