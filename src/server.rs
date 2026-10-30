use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use rmcp::{
    ErrorData as McpError, ServerHandler, ServiceExt,
    handler::server::{tool::ToolRouter, wrapper::Parameters},
    model::*,
    tool, tool_handler, tool_router,
    transport::stdio,
};
use serde::Serialize;
use tokio::sync::Mutex;
use uuid::Uuid;

use ticket_api::storage::store::TicketStore;

#[path = "server/board.rs"]
mod board;
#[path = "server/graph.rs"]
mod graph;
#[path = "server/health.rs"]
mod health;
#[path = "server/mutations.rs"]
mod mutations;
#[path = "server/next_tickets.rs"]
mod next_tickets;
#[path = "server/parts.rs"]
mod parts;
#[path = "server/query.rs"]
mod query;
#[path = "server/types.rs"]
mod types;
#[path = "server/workflow.rs"]
mod workflow;

pub use self::types::*;

#[derive(Clone)]
pub struct TicketServer {
    index_root: PathBuf,
    tool_router: ToolRouter<Self>,
    store_lock: Arc<Mutex<()>>,
}

pub fn open_canonical_store(
    index_root: &Path,
) -> Result<TicketStore, ticket_api::error::StorageError> {
    let resolution = TicketStore::resolve_workspace_store(
        index_root,
        memory_kernel::domain_store::StoreAccessMode::CreateOrOpen,
    )?;
    let workspace_root = resolution.local_workspace;
    let store = TicketStore::open_or_init_in_workspace(&workspace_root)?;
    store.reapply_workspace_policy(&workspace_root)?;
    Ok(store)
}

impl TicketServer {
    pub fn new(index_root: PathBuf) -> Self {
        Self {
            index_root,
            tool_router: Self::tool_router(),
            store_lock: Arc::new(Mutex::new(())),
        }
    }

    fn json_result<T: Serialize>(value: &T) -> Result<CallToolResult, McpError> {
        let mut value = serde_json::to_value(value)
            .map_err(|error| McpError::internal_error(format!("serialization: {error}"), None))?;
        ticket_api::output::strip_default_metadata(&mut value);
        let text = serde_json::to_string(&value)
            .map_err(|error| McpError::internal_error(format!("serialization: {error}"), None))?;
        Ok(CallToolResult::success(vec![Content::text(text)]))
    }

    fn store_err(error: ticket_api::error::StorageError) -> McpError {
        McpError::internal_error(format!("store error: {error}"), None)
    }

    fn board_err(error: ticket_api::BoardError) -> McpError {
        match error {
            ticket_api::BoardError::Storage(storage_error) => Self::store_err(storage_error.into()),
            other => McpError::invalid_params(other.to_string(), None),
        }
    }

    fn is_ticket_store_root(path: &Path) -> bool {
        path.join("tickets").is_dir()
            || path.join("tickets.db").is_file()
            || path.join("search_index").is_dir()
    }

    fn resolve_workspace_root(&self, workspace: &str) -> Result<PathBuf, McpError> {
        let workspace = workspace.trim();
        if workspace.is_empty() || workspace == "default" {
            return Ok(
                ticket_api::workspace::resolve_workspace_root_from_store_root(
                    &self.index_root,
                    ticket_api::workspace::TICKET_INDEX_DIR,
                ),
            );
        }

        let normalized =
            ticket_api::workspace::normalize_explicit_workspace_selector(Some(workspace))
                .map_err(|error| McpError::invalid_params(error.to_string(), None))?;
        let workspace = normalized.to_string_lossy();
        let requested_path = Path::new(workspace.as_ref());
        let resolution = TicketStore::resolve_workspace_store(
            requested_path,
            memory_kernel::domain_store::StoreAccessMode::ReadOnly,
        )
        .map_err(Self::store_err)?;
        if Self::is_ticket_store_root(requested_path) || requested_path.is_dir() {
            return Ok(resolution.local_workspace);
        }

        Err(McpError::invalid_params(
            format!(
                "invalid workspace '{workspace}': expected 'default', a repo root containing .ticket, the .ticket store itself, a path inside that store, or an existing ticket store root"
            ),
            None,
        ))
    }

    fn resolve_explicit_workspace_root(&self, workspace: &str) -> Result<PathBuf, McpError> {
        let workspace =
            ticket_api::workspace::normalize_explicit_workspace_selector(Some(workspace))
                .map_err(|error| McpError::invalid_params(error.to_string(), None))?;
        self.resolve_workspace_root(&workspace.to_string_lossy())
    }

