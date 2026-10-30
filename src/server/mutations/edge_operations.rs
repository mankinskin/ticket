#[allow(unused_imports)]
use super::*;

impl TicketServer {
    pub(crate) async fn add_edge_tool(
        &self,
        input: AddEdgeInput,
    ) -> Result<CallToolResult, McpError> {
        let workspace = input.workspace;
        let from_str = input.from;
        let to_str = input.to;
        let kind = input.kind;

        self.with_store_ext(&workspace.clone(), move |store| {
            let from = Self::resolve_uuid_with(store, &from_str)?;
            let to = Self::resolve_uuid_with(store, &to_str)?;
            let edge = EdgeRecord {
                from,
                to,
                kind: kind.clone(),
                created_at: chrono::Utc::now(),
            };
            store.add_edge(edge).map_err(Self::store_err)?;
            Self::json_result(&serde_json::json!({
                "workspace": workspace,
                "status": "ok",
                "edge": EdgeItem {
                    from: from.to_string(),
                    to: to.to_string(),
                    kind,
                },
            }))
        })
        .await
    }

    pub(crate) async fn remove_edge_tool(
        &self,
        input: RemoveEdgeInput,
    ) -> Result<CallToolResult, McpError> {
        let workspace = input.workspace;
        let from_str = input.from;
        let to_str = input.to;
        let kind = input.kind;

        self.with_store_ext(&workspace.clone(), move |store| {
            let edge = resolve_edge_for_remove(&from_str, &to_str, &kind, store)?;
            store.remove_edge(edge.clone()).map_err(Self::store_err)?;
            Self::json_result(&serde_json::json!({
                "workspace": workspace,
                "status": "ok",
                "removed": EdgeItem {
                    from: edge.from.to_string(),
                    to: edge.to.to_string(),
                    kind,
                },
            }))
        })
        .await
    }

    pub(crate) async fn prune_dangling_edges_tool(
        &self,
        input: PruneDanglingEdgesInput,
    ) -> Result<CallToolResult, McpError> {
        let workspace = input.workspace;
        let root_str = input.root;
        let all = input.all;
        let kind = input.kind;
        let strategy = input.strategy;
        let reason = input.reason;

        self.with_store_ext(&workspace.clone(), move |store| {
            let root = if all {
                None
            } else {
                let raw = root_str.as_deref().ok_or_else(|| {
                    McpError::invalid_params("root is required when all=false".to_string(), None)
                })?;
                Some(Self::resolve_uuid_with(store, raw)?)
            };

            let mut candidates = Vec::new();
            for edge in store.list_all_edges().map_err(Self::store_err)? {
                if edge.kind != kind {
                    continue;
                }
                if let Some(root_id) = root {
                    if edge.from != root_id {
                        continue;
                    }
                }
                let target_exists = store
                    .get_indexed(&edge.to)
                    .map_err(Self::store_err)?
                    .is_some();
                if !target_exists {
                    candidates.push(edge);
                }
            }

            let mut removed = 0usize;
            if strategy.mutates() {
                for edge in &candidates {
                    store.remove_edge(edge.clone()).map_err(Self::store_err)?;
                    removed += 1;
                }
            }

            let edges: Vec<EdgeItem> = candidates
                .iter()
                .map(|edge| EdgeItem {
                    from: edge.from.to_string(),
                    to: edge.to.to_string(),
                    kind: edge.kind.clone(),
                })
                .collect();

            Self::json_result(&serde_json::json!({
                "workspace": workspace,
                "status": "ok",
                "scope": {
                    "all": all,
                    "root": root.map(|id| id.to_string()),
                },
                "kind": kind,
                "strategy": strategy.as_str(),
                "mutated": strategy.mutates(),
                "candidate_count": edges.len(),
                "removed_count": removed,
                "reason": reason,
                "edges": edges,
            }))
        })
        .await
    }
}
