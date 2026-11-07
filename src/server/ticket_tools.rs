#[allow(unused_imports)]
use super::*;

#[tool_router(router = tool_router, vis = "pub(super)")]
impl TicketServer {
    #[tool(
        name = "health",
        description = "Check that the ticket store is accessible."
    )]
    pub(super) async fn health(&self) -> Result<CallToolResult, McpError> {
        self.health_tool().await
    }

    #[tool(
        name = "list_workspaces",
        description = "List available ticket workspaces. Requires an explicit workspace root or .ticket store path; omitted, empty, default and .. are rejected; . selects server cwd. Opens/initializes and rescans the addressed store."
    )]
    pub(super) async fn list_workspaces(&self) -> Result<CallToolResult, McpError> {
        self.list_workspaces_tool().await
    }

    #[tool(
        name = "ticket_capabilities",
        description = "List the canonical ticket/spec/rule workflows, their required parameters, and nested-root targeting semantics (self-describing capability catalog)."
    )]
    pub async fn ticket_capabilities(&self) -> Result<CallToolResult, McpError> {
        Self::json_result(&ticket_api::contracts::capability_catalog::capability_catalog())
    }

    #[tool(
        name = "list_tickets",
        description = "List tickets with state/type filters, or full-text query (query ignores state/type). Query limit defaults 100; otherwise unlimited; explicit limits cap at 1000. Returns count/items including effort. Empty/default workspace uses server store."
    )]
    pub async fn list_tickets(
        &self,
        Parameters(input): Parameters<ListTicketsInput>,
    ) -> Result<CallToolResult, McpError> {
        self.list_tickets_tool(input).await
    }

    #[tool(name = "get_ticket", description = "Read a ticket by UUID or 8+ hex prefix. Omitted/blank workspace uses server store. Default view summary; choose summary/plan/review/full or parts, not both. Returns projected ticket content and owning-store context.")]
    pub async fn get_ticket(
        &self,
        Parameters(input): Parameters<TicketRefInput>,
    ) -> Result<CallToolResult, McpError> {
        self.get_ticket_tool(input).await
    }

    #[tool(
        name = "get_ticket_description",
        description = "Read ticket markdown description by UUID or 8+ hex prefix. Omitted/blank workspace uses server store. view/parts are ignored here; use get_ticket for projections."
    )]
    pub(super) async fn get_ticket_description(
        &self,
        Parameters(input): Parameters<TicketRefInput>,
    ) -> Result<CallToolResult, McpError> {
        self.get_ticket_description_tool(input).await
    }

    #[tool(
        name = "list_edges",
        description = "List all graph edges, optionally filtered by kind; omitted/all means every kind. No pagination. Empty/default workspace uses server store."
    )]
    pub(super) async fn list_edges(
        &self,
        Parameters(input): Parameters<ListEdgesInput>,
    ) -> Result<CallToolResult, McpError> {
        self.list_edges_tool(input).await
    }

    #[tool(
        name = "subgraph",
        description = "BFS graph from root: direction out/in/both (default both), depth default 2 capped 8, node limit default 500, edge limit default 2000. Omitted/all edge_kind is unfiltered. Returns stats/truncated; inspect truncation before treating the result as complete. Requires an explicit workspace root or .ticket store path; omitted, empty, default and .. are rejected; . selects server cwd. Opens/initializes and rescans the addressed store."
    )]
    pub(super) async fn subgraph(
        &self,
        Parameters(input): Parameters<SubgraphInput>,
    ) -> Result<CallToolResult, McpError> {
        self.bfs_graph(
            input.workspace,
            &input.root,
            input.direction.as_deref().unwrap_or("both"),
            input.edge_kind.as_deref(),
            input.depth,
            input.limit_nodes,
            input.limit_edges,
        )
        .await
    }

    #[tool(
        name = "topgraph",
        description = "BFS reverse dependency graph from root: default direction in, overridable out/in/both. Same depth/node/edge defaults as subgraph; returns stats/truncated. Requires an explicit workspace root or .ticket store path; omitted, empty, default and .. are rejected; . selects server cwd. Opens/initializes and rescans the addressed store."
    )]
    pub(super) async fn topgraph(
        &self,
        Parameters(input): Parameters<TopgraphInput>,
    ) -> Result<CallToolResult, McpError> {
        self.bfs_graph(
            input.workspace,
            &input.root,
            input.direction.as_deref().unwrap_or("in"),
            input.edge_kind.as_deref(),
            input.depth,
            input.limit_nodes,
            input.limit_edges,
        )
        .await
    }

    #[tool(
        name = "next_tickets",
        description = "Select prerequisite-eligible tickets, excluding board-owned work. root scopes to frontier leaf blockers; omitted root selects global queue. filter is a title prefix. Limit defaults 20 without root, unlimited with root; supplied limit caps at 100. Returns eligible work and board exclusions; eligibility is not authority to dispatch or close. Requires an explicit workspace root or .ticket store path; omitted, empty, default and .. are rejected; . selects server cwd. Opens/initializes and rescans the addressed store."
    )]
    pub async fn next_tickets(
        &self,
        Parameters(input): Parameters<NextTicketsInput>,
    ) -> Result<CallToolResult, McpError> {
        self.next_tickets_tool(input).await
    }

    #[tool(
        name = "health_check",
        description = "Check descriptions/titles/dependency states/dangling edges. Scope precedence ids, all, root; one required. BFS depth defaults 6 capped 8, direction out. where entries are key=value; terminal tickets are excluded from tickets_checked. Requires an explicit workspace root or .ticket store path; omitted, empty, default and .. are rejected; . selects server cwd. Opens/initializes and rescans the addressed store."
    )]
    pub(super) async fn health_check(
        &self,
        Parameters(input): Parameters<HealthCheckInput>,
    ) -> Result<CallToolResult, McpError> {
        self.run_health_checks(
            &input.workspace,
            input.root.as_deref(),
            input.all,
            &input.ids,
            input.depth,
            input.direction.as_deref(),
            &input.r#where,
        )
        .await
    }

    #[tool(
        name = "update_ticket",
        description = "Update a ticket: apply field patches and/or transition state. Set undo=true to revert to the previous history revision. If description is provided, description_mode must also be provided; valid values are 'replace' and 'append'. There is no default. Omitting both description and description_mode leaves the description unchanged. Requires an explicit workspace root or .ticket store path; omitted, empty, default and .. are rejected; . selects server cwd. Opens/initializes and rescans the addressed store."
    )]
    pub async fn update_ticket(
        &self,
        Parameters(input): Parameters<UpdateTicketInput>,
    ) -> Result<CallToolResult, McpError> {
        self.update_ticket_tool(input).await
    }

    #[tool(
        name = "close_ticket",
        description = "Fast-forward through legal intermediate states to to_state (default done), returning traversed_states; not a force override of lifecycle guards. Requires an explicit workspace root or .ticket store path; omitted, empty, default and .. are rejected; . selects server cwd. Opens/initializes and rescans the addressed store."
    )]
    pub(super) async fn close_ticket(
        &self,
        Parameters(input): Parameters<CloseTicketInput>,
    ) -> Result<CallToolResult, McpError> {
        self.close_ticket_tool(input).await
    }

    #[tool(
        name = "cancel_ticket",
        description = "Fast-forward through legal intermediate states to cancelled, equivalent to close_ticket with to_state=cancelled; retains lifecycle guards. Requires an explicit workspace root or .ticket store path; omitted, empty, default and .. are rejected; . selects server cwd. Opens/initializes and rescans the addressed store."
    )]
    pub(super) async fn cancel_ticket(
        &self,
        Parameters(input): Parameters<CancelTicketInput>,
    ) -> Result<CallToolResult, McpError> {
        self.cancel_ticket_tool(input).await
    }

    #[tool(
        name = "create_ticket",
        description = "Create a ticket of required type with title and key=value string fields. Omitted state uses its workflow default. Returns created ticket identity; does not authorize execution. Requires an explicit workspace root or .ticket store path; omitted, empty, default and .. are rejected; . selects server cwd. Opens/initializes and rescans the addressed store."
    )]
    pub async fn create_ticket(
        &self,
        Parameters(input): Parameters<CreateTicketInput>,
    ) -> Result<CallToolResult, McpError> {
        self.create_ticket_tool(input).await
    }

    #[tool(
        name = "delete_ticket",
        description = "Permanently delete the ticket folder from disk, not a reversible cancellation. Requires an explicit workspace root or .ticket store path; omitted, empty, default and .. are rejected; . selects server cwd. Opens/initializes and rescans the addressed store."
    )]
    pub async fn delete_ticket(
        &self,
        Parameters(input): Parameters<DeleteTicketInput>,
    ) -> Result<CallToolResult, McpError> {
        self.delete_ticket_tool(input).await
    }

    #[tool(
        name = "add_edge",
        description = "Add a directed from-to edge between resolvable ticket UUIDs/prefixes. depends_on means the source depends on the target; kind may also be linked or another supported free-form label. Requires an explicit workspace root or .ticket store path; omitted, empty, default and .. are rejected; . selects server cwd. Opens/initializes and rescans the addressed store."
    )]
    pub(super) async fn add_edge(
        &self,
        Parameters(input): Parameters<AddEdgeInput>,
    ) -> Result<CallToolResult, McpError> {
        self.add_edge_tool(input).await
    }

    #[tool(
        name = "remove_edge",
        description = "Remove an edge of kind: from is a UUID or 8+ hex prefix; to is a UUID or prefix matched among the source's edges. Ambiguous/missing edges fail; a full missing-source UUID may still identify dangling edges. Requires an explicit workspace root or .ticket store path; omitted, empty, default and .. are rejected; . selects server cwd. Opens/initializes and rescans the addressed store."
    )]
    pub(super) async fn remove_edge(
        &self,
        Parameters(input): Parameters<RemoveEdgeInput>,
    ) -> Result<CallToolResult, McpError> {
        self.remove_edge_tool(input).await
    }

    #[tool(
        name = "prune_dangling_edges",
        description = "Remove/report only edges whose target is missing. Provide root or all=true. Default kind depends_on; strategy unlink mutates (default), reconcile_only reports. Returns candidate_count/removed_count/reason. Requires an explicit workspace root or .ticket store path; omitted, empty, default and .. are rejected; . selects server cwd. Opens/initializes and rescans the addressed store."
    )]
    pub(super) async fn prune_dangling_edges(
        &self,
        Parameters(input): Parameters<PruneDanglingEdgesInput>,
    ) -> Result<CallToolResult, McpError> {
        self.prune_dangling_edges_tool(input).await
    }

    #[tool(
        name = "move_preflight",
        description = "Preflight a ticket move to explicit to_workspace_root without moving entities. Inspect plan support/blockers; preflight does not authorize apply. Requires an explicit workspace root or .ticket store path; omitted, empty, default and .. are rejected; . selects server cwd. Opens/initializes and rescans the addressed store."
    )]
    pub async fn move_preflight(
        &self,
        Parameters(input): Parameters<MovePreflightInput>,
    ) -> Result<CallToolResult, McpError> {
        self.move_preflight_tool(input).await
    }

    #[tool(
        name = "move_apply",
        description = "Re-plan then move a ticket between stores; a blocked plan fails. Retain returned outcome journal identity for recovery, rather than blindly replaying apply. Requires an explicit workspace root or .ticket store path; omitted, empty, default and .. are rejected; . selects server cwd. Opens/initializes and rescans the addressed store."
    )]
    pub async fn move_apply(
        &self,
        Parameters(input): Parameters<MoveApplyInput>,
    ) -> Result<CallToolResult, McpError> {
        self.move_apply_tool(input).await
    }

    #[tool(
        name = "move_resume",
        description = "Resume an interrupted move using id as the move journal UUID from move_apply, not a ticket UUID. Requires an explicit workspace root or .ticket store path; omitted, empty, default and .. are rejected; . selects server cwd. Opens/initializes and rescans the addressed store."
    )]
    pub async fn move_resume(
        &self,
        Parameters(input): Parameters<MoveJournalInput>,
    ) -> Result<CallToolResult, McpError> {
        self.move_resume_tool(input).await
    }

    #[tool(
        name = "move_rollback",
        description = "Roll back a move using id as the move journal UUID from move_apply, not a ticket UUID. Requires an explicit workspace root or .ticket store path; omitted, empty, default and .. are rejected; . selects server cwd. Opens/initializes and rescans the addressed store."
    )]
    pub async fn move_rollback(
        &self,
        Parameters(input): Parameters<MoveJournalInput>,
    ) -> Result<CallToolResult, McpError> {
        self.move_rollback_tool(input).await
    }

    #[tool(
        name = "workflow",
        description = "Show ready-to-run ticket MCP call sequences for common tasks."
    )]
    pub(super) async fn workflow(
        &self,
        Parameters(input): Parameters<WorkflowInput>,
    ) -> Result<CallToolResult, McpError> {
        self.workflow_tool(input).await
    }

    #[tool(
        name = "board_show",
        description = "Read the current draftboard snapshot. Completed history is excluded. When agent_id is supplied, performs a follow-up heartbeat for the caller's active entries and returns the refreshed entry alongside the snapshot. Requires an explicit workspace root or .ticket store path; omitted, empty, default and .. are rejected; . selects server cwd. Opens/initializes and rescans the addressed store."
    )]
    pub async fn board_show(
        &self,
        Parameters(input): Parameters<BoardShowInput>,
    ) -> Result<CallToolResult, McpError> {
        self.board_show_tool(input).await
    }

    #[tool(
        name = "board_history",
        description = "Read recently completed board history separately from the live draftboard. Uses the configured completed_audit_window_secs as the default history window. Requires an explicit workspace root or .ticket store path; omitted, empty, default and .. are rejected; . selects server cwd. Opens/initializes and rescans the addressed store."
    )]
    pub async fn board_history(
        &self,
        Parameters(input): Parameters<BoardHistoryInput>,
    ) -> Result<CallToolResult, McpError> {
        self.board_history_tool(input).await
    }

    #[tool(
        name = "board_worktrees",
        description = "List active worktrees and their sessions, agents, and tickets. Requires an explicit workspace root or .ticket store path; omitted, empty, default and .. are rejected; . selects server cwd. Opens/initializes and rescans the addressed store."
    )]
    pub async fn board_worktrees(
        &self,
        Parameters(input): Parameters<BoardWorktreesInput>,
    ) -> Result<CallToolResult, McpError> {
        self.board_worktrees_tool(input).await
    }

    #[tool(
        name = "board_check_in",
        description = "Claim WIP/file ownership for ticket_id and agent_id. ttl_secs defaults 3600; files are claimed paths; intent defaults empty. Optional session/worktree/branch values are provenance. WIP/file conflicts fail. Requires an explicit workspace root or .ticket store path; omitted, empty, default and .. are rejected; . selects server cwd. Opens/initializes and rescans the addressed store."
    )]
    pub async fn board_check_in(
        &self,
        Parameters(input): Parameters<BoardCheckInInput>,
    ) -> Result<CallToolResult, McpError> {
        self.board_check_in_tool(input).await
    }

    #[tool(
        name = "board_check_out",
        description = "Check out WIP and release its lease. Omitted agent_id selects the first active entry for the ticket; reason records the handoff. Requires an explicit workspace root or .ticket store path; omitted, empty, default and .. are rejected; . selects server cwd. Opens/initializes and rescans the addressed store."
    )]
    pub async fn board_check_out(
        &self,
        Parameters(input): Parameters<BoardCheckOutInput>,
    ) -> Result<CallToolResult, McpError> {
        self.board_check_out_tool(input).await
    }

    #[tool(
        name = "board_release_lease",
        description = "Release a ticket lease using owner/stale semantics: the requester may always release its own lease, any requester may release stale leases, and live leases held by others return a lease-conflict error. Requires an explicit workspace root or .ticket store path; omitted, empty, default and .. are rejected; . selects server cwd. Opens/initializes and rescans the addressed store."
    )]
    pub async fn board_release_lease(
        &self,
        Parameters(input): Parameters<BoardReleaseLeaseInput>,
    ) -> Result<CallToolResult, McpError> {
        self.board_release_lease_tool(input).await
    }

    #[tool(
        name = "board_heartbeat",
        description = "Refresh a board entry's heartbeat; entry_id must be the full board-entry UUID, not a ticket ID or prefix. Requires an explicit workspace root or .ticket store path; omitted, empty, default and .. are rejected; . selects server cwd. Opens/initializes and rescans the addressed store."
    )]
    pub async fn board_heartbeat(
        &self,
        Parameters(input): Parameters<BoardHeartbeatInput>,
    ) -> Result<CallToolResult, McpError> {
        self.board_heartbeat_tool(input).await
    }

    #[tool(
        name = "board_configure",
        description = "Read board configuration when all options are omitted; supplied max_wip/stale_after_secs/completed_audit_window_secs patch and persist configuration. Requires an explicit workspace root or .ticket store path; omitted, empty, default and .. are rejected; . selects server cwd. Opens/initializes and rescans the addressed store."
    )]
    pub async fn board_configure(
        &self,
        Parameters(input): Parameters<BoardConfigureInput>,
    ) -> Result<CallToolResult, McpError> {
        self.board_configure_tool(input).await
    }

    #[tool(
        name = "board_clean_preview",
        description = "Preview board cleanup; include_stale defaults false. Returns a confirmation token for board_clean_apply; does not apply cleanup. Requires an explicit workspace root or .ticket store path; omitted, empty, default and .. are rejected; . selects server cwd. Opens/initializes and rescans the addressed store."
    )]
    pub async fn board_clean_preview(
        &self,
        Parameters(input): Parameters<BoardCleanPreviewInput>,
    ) -> Result<CallToolResult, McpError> {
        self.board_clean_preview_tool(input).await
    }

    #[tool(
        name = "board_clean_apply",
        description = "Apply the cleanup identified by a board_clean_preview token. include_stale defaults false; reuse the reviewed preview options, not a guessed token. Requires an explicit workspace root or .ticket store path; omitted, empty, default and .. are rejected; . selects server cwd. Opens/initializes and rescans the addressed store."
    )]
    pub async fn board_clean_apply(
        &self,
        Parameters(input): Parameters<BoardCleanApplyInput>,
    ) -> Result<CallToolResult, McpError> {
        self.board_clean_apply_tool(input).await
    }

    #[tool(
        name = "board_update_files",
        description = "Add/remove claimed paths on the active agent_id/ticket_id board entry; additions are conflict-checked. Requires an explicit workspace root or .ticket store path; omitted, empty, default and .. are rejected; . selects server cwd. Opens/initializes and rescans the addressed store."
    )]
    pub async fn board_update_files(
        &self,
        Parameters(input): Parameters<BoardUpdateFilesInput>,
    ) -> Result<CallToolResult, McpError> {
        self.board_update_files_tool(input).await
    }

    #[tool(
        name = "board_rename_file",
        description = "Replace old_path with new_path in the active agent_id/ticket_id ownership entry. Requires an explicit workspace root or .ticket store path; omitted, empty, default and .. are rejected; . selects server cwd. Opens/initializes and rescans the addressed store."
    )]
    pub async fn board_rename_file(
        &self,
        Parameters(input): Parameters<BoardRenameFileInput>,
    ) -> Result<CallToolResult, McpError> {
        self.board_rename_file_tool(input).await
    }

    #[tool(
        name = "help",
        description = "List ticket-mcp tools and their parameters."
    )]
    pub(super) async fn help(&self) -> Result<CallToolResult, McpError> {
        self.help_tool().await
    }

    #[tool(
        name = "list_parts",
        description = "List ticket content parts and orphan files; with_content defaults false. id is the ticket UUID/prefix. Requires an explicit workspace root or .ticket store path; omitted, empty, default and .. are rejected; . selects server cwd. Opens/initializes and rescans the addressed store."
    )]
    pub async fn list_parts(
        &self,
        Parameters(input): Parameters<ListPartsInput>,
    ) -> Result<CallToolResult, McpError> {
        self.list_parts_tool(input).await
    }

    #[tool(
        name = "get_part",
        description = "Read a ticket part; id is ticket UUID/prefix, part_id is a full opaque part UUID. Requires an explicit workspace root or .ticket store path; omitted, empty, default and .. are rejected; . selects server cwd. Opens/initializes and rescans the addressed store."
    )]
    pub async fn get_part(
        &self,
        Parameters(input): Parameters<GetPartInput>,
    ) -> Result<CallToolResult, McpError> {
        self.get_part_tool(input).await
    }

    #[tool(
        name = "write_part",
        description = "Write ticket part content, rejecting frozen parts. Omit part_id to create with generated identity; updating uses the full part UUID and ignores kind. author records provenance. Requires an explicit workspace root or .ticket store path; omitted, empty, default and .. are rejected; . selects server cwd. Opens/initializes and rescans the addressed store."
    )]
    pub async fn write_part(
        &self,
        Parameters(input): Parameters<WritePartInput>,
    ) -> Result<CallToolResult, McpError> {
        self.write_part_tool(input).await
    }

    #[tool(
        name = "write_amendment",
        description = "Write an amendment superseding a full part UUID; omit part_id for a generated amendment identity. author records provenance; this does not unfreeze the original. Requires an explicit workspace root or .ticket store path; omitted, empty, default and .. are rejected; . selects server cwd. Opens/initializes and rescans the addressed store."
    )]
    pub async fn write_amendment(
        &self,
        Parameters(input): Parameters<WriteAmendmentInput>,
    ) -> Result<CallToolResult, McpError> {
        self.write_amendment_tool(input).await
    }

    #[tool(
        name = "undo_part",
        description = "Restore the previous content of a full part_id UUID on the addressed ticket. Frozen parts cannot be undone; author is provenance. Requires an explicit workspace root or .ticket store path; omitted, empty, default and .. are rejected; . selects server cwd. Opens/initializes and rescans the addressed store."
    )]
    pub async fn undo_part(
        &self,
        Parameters(input): Parameters<UndoPartInput>,
    ) -> Result<CallToolResult, McpError> {
        self.undo_part_tool(input).await
    }
}
