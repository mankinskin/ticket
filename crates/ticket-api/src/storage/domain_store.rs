use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

use memory_kernel::{
    domain_store::{
        CreateEntity, DeleteEntity, DomainStore, DomainStoreResolution, ListEntities, ReadEntity,
        StoreAccessMode, UpdateEntity,
    },
    model::domain::DomainId,
    workspace::StoreRootDiagnostic,
};
use serde_json::Value;

use crate::{
    error::StorageError,
    model::ticket::{TicketId, TicketManifest},
    storage::{indexed::IndexedTicket, store::TicketStore},
};

/// DomainStore input for creating a Ticket through the existing Ticket API.
#[derive(Debug, Clone)]
pub struct TicketCreateInput {
    pub id: Option<TicketId>,
    pub type_id: String,
    pub title: Option<String>,
    pub initial_state: Option<String>,
    pub extra: BTreeMap<String, Value>,
    pub target_root: Option<PathBuf>,
    pub body: Option<String>,
}

/// Ticket-specific update options forwarded to the existing lifecycle API.
#[derive(Debug, Clone, Default)]
pub struct TicketUpdatePatch {
    pub fields: BTreeMap<String, Value>,
    pub transition_states: Option<Vec<String>>,
    pub to_state: Option<String>,
    pub description: Option<String>,
    pub description_mode: Option<super::store::DescriptionUpdateMode>,
    pub author: Option<String>,
    pub single_hop: bool,
}

/// Ticket-specific filters for the optional `ListEntities` capability.
#[derive(Debug, Clone, Default)]
pub struct TicketListQuery {
    pub state_filter: Option<String>,
    pub type_filter: Option<String>,
    pub limit: Option<usize>,
    pub field_filters: Vec<(String, String)>,
}

impl DomainStore for TicketStore {
    fn domain_id() -> DomainId {
        DomainId::new("ticket").expect("ticket is a valid domain id")
    }

    fn store_dir_name() -> &'static str {
        ".ticket"
    }
}

impl TicketStore {
    /// Resolve a selected workspace and report legacy-layout diagnostics.
    ///
    /// Read-only resolution may select an existing `.ticket` store. Create/open
    /// resolution always selects the canonical `.workflow-tools/ticket` store.
    pub fn resolve_workspace_store(
        local_workspace: &Path,
        access_mode: StoreAccessMode,
    ) -> Result<DomainStoreResolution, StorageError> {
        let mut resolution = <Self as DomainStore>::resolve_store(local_workspace, access_mode)?;
        let legacy_root = resolution.local_workspace.join(".ticket");
        let canonical_root =
            memory_kernel::workspace::canonical_store_root(&resolution.local_workspace, ".ticket");
        let legacy_exists = legacy_root.is_dir();
        let canonical_exists = canonical_root.is_dir();

        if access_mode == StoreAccessMode::ReadOnly && legacy_exists && !canonical_exists {
            resolution.store_root = legacy_root.clone();
        }
        if legacy_exists {
            let diagnostic = if canonical_exists {
                StoreRootDiagnostic::BothLayoutsPresent {
                    domain: "ticket".to_string(),
                    legacy_path: legacy_root,
                    canonical_path: canonical_root,
                }
            } else {
                StoreRootDiagnostic::LegacyStore {
                    domain: "ticket".to_string(),
                    legacy_path: legacy_root,
                    canonical_path: canonical_root,
                }
            };
            if !resolution.diagnostics.contains(&diagnostic) {
                resolution.diagnostics.push(diagnostic);
            }
        }
        Ok(resolution)
    }
}

impl CreateEntity for TicketStore {
    type Entity = TicketManifest;
    type CreateInput = TicketCreateInput;
    type CreateResult = TicketId;
    type Error = StorageError;

    fn create_entity(&self, input: Self::CreateInput) -> Result<Self::CreateResult, Self::Error> {
        self.create(
            input.id,
            &input.type_id,
            input.title.as_deref(),
            input.initial_state.as_deref(),
            input.extra,
            input.target_root.as_deref(),
            input.body.as_deref(),
        )
    }
}

