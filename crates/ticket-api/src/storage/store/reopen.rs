use super::*;

impl TicketStore {
    /// Explicit done -> ready domain operation; never a general terminal-state
    /// patch. Retries reuse the exact operation, revision and reason.
    pub fn reopen_done(
        &self,
        id: &Uuid,
        expected_revision: u64,
        reason: &str,
        operation_id: Uuid,
        author: Option<&str>,
    ) -> Result<TicketManifest, StorageError> {
        let mut indexed = self.get_indexed(id)?.ok_or(StorageError::NotFound(*id))?;
        let schema = self
            .schema_registry
            .get(&indexed.type_id)
            .ok_or_else(|| StorageError::Other("ticket_transition_rejected: no schema".into()))?;
        if !schema.states.iter().any(|s| s == "ready") || !schema.states.iter().any(|s| s == "done")
        {
            return Err(StorageError::Other(
                "ticket_transition_rejected: schema lacks done/ready".into(),
            ));
        }
        let manifest = TicketFs::reopen_done(
            &indexed.path,
            expected_revision,
            reason,
            operation_id,
            author,
        )?;
        self.refresh_index_and_search(
            id,
            &BTreeMap::new(),
            &Some("ready".into()),
            &manifest,
            &mut indexed,
            Utc::now(),
        )?;
        self.refresh_workflow_facts_for_roots(&[*id], false, Utc::now())?;
        self.board_reconcile(id, false);
        Ok(manifest)
    }
}
