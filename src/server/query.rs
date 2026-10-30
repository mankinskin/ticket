use ticket_api::storage::ticket_fs::TicketFs;

use super::{types::*, *};

impl TicketServer {
    pub(crate) async fn health_tool(&self) -> Result<CallToolResult, McpError> {
        match self
            .with_store("default", |store| store.list(None, None, Some(0)))
            .await
        {
            Ok(_) => Self::json_result(&serde_json::json!({
                "status": "ok",
                "service": "ticket-mcp",
                "mode": "direct",
            })),
            Err(error) => Self::json_result(&serde_json::json!({
                "status": "error",
                "error": error.to_string(),
            })),
        }
    }

    pub(crate) async fn list_workspaces_tool(&self) -> Result<CallToolResult, McpError> {
        let active_path = self.index_root.to_string_lossy().to_string();
        Self::json_result(&serde_json::json!({
            "workspaces": ["default", active_path],
            "active": "default",
            "active_path": self.index_root,
        }))
    }

    pub(crate) async fn list_tickets_tool(
        &self,
        input: ListTicketsInput,
    ) -> Result<CallToolResult, McpError> {
        let workspace = input.workspace.clone();
        let items = if let Some(query) = input.query.as_deref() {
            let limit = input.limit.unwrap_or(100).min(1000);
            self.with_store(&workspace, |store| {
                search_ticket_summaries(store, query, limit)
            })
            .await?
        } else {
            self.with_store(&workspace, |store| listed_ticket_summaries(store, &input))
                .await?
        };

        Self::json_result(&serde_json::json!({
            "workspace": workspace,
            "items": items,
        }))
    }

    pub(crate) async fn get_ticket_tool(
        &self,
        input: TicketRefInput,
    ) -> Result<CallToolResult, McpError> {
        let workspace_selector = input.workspace;
        let workspace = workspace_selector
            .clone()
            .unwrap_or_else(|| "default".to_string());
        let id_str = input.id;
        let view = input.view;
        let parts = input.parts;

        self.with_read_store_ext(workspace_selector.as_deref(), move |store| {
            let id = Self::resolve_uuid_for_read(store, &id_str)?;
            let projection =
                ticket_api::storage::ReadProjection::decode(view.as_deref(), parts.as_deref())
                    .map_err(Self::store_err)?;

            if let Some(projection) = projection {
                let projected = store.project(&id, &projection).map_err(Self::store_err)?;
                return Self::json_result(&serde_json::json!({
                    "workspace": workspace,
                    "ticket": projected,
                }));
            }

            let path = store
                .get_indexed(&id)
                .map_err(Self::store_err)?
                .map(|ticket| ticket.path.display().to_string());
            let manifest = store.get(&id).map_err(Self::store_err)?;
            Self::json_result(&serde_json::json!({
                "workspace": workspace,
                "ticket": TicketDetail {
                    id: manifest.id.to_string(),
                    path,
                    created_at: manifest.created_at,
                    fields: manifest.extra,
                },
            }))
        })
        .await
    }

    pub(crate) async fn get_ticket_description_tool(
        &self,
        input: TicketRefInput,
    ) -> Result<CallToolResult, McpError> {
        let workspace_selector = input.workspace;
        let workspace = workspace_selector
            .clone()
            .unwrap_or_else(|| "default".to_string());
        let id_str = input.id;

        self.with_read_store_ext(workspace_selector.as_deref(), move |store| {
            let id = Self::resolve_uuid_for_read(store, &id_str)?;
            let indexed = store
                .get_indexed(&id)
                .map_err(Self::store_err)?
                .ok_or_else(|| McpError::invalid_params(format!("ticket not found: {id}"), None))?;

            Self::json_result(&serde_json::json!({
                "workspace": workspace,
                "id": id.to_string(),
                "description": TicketFs::read_description(&indexed.path)
                    .map_err(Self::store_err)?,
            }))
        })
        .await
    }

    pub(crate) async fn list_edges_tool(
        &self,
        input: ListEdgesInput,
    ) -> Result<CallToolResult, McpError> {
        let workspace = input.workspace;
        let items = self
            .with_store(&workspace, |store| store.list_all_edges())
            .await?
            .into_iter()
            .filter(|edge| match &input.kind {
                Some(kind) => kind == "all" || edge.kind == *kind,
                None => true,
            })
            .map(|edge| EdgeItem {
                from: edge.from.to_string(),
                to: edge.to.to_string(),
                kind: edge.kind,
            })
            .collect::<Vec<_>>();

        Self::json_result(&serde_json::json!({
            "workspace": workspace,
            "items": items,
        }))
    }
}

fn search_ticket_summaries(
    store: &TicketStore,
    query: &str,
    limit: usize,
) -> Result<Vec<TicketSummary>, ticket_api::error::StorageError> {
    let results = store.search_tickets(query, limit)?;
    Ok(results
        .into_iter()
        .map(|result| TicketSummary {
            id: result.id.to_string(),
            type_id: result.ticket_type.unwrap_or_default(),
            title: result.title,
            state: result.state,
            effort: None,
            updated_at: indexed_updated_at(store, &result.id),
        })
        .collect())
}

fn listed_ticket_summaries(
    store: &TicketStore,
    input: &ListTicketsInput,
) -> Result<Vec<TicketSummary>, ticket_api::error::StorageError> {
    let limit = input.limit.map(|value| value.min(1000));
    let tickets = store.list(input.state.as_deref(), input.type_id.as_deref(), limit)?;
    let model = ticket_api::workflow::WorkflowModel::build(
        store,
        tickets.clone(),
        store.list_all_edges()?,
    )?;
    Ok(tickets
        .into_iter()
        .map(|ticket| TicketSummary {
            id: ticket.id.to_string(),
            type_id: ticket.type_id,
            title: ticket.title,
            state: ticket.state,
            effort: model.effort(&ticket.id),
            updated_at: ticket.updated_at,
        })
        .collect())
}

#[cfg(test)]
#[path = "query/tests.rs"]
mod tests;

fn indexed_updated_at(store: &TicketStore, id: &Uuid) -> chrono::DateTime<chrono::Utc> {
    store
        .get_indexed(id)
        .ok()
        .flatten()
        .map(|ticket| ticket.updated_at)
        .unwrap_or_else(|| chrono::DateTime::<chrono::Utc>::from(std::time::SystemTime::UNIX_EPOCH))
}