impl ReadEntity for TicketStore {
    type EntityId = TicketId;
    type Entity = TicketManifest;
    type ReadResult = TicketManifest;
    type Error = StorageError;

    fn read_entity(&self, id: Self::EntityId) -> Result<Self::ReadResult, Self::Error> {
        self.get(&id)
    }
}

impl UpdateEntity for TicketStore {
    type EntityId = TicketId;
    type Entity = TicketManifest;
    type Patch = TicketUpdatePatch;
    type UpdateResult = TicketManifest;
    type Error = StorageError;

    fn update_entity(
        &self,
        id: Self::EntityId,
        patch: Self::Patch,
    ) -> Result<Self::UpdateResult, Self::Error> {
        self.update_with_options(
            &id,
            patch.fields,
            patch.transition_states.as_deref(),
            patch.to_state.as_deref(),
            patch.description.as_deref(),
            patch.description_mode,
            patch.author.as_deref(),
            patch.single_hop,
        )
    }
}

impl DeleteEntity for TicketStore {
    type EntityId = TicketId;
    type DeleteResult = ();
    type Error = StorageError;

    fn delete_entity(&self, id: Self::EntityId) -> Result<Self::DeleteResult, Self::Error> {
        self.delete(&id)
    }
}

impl ListEntities for TicketStore {
    type Query = TicketListQuery;
    type Entity = IndexedTicket;
    type ListResult = Vec<IndexedTicket>;
    type Error = StorageError;

    fn list_entities(&self, query: Self::Query) -> Result<Self::ListResult, Self::Error> {
        self.list_extended(
            query.state_filter.as_deref(),
            query.type_filter.as_deref(),
            query.limit,
            &query.field_filters,
        )
    }
}

#[cfg(test)]
mod tests {
    use memory_kernel::{
        domain_store::{CreateEntity, ReadEntity, StoreAccessMode, UpdateEntity},
        workspace::canonical_store_root,
    };
    use serde_json::json;
    use tempfile::tempdir;

    use super::*;

    #[test]
    fn selected_workspace_create_and_update_read_back_only_from_its_canonical_store() {
        let parent = tempdir().unwrap();
        let selected = parent.path().join("selected");
        let sibling = parent.path().join("sibling");
        let ambient = parent.path().join("ambient");
        let parent_workspace = parent.path().to_path_buf();
        std::fs::create_dir_all(&selected).unwrap();
        std::fs::create_dir_all(&sibling).unwrap();
        std::fs::create_dir_all(&ambient).unwrap();

        let store = TicketStore::open_or_init_in_workspace(&selected).unwrap();
        let id = store
            .create_entity(TicketCreateInput {
                id: None,
                type_id: "tracker-improvement".to_string(),
                title: Some("Created through DomainStore".to_string()),
                initial_state: None,
                extra: BTreeMap::new(),
                target_root: None,
                body: None,
            })
            .unwrap();

        let created = store.read_entity(id).unwrap();
        assert_eq!(created.id, id);
        assert_eq!(
            created.extra.get("title").and_then(Value::as_str),
            Some("Created through DomainStore")
        );

        let updated = store
            .update_entity(
                id,
                TicketUpdatePatch {
                    fields: BTreeMap::from([(
                        "title".to_string(),
                        json!("Updated through DomainStore"),
                    )]),
                    ..TicketUpdatePatch::default()
                },
            )
            .unwrap();
        assert_eq!(
            updated.extra.get("title").and_then(Value::as_str),
            Some("Updated through DomainStore")
        );
        let read_back = store.read_entity(id).unwrap();
        assert_eq!(
            read_back.extra.get("title").and_then(Value::as_str),
            Some("Updated through DomainStore")
        );

        let selected_store = canonical_store_root(&selected, ".ticket");
        assert!(selected_store.join("tickets").join(id.to_string()).is_dir());
        for workspace in [
            parent_workspace.as_path(),
            sibling.as_path(),
            ambient.as_path(),
        ] {
            assert!(
                !canonical_store_root(workspace, ".ticket").exists(),
                "unexpected Ticket store created under {}",
                workspace.display()
            );
        }
    }

