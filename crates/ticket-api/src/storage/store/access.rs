#[allow(unused_imports)]
use super::*;

impl TicketStore {
    /// Read the full manifest for a ticket by ID.
    pub fn get(&self, id: &Uuid) -> Result<TicketManifest, StorageError> {
        let mut indexed = self.get_indexed(id)?.ok_or(StorageError::NotFound(*id))?;
        if self.is_external_worktree_path(&indexed.path) {
            self.scan(false)?;
            indexed = self.get_indexed(id)?.ok_or(StorageError::NotFound(*id))?;
        }
        TicketFs::read(&indexed.path)
    }

    /// Get just the indexed metadata (faster than a full read).
    pub fn get_indexed(&self, id: &Uuid) -> Result<Option<IndexedTicket>, StorageError> {
        Ok(self
            .index
            .get_ticket(id)?
            .map(|ticket| self.normalize_indexed_ticket(ticket)))
    }

    /// Fetch multiple tickets by ID in a single ReDB read transaction.
    ///
    /// Returns a `HashMap<Uuid, IndexedTicket>` for O(1) lookup. Missing
    /// IDs are omitted. Prefer this over N separate `get_indexed()`
    /// calls when you need metadata for a known set of IDs (e.g. BFS nodes).
    pub fn get_indexed_many(
        &self,
        ids: &[Uuid],
    ) -> Result<std::collections::HashMap<Uuid, IndexedTicket>, StorageError> {
        Ok(self
            .index
            .get_tickets_by_ids(ids)?
            .into_iter()
            .map(|(id, ticket)| (id, self.normalize_indexed_ticket(ticket)))
            .collect())
    }

    pub fn get_workflow_facts(
        &self,
        id: &Uuid,
    ) -> Result<Option<crate::storage::indexed::WorkflowFacts>, StorageError> {
        Ok(self.index.get_workflow_facts(id)?)
    }

    pub fn get_workflow_facts_many(
        &self,
        ids: &[Uuid],
    ) -> Result<HashMap<Uuid, crate::storage::indexed::WorkflowFacts>, StorageError> {
        Ok(self.index.get_workflow_facts_many(ids)?)
    }
}
