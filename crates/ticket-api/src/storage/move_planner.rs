//! Ticket-domain adapter onto the domain-neutral move kernel.
//!
//! The generic move machinery (preflight planning, journaled execution, git
//! topology, path-reference rewriting) lives in
//! [`memory_kernel::storage::move_kernel`]. This module supplies the ticket-domain
//! specialization via [`TicketMoveDomain`] and re-exports the kernel types under
//! the names the ticket surfaces (CLI/MCP/HTTP) consume.

use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Path, PathBuf},
};

use memory_kernel::storage::move_kernel::{
    self, MoveBoardState, MoveDomain, MoveError, MoveLeaseBlock, MoveReferences, MoveResult,
    MoveSetPlan,
};
use uuid::Uuid;

use crate::{
    error::StorageError,
    storage::{BoardEntry, BoardEntryStatus, store::TicketStore},
    workspace,
};

// Re-export the neutral kernel types as the ticket move surface. The move
// command is domain-neutral now; these aliases keep the existing public paths
// (`ticket_api::storage::move_planner::MovePreflightReport`, etc.) stable.
pub use memory_kernel::storage::move_kernel::{
    GitWorktreeTopology, MoveBlocker as MovePreflightBlocker, MovePlan as MovePreflightReport,
    MoveReferenceDirection, MoveReferenceVisibility,
};

/// Map a ticket [`StorageError`] into a kernel [`MoveError`].
fn to_move_error<E>(error: E) -> MoveError
where
    E: Into<StorageError>,
{
    let error: StorageError = error.into();
    match error {
        StorageError::Io(io) => MoveError::Io(io),
        other => MoveError::Domain(other.to_string()),
    }
}

/// Map a kernel [`MoveError`] back into a ticket [`StorageError`].
pub(crate) fn from_move_error(error: MoveError) -> StorageError {
    match error {
        MoveError::Io(io) => StorageError::Io(io),
        MoveError::Domain(message) => StorageError::Other(message),
        MoveError::InteroperabilityContract {
            artifact_class,
            detail,
        } => StorageError::Other(format!(
            "interoperability contract violation for {artifact_class}: {detail}"
        )),
    }
}

fn map_board_error(error: crate::storage::BoardError) -> MoveError {
    match error {
        crate::storage::BoardError::Storage(storage_error) => to_move_error(storage_error),
        other => MoveError::Domain(other.to_string()),
    }
}

fn ticket_entity_root(store_root: &Path) -> PathBuf {
    let index_root = if store_root.file_name().and_then(|name| name.to_str()) == Some("ticket")
        && store_root
            .parent()
            .and_then(|parent| parent.file_name())
            .and_then(|name| name.to_str())
            == Some(".workflow-tools")
    {
        store_root.to_path_buf()
    } else {
        workspace::canonical_store_root(store_root, workspace::TICKET_INDEX_DIR)
    };

    index_root.join("tickets")
}

fn ticket_path_belongs_to_store(store_root: &Path, ticket_path: &Path) -> bool {
    let entity_root = std::fs::canonicalize(ticket_entity_root(store_root))
        .unwrap_or_else(|_| ticket_entity_root(store_root));
    let ticket_path =
        std::fs::canonicalize(ticket_path).unwrap_or_else(|_| ticket_path.to_path_buf());
    ticket_path.starts_with(entity_root)
}

/// Ticket-domain implementation of the move kernel's [`MoveDomain`] trait.
pub(crate) struct TicketMoveDomain<'a> {
    store: &'a TicketStore,
}

impl<'a> TicketMoveDomain<'a> {
    pub(crate) fn new(store: &'a TicketStore) -> Self {
        Self { store }
    }

    fn open(&self, store_root: &Path) -> MoveResult<TicketStore> {
        TicketStore::open_with(store_root, self.store.schema_registry().clone())
            .map_err(to_move_error)
    }
}