    #[test]
    fn read_only_resolution_reports_layouts_without_initializing_a_store() {
        let workspace = tempdir().unwrap();
        let legacy = workspace.path().join(".ticket");
        let canonical = canonical_store_root(workspace.path(), ".ticket");
        std::fs::create_dir_all(&legacy).unwrap();
        std::fs::create_dir_all(&canonical).unwrap();

        let resolution =
            TicketStore::resolve_workspace_store(workspace.path(), StoreAccessMode::ReadOnly)
                .unwrap();
        assert_eq!(resolution.domain.as_str(), "ticket");
        assert_eq!(resolution.store_root, canonical);
        assert_eq!(resolution.local_workspace, workspace.path());
        assert!(
            matches!(
                resolution.diagnostics.as_slice(),
                [memory_kernel::workspace::StoreRootDiagnostic::BothLayoutsPresent { .. }]
            ),
            "{:?}",
            resolution.diagnostics
        );

        let missing = tempdir().unwrap();
        assert!(matches!(
            TicketStore::open_in_workspace(missing.path()),
            Err(StorageError::WorkspaceNotFound { .. })
        ));
        assert!(!canonical_store_root(missing.path(), ".ticket").exists());

        let legacy_only = tempdir().unwrap();
        let legacy = legacy_only.path().join(".ticket");
        std::fs::create_dir_all(&legacy).unwrap();
        let resolution =
            TicketStore::resolve_workspace_store(legacy_only.path(), StoreAccessMode::ReadOnly)
                .unwrap();
        assert_eq!(resolution.store_root, legacy);
        assert!(matches!(
            resolution.diagnostics.as_slice(),
            [memory_kernel::workspace::StoreRootDiagnostic::LegacyStore { .. }]
        ));
    }

    #[test]
    fn legacy_ticket_store_remains_readable_while_new_writes_are_canonical() {
        let workspace = tempdir().unwrap();
        let canonical = canonical_store_root(workspace.path(), ".ticket");
        let mut store = TicketStore::open_or_init_in_workspace(workspace.path()).unwrap();
        let legacy_id = store
            .create_entity(TicketCreateInput {
                id: None,
                type_id: "tracker-improvement".to_string(),
                title: Some("Legacy read".to_string()),
                initial_state: None,
                extra: BTreeMap::new(),
                target_root: None,
                body: None,
            })
            .unwrap();
        drop(store);
        std::fs::rename(&canonical, workspace.path().join(".ticket")).unwrap();

        store = TicketStore::open_in_workspace(workspace.path()).unwrap();
        store.scan(true).unwrap();
        assert_eq!(
            store
                .read_entity(legacy_id)
                .unwrap()
                .extra
                .get("title")
                .and_then(Value::as_str),
            Some("Legacy read")
        );
        drop(store);

        let store = TicketStore::open_or_init_in_workspace(workspace.path()).unwrap();
        let canonical_id = store
            .create_entity(TicketCreateInput {
                id: None,
                type_id: "tracker-improvement".to_string(),
                title: Some("Canonical write".to_string()),
                initial_state: None,
                extra: BTreeMap::new(),
                target_root: None,
                body: None,
            })
            .unwrap();
        assert_eq!(
            store
                .read_entity(canonical_id)
                .unwrap()
                .extra
                .get("title")
                .and_then(Value::as_str),
            Some("Canonical write")
        );
        assert!(
            canonical
                .join("tickets")
                .join(canonical_id.to_string())
                .is_dir()
        );
        assert!(
            workspace
                .path()
                .join(".ticket")
                .join("tickets")
                .join(legacy_id.to_string())
                .is_dir()
        );
        assert!(
            !workspace
                .path()
                .join(".ticket")
                .join("tickets")
                .join(canonical_id.to_string())
                .exists()
        );
    }
}
