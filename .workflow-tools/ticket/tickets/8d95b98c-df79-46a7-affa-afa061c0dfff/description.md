Implemented child-workspace ownership fixes for ticket refs and follow-up reads.

Validation completed:
- Canonical workspace names now come from the owning workspace roots rather than synthetic aliases.
- Child-owned and ancestor-owned follow-up routes preserve explicit workspace ownership for downstream detail and history reads.
- Ticket HTTP storage misses return actionable API error envelopes instead of opaque generic failures.

Focused validation:
- `cargo test -p ticket-http descendant_ticket_ref_from_list_is_followable`
- `cargo test -p ticket-http ancestor_graph_ref_from_child_workspace_is_followable`
- `cargo test -p ticket-http io_not_found_maps_to_actionable_404`
- `cargo test -p ticket-http other_storage_errors_keep_specific_message`
- `cargo test -p ticket-http descendant_workspaces_use_workspace_root_name`
- `cargo test -p ticket-http preferred_active_workspace_prefers_primary_workspace`