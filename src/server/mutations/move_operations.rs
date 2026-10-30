#[allow(unused_imports)]
use super::*;

impl TicketServer {
    pub(crate) async fn move_preflight_tool(
        &self,
        input: MovePreflightInput,
    ) -> Result<CallToolResult, McpError> {
        let tool_request_id = Uuid::new_v4();
        let workspace = input.workspace;
        let id_str = input.id;
        let target_workspace = normalize_workspace_root(&input.to_workspace_root)?;
        let span = tracing::info_span!(
            target: "ticket_mcp::transport",
            "ticket_mcp_move_preflight",
            tool_request_id = %tool_request_id,
            workspace = %workspace,
            ticket_ref = %id_str,
            target_workspace_root = %target_workspace.display(),
        );
        let plan = self
            .with_store_ext(&workspace.clone(), move |store| {
                let id = Self::resolve_uuid_with(store, &id_str)?;
                let report = store
                    .plan_move_preflight(&id, &target_workspace)
                    .map_err(Self::store_err)?;
                Ok((id, report))
            })
            .await?;

        tracing::info!(
            target: "ticket_mcp::transport",
            parent: &span,
            tool_request_id = %tool_request_id,
            workspace = %workspace,
            ticket_id = %plan.0,
            supported = plan.1.supported(),
            blockers = plan.1.blockers.len(),
            "ticket_mcp_move_preflight_complete"
        );

        Self::json_result(&serde_json::json!({
            "workspace": workspace,
            "status": "ok",
            "mode": "preflight",
            "id": plan.0.to_string(),
            "plan": move_plan_json(&plan.1)?,
            "recovery": move_recovery_json(),
        }))
    }

    pub(crate) async fn move_apply_tool(
        &self,
        input: MoveApplyInput,
    ) -> Result<CallToolResult, McpError> {
        let tool_request_id = Uuid::new_v4();
        let workspace = input.workspace;
        let id_str = input.id;
        let target_workspace = normalize_workspace_root(&input.to_workspace_root)?;
        let span = tracing::info_span!(
            target: "ticket_mcp::transport",
            "ticket_mcp_move_apply",
            tool_request_id = %tool_request_id,
            workspace = %workspace,
            ticket_ref = %id_str,
            target_workspace_root = %target_workspace.display(),
            journal_id = tracing::field::Empty,
        );
        let (id, report, outcome) = self
            .with_store_ext(&workspace.clone(), move |store| {
                let id = Self::resolve_uuid_with(store, &id_str)?;
                let report = store
                    .plan_move_preflight(&id, &target_workspace)
                    .map_err(Self::store_err)?;
                if !report.supported() {
                    let blockers = serde_json::to_string(&report.blockers)
                        .unwrap_or_else(|_| "[]".to_string());
                    return Err(McpError::invalid_params(
                        format!(
                            "move preflight blocked; run move_preflight for details. blockers={blockers}"
                        ),
                        None,
                    ));
                }
                let outcome = store
                    .execute_move_with_journal(&report)
                    .map_err(Self::store_err)?;
                Ok((id, report, outcome))
            })
            .await?;

        span.record("journal_id", outcome.journal.id.to_string());
        tracing::info!(
            target: "ticket_mcp::transport",
            parent: &span,
            tool_request_id = %tool_request_id,
            workspace = %workspace,
            ticket_id = %id,
            journal_id = %outcome.journal.id,
            phase = ?outcome.journal.phase,
            "ticket_mcp_move_apply_complete"
        );

        Self::json_result(&serde_json::json!({
            "workspace": workspace,
            "status": "ok",
            "mode": "apply",
            "id": id.to_string(),
            "plan": move_plan_json(&report)?,
            "outcome": move_outcome_json(&outcome),
            "recovery": move_recovery_json(),
        }))
    }

    pub(crate) async fn move_resume_tool(
        &self,
        input: MoveJournalInput,
    ) -> Result<CallToolResult, McpError> {
        let tool_request_id = Uuid::new_v4();
        let workspace = input.workspace;
        let journal_id = input.id.parse::<uuid::Uuid>().map_err(|error| {
            McpError::invalid_params(
                format!("invalid move journal id '{}': {error}", input.id),
                None,
            )
        })?;
        let span = tracing::info_span!(
            target: "ticket_mcp::transport",
            "ticket_mcp_move_resume",
            tool_request_id = %tool_request_id,
            workspace = %workspace,
            journal_id = %journal_id,
        );

        let outcome = self
            .with_store_ext(&workspace.clone(), move |store| {
                store
                    .resume_move_with_journal(journal_id)
                    .map_err(Self::store_err)
            })
            .await?;

        tracing::info!(
            target: "ticket_mcp::transport",
            parent: &span,
            tool_request_id = %tool_request_id,
            workspace = %workspace,
            journal_id = %outcome.journal.id,
            phase = ?outcome.journal.phase,
            resumed = outcome.resumed,
            "ticket_mcp_move_resume_complete"
        );

        Self::json_result(&serde_json::json!({
            "workspace": workspace,
            "status": "ok",
            "mode": "resume",
            "outcome": move_outcome_json(&outcome),
            "recovery": move_recovery_json(),
        }))
    }

    pub(crate) async fn move_rollback_tool(
        &self,
        input: MoveJournalInput,
    ) -> Result<CallToolResult, McpError> {
        let tool_request_id = Uuid::new_v4();
        let workspace = input.workspace;
        let journal_id = input.id.parse::<uuid::Uuid>().map_err(|error| {
            McpError::invalid_params(
                format!("invalid move journal id '{}': {error}", input.id),
                None,
            )
        })?;
        let span = tracing::info_span!(
            target: "ticket_mcp::transport",
            "ticket_mcp_move_rollback",
            tool_request_id = %tool_request_id,
            workspace = %workspace,
            journal_id = %journal_id,
        );

        let outcome = self
            .with_store_ext(&workspace.clone(), move |store| {
                store
                    .rollback_move_with_journal(journal_id)
                    .map_err(Self::store_err)
            })
            .await?;

        tracing::info!(
            target: "ticket_mcp::transport",
            parent: &span,
            tool_request_id = %tool_request_id,
            workspace = %workspace,
            journal_id = %outcome.journal.id,
            phase = ?outcome.journal.phase,
            rolled_back = outcome.rolled_back,
            "ticket_mcp_move_rollback_complete"
        );

        Self::json_result(&serde_json::json!({
            "workspace": workspace,
            "status": "ok",
            "mode": "rollback",
            "outcome": move_outcome_json(&outcome),
            "recovery": move_recovery_json(),
        }))
    }
}
