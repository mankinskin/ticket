#[allow(unused_imports)]
use super::*;

pub(in super::super) async fn handle_subgraph(
    state: AppState,
    request_id: String,
    params: SubgraphQuery,
) -> Response {
    tracing::debug!(
        workspace = %params.workspace,
        root = %params.root,
        depth = params.depth,
        request_id = %request_id,
        "subgraph request received"
    );
    run_graph_request(state, request_id, GraphRequest::from_subgraph(params)).await
}

pub(in super::super) async fn handle_topgraph(
    state: AppState,
    request_id: String,
    params: TopgraphQuery,
) -> Response {
    run_graph_request(state, request_id, GraphRequest::from_topgraph(params)).await
}

pub(in super::super) async fn handle_workspace_graph(
    state: AppState,
    request_id: String,
    params: WorkspaceGraphQuery,
) -> Response {
    tracing::debug!(
        workspace = %params.workspace,
        request_id = %request_id,
        "workspace graph request received"
    );

    let task_request_id = request_id.clone();
    let request = WorkspaceGraphRequest::from_query(params);
    tokio::task::spawn_blocking(move || workspace_graph(state, &task_request_id, request))
        .await
        .unwrap_or_else(|_| task_join_err(&request_id, "workspace graph request"))
}

pub(super) async fn run_graph_request(
    state: AppState,
    request_id: String,
    request: GraphRequest,
) -> Response {
    let task_request_id = request_id.clone();
    tokio::task::spawn_blocking(move || bfs_graph(state, &task_request_id, request))
        .await
        .unwrap_or_else(|_| task_join_err(&request_id, "graph traversal request"))
}