    fn resolve_uuid_with(store: &TicketStore, value: &str) -> Result<Uuid, McpError> {
        ticket_api::query_helpers::resolve_uuid_with_prefix(store, value).map_err(Self::store_err)
    }

    fn resolve_uuid_for_read(store: &TicketStore, value: &str) -> Result<Uuid, McpError> {
        match ticket_api::query_helpers::resolve_uuid_with_prefix(store, value) {
            Ok(id) => Ok(id),
            Err(ticket_api::error::StorageError::Other(message))
                if message.starts_with("no ticket found matching prefix") =>
            {
                let searched = store
                    .list_scan_roots()
                    .map_err(Self::store_err)?
                    .into_iter()
                    .map(|root| root.path.display().to_string())
                    .collect::<Vec<_>>();
                let searched = if searched.is_empty() {
                    store.index_root.display().to_string()
                } else {
                    searched.join(", ")
                };
                Err(McpError::invalid_params(
                    format!("{message}; searched workspaces: {searched}"),
                    None,
                ))
            }
            Err(error) => Err(Self::store_err(error)),
        }
    }

    async fn with_store<T>(
        &self,
        workspace: &str,
        f: impl FnOnce(&TicketStore) -> Result<T, ticket_api::error::StorageError>,
    ) -> Result<T, McpError> {
        let index_root = self.resolve_workspace_root(workspace)?;
        let _guard = self.store_lock.lock().await;
        let store = TicketStore::open_or_init_in_workspace(&index_root).map_err(Self::store_err)?;
        store.scan(false).map_err(Self::store_err)?;
        let result = f(&store).map_err(Self::store_err);
        drop(store);
        result
    }

    async fn with_store_ext<T>(
        &self,
        workspace: &str,
        f: impl FnOnce(&TicketStore) -> Result<T, McpError>,
    ) -> Result<T, McpError> {
        let index_root = self.resolve_explicit_workspace_root(workspace)?;
        self.with_resolved_store_ext(index_root, f).await
    }

    async fn with_read_store_ext<T>(
        &self,
        workspace: Option<&str>,
        f: impl FnOnce(&TicketStore) -> Result<T, McpError>,
    ) -> Result<T, McpError> {
        let index_root = match workspace {
            Some(workspace) => self.resolve_explicit_workspace_root(workspace)?,
            None => self.resolve_workspace_root("default")?,
        };
        self.with_resolved_store_ext(index_root, f).await
    }

    async fn with_resolved_store_ext<T>(
        &self,
        index_root: PathBuf,
        f: impl FnOnce(&TicketStore) -> Result<T, McpError>,
    ) -> Result<T, McpError> {
        let _guard = self.store_lock.lock().await;
        let store = TicketStore::open_or_init_in_workspace(&index_root).map_err(Self::store_err)?;
        store.scan(false).map_err(Self::store_err)?;
        let result = f(&store);
        drop(store);
        result
    }
}

#[tool_handler]
impl ServerHandler for TicketServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            server_info: Implementation {
                name: env!("CARGO_PKG_NAME").to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
                ..Default::default()
            },
            instructions: Some(
                "ticket-mcp provides direct access to the ticket store. No HTTP backend required. Use named tools for ticket operations. Call ticket_capabilities to discover the canonical ticket/spec/rule workflows, required params, and nested-root targeting."
                    .to_string(),
            ),
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            ..Default::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn explicit_workspace_resolution_does_not_fall_back_to_parent_store() {
        let root = tempfile::tempdir().unwrap();
        let parent = root.path().join("parent");
        let selected = parent.join("selected");
        std::fs::create_dir_all(&selected).unwrap();
        let parent_store = TicketStore::init_in_workspace(&parent).unwrap();

        let server = TicketServer::new(parent_store.index_root.clone());
        let resolved = server
            .resolve_workspace_root(selected.to_str().unwrap())
            .unwrap();

        assert_eq!(resolved, selected);
        let selected_store = TicketStore::open_or_init_in_workspace(&resolved).unwrap();
        assert_eq!(
            selected_store.index_root,
            ticket_api::workspace::canonical_store_root(&selected, ".ticket")
        );
    }
}

pub async fn run_mcp_server(
    index_root: PathBuf,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let server = TicketServer::new(index_root);

    tracing::info!("Starting ticket-mcp server on stdio (direct store access)");

    let service = server.serve(stdio()).await.inspect_err(|error| {
        eprintln!("Server error: {error:?}");
    })?;

    service.waiting().await?;
    Ok(())
}

#[path = "server/ticket_tools.rs"]
mod ticket_tools;
