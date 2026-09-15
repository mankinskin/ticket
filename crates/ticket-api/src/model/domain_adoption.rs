//! Ticket-domain adapter boundary onto the generic entity kernel's domain
//! manifest contract (`memory_kernel::model::domain_manifest`).
//!
//! This module registers the ticket domain's *current* entity types and
//! schema state with the kernel's generic [`DomainManifest`] without
//! changing any ticket business field, state, transition, or history. Every
//! delivered ticket type schema (see [`builtin_schemas`]) is registered as
//! schema version 1, active — matching the fact that ticket-api has not yet
//! introduced per-type schema versioning of its own. Later adoption steps
//! may wire this manifest into migration, hooks, or workspace-capability
//! reporting; this step only proves the manifest boundary itself.

use memory_kernel::model::{
    domain::{DomainId, DomainSchemaVersion, EntityTypeId, EntityTypeSchemaVersion},
    domain_manifest::{
        DomainManifest, DomainManifestError, EntityTypeMembership, EntityTypeStatus,
    },
};

use super::default_schema::builtin_schemas;

/// The kernel [`DomainId`] under which ticket-api registers itself.
pub const TICKET_DOMAIN_ID: &str = "ticket";

/// The ticket domain's own [`DomainSchemaVersion`] as of this adoption step.
/// This tracks the domain manifest's own membership/activation set, not any
/// individual entity type's schema version (see [`EntityTypeMembership`]).
pub const TICKET_DOMAIN_SCHEMA_VERSION: u32 = 1;

/// Build the ticket domain's [`DomainManifest`] from the currently delivered
/// ticket type schemas ([`builtin_schemas`]). Every delivered type is
/// registered at schema version 1 and [`EntityTypeStatus::Active`], since
/// ticket-api has no existing per-type versioning or inactive-type concept
/// to preserve as of this adoption step.
pub fn ticket_domain_manifest() -> Result<DomainManifest, DomainManifestError> {
    let domain_id = DomainId::new(TICKET_DOMAIN_ID).expect("TICKET_DOMAIN_ID is non-empty");

    let entity_types = builtin_schemas()
        .into_iter()
        .map(|schema| EntityTypeMembership {
            entity_type_id: EntityTypeId::new(schema.type_id)
                .expect("delivered ticket schema type_id is non-empty"),
            schema_version: EntityTypeSchemaVersion(1),
            status: EntityTypeStatus::Active,
        })
        .collect();

    DomainManifest::new(
        domain_id,
        DomainSchemaVersion(TICKET_DOMAIN_SCHEMA_VERSION),
        entity_types,
    )
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;

    #[test]
    fn ticket_domain_manifest_registers_every_delivered_type_as_active() {
        let manifest = ticket_domain_manifest().expect("ticket domain manifest is valid");

        assert_eq!(manifest.domain_id.as_str(), TICKET_DOMAIN_ID);
        assert_eq!(
            manifest.schema_version,
            DomainSchemaVersion(TICKET_DOMAIN_SCHEMA_VERSION)
        );

        let delivered: BTreeSet<String> =
            builtin_schemas().into_iter().map(|s| s.type_id).collect();
        let registered: BTreeSet<String> = manifest
            .entity_types()
            .iter()
            .map(|m| m.entity_type_id.as_str().to_string())
            .collect();
        assert_eq!(delivered, registered);

        assert_eq!(manifest.active_entity_types().count(), delivered.len());
        assert_eq!(manifest.inactive_entity_types().count(), 0);
    }

    #[test]
    fn ticket_domain_manifest_membership_matches_schema_registry() {
        let manifest = ticket_domain_manifest().expect("ticket domain manifest is valid");

        for schema in builtin_schemas() {
            let entity_type_id = EntityTypeId::new(schema.type_id.clone()).unwrap();
            let membership = manifest.membership(&entity_type_id).unwrap_or_else(|| {
                panic!("{} missing from ticket domain manifest", schema.type_id)
            });
            assert_eq!(membership.schema_version, EntityTypeSchemaVersion(1));
            assert!(membership.status.is_active());
        }
    }
}