impl MoveDomain for TicketMoveDomain<'_> {
    fn entity_subdir(&self) -> &str {
        "tickets"
    }

    fn store_index_dir(&self) -> &str {
        workspace::TICKET_INDEX_DIR
    }

    fn source_store_root(&self) -> PathBuf {
        self.store.index_root.clone()
    }

    fn source_entity_path(&self, entity_id: &Uuid) -> MoveResult<Option<PathBuf>> {
        Ok(self
            .store
            .get_indexed(entity_id)
            .map_err(to_move_error)?
            .and_then(|ticket| {
                ticket_path_belongs_to_store(&self.store.index_root, &ticket.path)
                    .then_some(ticket.path)
            }))
    }

    fn source_entity_paths_for_set(
        &self,
        entity_ids: &[Uuid],
    ) -> MoveResult<BTreeMap<Uuid, PathBuf>> {
        let indexed = self
            .store
            .get_indexed_many(entity_ids)
            .map_err(to_move_error)?;
        Ok(entity_ids
            .iter()
            .filter_map(|entity_id| {
                indexed
                    .get(entity_id)
                    .filter(|ticket| {
                        ticket_path_belongs_to_store(&self.store.index_root, &ticket.path)
                    })
                    .map(|ticket| (*entity_id, ticket.path.clone()))
            })
            .collect())
    }

    fn related_entities(&self, entity_id: &Uuid) -> MoveResult<MoveReferences> {
        let mut references = MoveReferences::default();
        for edge in self.store.list_all_edges().map_err(to_move_error)? {
            if edge.from == *entity_id {
                references.outbound.push(edge.to);
            }
            if edge.to == *entity_id {
                references.inbound.push(edge.from);
            }
        }
        Ok(references)
    }

    fn related_entities_for_set(
        &self,
        entity_ids: &[Uuid],
    ) -> MoveResult<BTreeMap<Uuid, MoveReferences>> {
        let mut references = entity_ids
            .iter()
            .map(|entity_id| (*entity_id, MoveReferences::default()))
            .collect::<BTreeMap<_, _>>();
        for edge in self.store.list_all_edges().map_err(to_move_error)? {
            if let Some(entry) = references.get_mut(&edge.from) {
                entry.outbound.push(edge.to);
            }
            if let Some(entry) = references.get_mut(&edge.to) {
                entry.inbound.push(edge.from);
            }
        }
        Ok(references)
    }

    fn target_store_present(&self, target_store_root: &Path) -> MoveResult<bool> {
        match TicketStore::open_with(target_store_root, self.store.schema_registry().clone()) {
            Ok(_) => Ok(true),
            Err(StorageError::WorkspaceNotFound { .. }) => Ok(false),
            Err(error) => Err(to_move_error(error)),
        }
    }

    fn entity_indexed_in(&self, store_root: &Path, entity_id: &Uuid) -> MoveResult<bool> {
        let store = self.open(store_root)?;
        Ok(store
            .get_indexed(entity_id)
            .map_err(to_move_error)?
            .map(|ticket| ticket_path_belongs_to_store(store_root, &ticket.path))
            .unwrap_or(false))
    }

    fn entity_indexed_in_many(
        &self,
        store_root: &Path,
        entity_ids: &[Uuid],
    ) -> MoveResult<BTreeMap<Uuid, bool>> {
        let store = self.open(store_root)?;
        let indexed = store.get_indexed_many(entity_ids).map_err(to_move_error)?;
        Ok(entity_ids
            .iter()
            .map(|entity_id| {
                (
                    *entity_id,
                    indexed
                        .get(entity_id)
                        .map(|ticket| ticket_path_belongs_to_store(store_root, &ticket.path))
                        .unwrap_or(false),
                )
            })
            .collect())
    }

    fn board_state(&self, entity_id: &Uuid) -> MoveResult<MoveBoardState> {
        let mut state = MoveBoardState::default();
        let snapshot = self.store.board_show(None).map_err(map_board_error)?;
        for entry in snapshot.entries {
            if entry.ticket_id == *entity_id
                && (entry.status == BoardEntryStatus::Active
                    || entry.status == BoardEntryStatus::Stale)
            {
                state.active_entries.push(entry);
            }
        }
        let history = self.store.board_history(None).map_err(map_board_error)?;
        for entry in history.entries {
            if entry.ticket_id == *entity_id {
                state.historical_entries.push(entry);
            }
        }
        Ok(state)
    }

    fn board_state_for_set(
        &self,
        entity_ids: &[Uuid],
    ) -> MoveResult<BTreeMap<Uuid, MoveBoardState>> {
        let mut states = entity_ids
            .iter()
            .map(|entity_id| (*entity_id, MoveBoardState::default()))
            .collect::<BTreeMap<_, _>>();
        for entry in self
            .store
            .board_show(None)
            .map_err(map_board_error)?
            .entries
        {
            if (entry.status == BoardEntryStatus::Active || entry.status == BoardEntryStatus::Stale)
                && let Some(state) = states.get_mut(&entry.ticket_id)
            {
                state.active_entries.push(entry);
            }
        }
        for entry in self
            .store
            .board_history(None)
            .map_err(map_board_error)?
            .entries
        {
            if let Some(state) = states.get_mut(&entry.ticket_id) {
                state.historical_entries.push(entry);
            }
        }
        Ok(states)
    }

    fn active_leases(&self, entity_id: &Uuid) -> MoveResult<Vec<MoveLeaseBlock>> {
        let mut leases = Vec::new();
        for lease in self.store.list_leases().map_err(to_move_error)? {
            if lease.ticket_id == *entity_id {
                leases.push(MoveLeaseBlock {
                    entity_id: lease.ticket_id,
                    working_by: lease.working_by.clone(),
                });
            }
        }
        Ok(leases)
    }

    fn active_leases_for_set(
        &self,
        entity_ids: &[Uuid],
    ) -> MoveResult<BTreeMap<Uuid, Vec<MoveLeaseBlock>>> {
        let mut leases = entity_ids
            .iter()
            .map(|entity_id| (*entity_id, Vec::new()))
            .collect::<BTreeMap<_, _>>();
        for lease in self.store.list_leases().map_err(to_move_error)? {
            if let Some(entries) = leases.get_mut(&lease.ticket_id) {
                entries.push(MoveLeaseBlock {
                    entity_id: lease.ticket_id,
                    working_by: lease.working_by.clone(),
                });
            }
        }
        Ok(leases)
    }

    fn migrate_board_history(
        &self,
        target_store_root: &Path,
        entity_id: &Uuid,
    ) -> MoveResult<Vec<BoardEntry>> {
        let target_store = self.open(target_store_root)?;
        let entries = self
            .store
            .board_list_entries_for_ticket(entity_id)
            .map_err(map_board_error)?;

        let mut historical_entries = Vec::new();
        for entry in entries {
            if entry.status == BoardEntryStatus::Active || entry.status == BoardEntryStatus::Stale {
                return Err(MoveError::Domain(format!(
                    "cannot move entity {} while board entry {} is active/stale",
                    entity_id, entry.entry_id
                )));
            }
            historical_entries.push(entry);
        }

        if historical_entries.is_empty() {
            return Ok(Vec::new());
        }

        target_store
            .board_import_entries(&historical_entries)
            .map_err(map_board_error)?;
        let ids: Vec<Uuid> = historical_entries
            .iter()
            .map(|entry| entry.entry_id)
            .collect();
        self.store
            .board_delete_entries(&ids)
            .map_err(map_board_error)?;

        Ok(historical_entries)
    }

    fn migrate_board_history_for_set(
        &self,
        target_store_root: &Path,
        entity_ids: &[Uuid],
    ) -> MoveResult<BTreeMap<Uuid, Vec<BoardEntry>>> {
        let requested = entity_ids.iter().copied().collect::<BTreeSet<_>>();
        let mut entries_by_entity = entity_ids
            .iter()
            .map(|entity_id| (*entity_id, Vec::new()))
            .collect::<BTreeMap<_, _>>();
        for entry in self
            .store
            .board_show(None)
            .map_err(map_board_error)?
            .entries
        {
            if (entry.status == BoardEntryStatus::Active || entry.status == BoardEntryStatus::Stale)
                && requested.contains(&entry.ticket_id)
            {
                return Err(MoveError::Domain(format!(
                    "cannot move entity {} while board entry {} is active/stale",
                    entry.ticket_id, entry.entry_id
                )));
            }
        }
        for entry in self
            .store
            .board_history(None)
            .map_err(map_board_error)?
            .entries
        {
            if let Some(entries) = entries_by_entity.get_mut(&entry.ticket_id) {
                entries.push(entry);
            }
        }
        let historical_entries = entries_by_entity
            .values()
            .flat_map(|entries| entries.iter().cloned())
            .collect::<Vec<_>>();
        if historical_entries.is_empty() {
            return Ok(entries_by_entity);
        }
        let target_store = self.open(target_store_root)?;
        target_store
            .board_import_entries(&historical_entries)
            .map_err(map_board_error)?;
        let ids = historical_entries
            .iter()
            .map(|entry| entry.entry_id)
            .collect::<Vec<_>>();
        self.store
            .board_delete_entries(&ids)
            .map_err(map_board_error)?;
        Ok(entries_by_entity)
    }

    fn restore_board_history(
        &self,
        target_store_root: &Path,
        entries: &[BoardEntry],
    ) -> MoveResult<()> {
        if entries.is_empty() {
            return Ok(());
        }
        let target_store = self.open(target_store_root)?;
        self.store
            .board_import_entries(entries)
            .map_err(map_board_error)?;
        let ids: Vec<Uuid> = entries.iter().map(|entry| entry.entry_id).collect();
        target_store
            .board_delete_entries(&ids)
            .map_err(map_board_error)?;
        Ok(())
    }

    fn scan_store(&self, store_root: &Path) -> MoveResult<()> {
        let store = self.open(store_root)?;
        store.scan(false).map_err(to_move_error)?;
        Ok(())
    }

    fn reconcile_store_touched(
        &self,
        store_root: &Path,
        touched_entity_ids: &[Uuid],
    ) -> MoveResult<()> {
        let store = self.open(store_root)?;
        store
            .reconcile_known_tickets(touched_entity_ids)
            .map_err(to_move_error)?;
        Ok(())
    }
}

impl TicketStore {
    /// Build a read-only preflight plan for moving `ticket_id` to
    /// `target_workspace_root`, delegating to the domain-neutral move kernel.
    pub fn plan_move_preflight(
        &self,
        ticket_id: &Uuid,
        target_workspace_root: &Path,
    ) -> Result<MovePreflightReport, StorageError> {
        let domain = TicketMoveDomain::new(self);
        move_kernel::plan_move(&domain, ticket_id, target_workspace_root).map_err(from_move_error)
    }

    /// Build one normalized preflight plan for a set of tickets.
    pub fn plan_move_set(
        &self,
        ticket_ids: &[Uuid],
        target_workspace_root: &Path,
    ) -> Result<MoveSetPlan, StorageError> {
        let domain = TicketMoveDomain::new(self);
        move_kernel::plan_move_set(&domain, ticket_ids, target_workspace_root)
            .map_err(from_move_error)
    }
}

#[cfg(test)]
#[path = "move_planner/tests.rs"]
mod tests;
