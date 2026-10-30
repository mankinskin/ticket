#[allow(unused_imports)]
use super::*;

pub(super) type AdjEntry = (Uuid, Uuid, Uuid, String);

#[derive(Clone)]
pub(super) struct RawEdgeItem {
    pub(super) from: Uuid,
    pub(super) to: Uuid,
    pub(super) kind: String,
}

pub(super) struct GraphRequest {
    pub(super) workspace: String,
    pub(super) root: Uuid,
    pub(super) direction: String,
    pub(super) edge_kind: Option<String>,
    pub(super) depth: usize,
    pub(super) limit_nodes: usize,
    pub(super) limit_edges: usize,
}

pub(super) struct TraversalResult {
    pub(super) visited: HashMap<Uuid, usize>,
    pub(super) edges: Vec<RawEdgeItem>,
    pub(super) truncated: bool,
    pub(super) max_depth_reached: usize,
}

pub(super) struct WorkspaceGraphRequest {
    pub(super) workspace: String,
    pub(super) edge_kind: Option<String>,
}

impl GraphRequest {
    pub(super) fn from_subgraph(params: SubgraphQuery) -> Self {
        Self {
            workspace: params.workspace,
            root: params.root,
            direction: params.direction.unwrap_or_else(|| "both".to_string()),
            edge_kind: params.edge_kind,
            depth: params.depth,
            limit_nodes: params.limit_nodes,
            limit_edges: params.limit_edges,
        }
    }

    pub(super) fn from_topgraph(params: TopgraphQuery) -> Self {
        Self {
            workspace: params.workspace,
            root: params.root,
            direction: params.direction.unwrap_or_else(|| "in".to_string()),
            edge_kind: params.edge_kind,
            depth: params.depth,
            limit_nodes: params.limit_nodes,
            limit_edges: params.limit_edges,
        }
    }

    pub(super) fn depth_limit(&self) -> usize {
        self.depth.min(8)
    }

    pub(super) fn edge_kind_filter(&self) -> &str {
        self.edge_kind.as_deref().unwrap_or("all")
    }
}

impl WorkspaceGraphRequest {
    pub(super) fn from_query(params: WorkspaceGraphQuery) -> Self {
        Self {
            workspace: params.workspace,
            edge_kind: params.edge_kind,
        }
    }

    pub(super) fn edge_kind_filter(&self) -> &str {
        self.edge_kind.as_deref().unwrap_or("all")
    }
}
