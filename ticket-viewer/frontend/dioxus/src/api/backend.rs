use crate::types::*;

/// Abstracts over the ticket REST API so components are not hard-wired to a
/// specific transport.
///
/// Note: futures are not required to be `Send` — WASM is single-threaded.
pub trait TicketBackend {
    fn list_workspaces(
        &self
    ) -> impl std::future::Future<Output = Result<WorkspacesResponse, String>>;

    fn list_tickets(
        &self,
        workspace: &str,
        state: Option<&str>,
        query: Option<&str>,
        limit: Option<u32>,
    ) -> impl std::future::Future<Output = Result<TicketsResponse, String>>;

    fn get_ticket(
        &self,
        workspace: &str,
        id: &str,
    ) -> impl std::future::Future<Output = Result<TicketDetailResponse, String>>;

    fn get_ticket_description(
        &self,
        workspace: &str,
        id: &str,
    ) -> impl std::future::Future<Output = Result<TicketDescriptionResponse, String>>;

    /// `GET /api/tickets/{id}?view=full` — parts, refs, and frozen state.
    fn get_ticket_full(
        &self,
        workspace: &str,
        id: &str,
    ) -> impl std::future::Future<Output = Result<TicketFullResponse, String>>;

    #[allow(dead_code)]
    fn get_subgraph(
        &self,
        workspace: &str,
        root: &str,
        depth: u32,
    ) -> impl std::future::Future<Output = Result<GraphSubgraphResponse, String>>;

    fn get_workspace_graph(
        &self,
        workspace: &str,
    ) -> impl std::future::Future<Output = Result<GraphSubgraphResponse, String>>;

    fn get_workflow_next(
        &self,
        workspace: &str,
    ) -> impl std::future::Future<Output = Result<WorkflowNextResponse, String>>;

    fn get_workflow_blockers(
        &self,
        workspace: &str,
        root: &str,
    ) -> impl std::future::Future<Output = Result<WorkflowTreeResponse, String>>;

    fn get_workflow_unblocked_by(
        &self,
        workspace: &str,
        root: &str,
    ) -> impl std::future::Future<Output = Result<WorkflowTreeResponse, String>>;

    fn patch_ticket(
        &self,
        workspace: &str,
        id: &str,
        patch: &TicketPatch,
    ) -> impl std::future::Future<Output = Result<TicketDetailResponse, String>>;

    fn list_schemas(
        &self,
        workspace: &str,
    ) -> impl std::future::Future<Output = Result<SchemaListResponse, String>>;

    fn create_edge(
        &self,
        workspace: &str,
        body: &EdgeMutationBody,
    ) -> impl std::future::Future<Output = Result<(), String>>;

    fn delete_edge(
        &self,
        workspace: &str,
        body: &EdgeMutationBody,
    ) -> impl std::future::Future<Output = Result<(), String>>;

    fn create_ticket(
        &self,
        workspace: &str,
        body: &CreateTicketRequest,
    ) -> impl std::future::Future<Output = Result<CreateTicketResponse, String>>;

    fn get_schema_by_type(
        &self,
        workspace: &str,
        type_id: &str,
    ) -> impl std::future::Future<Output = Result<SchemaDetailResponse, String>>;

    fn get_ticket_history(
        &self,
        workspace: &str,
        id: &str,
    ) -> impl std::future::Future<Output = Result<TicketHistoryResponse, String>>;

    fn revert_ticket(
        &self,
        workspace: &str,
        id: &str,
        revision: u64,
    ) -> impl std::future::Future<Output = Result<TicketDetailResponse, String>>;

    fn undo_ticket(
        &self,
        workspace: &str,
        id: &str,
    ) -> impl std::future::Future<Output = Result<TicketDetailResponse, String>>;

    fn update_ticket_description(
        &self,
        workspace: &str,
        id: &str,
        text: &str,
    ) -> impl std::future::Future<Output = Result<(), String>>;

    fn list_ticket_files(
        &self,
        workspace: &str,
        id: &str,
    ) -> impl std::future::Future<Output = Result<TicketFilesResponse, String>>;

    fn get_ticket_asset(
        &self,
        workspace: &str,
        id: &str,
        path: &str,
    ) -> impl std::future::Future<Output = Result<TicketAssetResponse, String>>;

    fn batch_tickets(
        &self,
        body: &BatchRequest,
    ) -> impl std::future::Future<Output = Result<BatchResponse, String>>;
}
