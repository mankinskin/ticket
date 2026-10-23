use super::{integrity::public_fingerprint, reports::*};
use crate::{
    error::StorageError,
    model::filesystem::TICKET_PARTS_DIR,
    model::{
        parts::{is_planning_part_kind, part_kind_slug, part_slug_filename},
        ticket::{TicketManifestExt, TicketPart},
    },
    storage::ticket_fs::{PartLayoutSnapshot, sha256_bytes},
};
use std::{
    collections::{BTreeMap, BTreeSet, HashMap, HashSet},
    fs,
    path::Path,
};
use uuid::Uuid;

pub(super) fn build_ticket_plan(
    ticket_id: Uuid,
    title: Option<String>,
    ticket_path: String,
    snapshot: PartLayoutSnapshot,
) -> Result<PartLayoutTicketPlan, StorageError> {
    let mut conflicts = Vec::new();
    let parts = snapshot.manifest.parts();
    let file_by_part = snapshot
        .files
        .iter()
        .filter_map(|file| file.part_id.map(|id| (id, file)))
        .collect::<HashMap<_, _>>();
    let mut part_inventory = Vec::with_capacity(parts.len());
    let mut cleanup_candidates = Vec::new();
    let mut skipped_parts = Vec::new();
    let mut removed_ids = HashSet::new();

    let mut seen_paths = HashSet::new();
    for (index, part) in parts.iter().enumerate() {
        if !seen_paths.insert(part.path.clone()) {
            conflicts.push(format!(
                "multiple manifest parts reference the same path: {}",
                part.path
            ));
        }
        let file = file_by_part.get(&part.id).copied();
        let (size, sha256) = file
            .map(|file| (file.size, file.sha256.clone()))
            .unwrap_or((None, None));
        if size.is_none() || sha256.is_none() {
            conflicts.push(format!(
                "manifest part {} has no readable backing file at {}",
                part.id, part.path
            ));
        }
        let inventory = PartLayoutPart {
            part: part.clone(),
            original_index: index,
            size,
            sha256: sha256.clone(),
        };

        if is_planning_part_kind(&part.kind) && part.frozen && size == Some(0) {
            let Some(sha256) = sha256 else {
                conflicts.push(format!(
                    "eligible empty planning part {} has no SHA-256",
                    part.id
                ));
                part_inventory.push(inventory);
                continue;
            };
            removed_ids.insert(part.id);
            cleanup_candidates.push(PartLayoutCleanupCandidate {
                part: part.clone(),
                original_index: index,
                size: 0,
                sha256,
            });
        } else {
            skipped_parts.push(PartLayoutSkip {
                part_id: part.id,
                path: part.path.clone(),
                reason: skip_reason(part, size),
            });
        }
        part_inventory.push(inventory);
    }

    let retained = parts
        .iter()
        .filter(|part| !removed_ids.contains(&part.id))
        .collect::<Vec<_>>();
    let mut ordinal_by_base = BTreeMap::<String, usize>::new();
    let mut ids_by_base = BTreeMap::<String, Vec<(Uuid, String)>>::new();
    let mut desired_paths = HashMap::<Uuid, String>::new();
    for part in &retained {
        let base = part_kind_slug(&part.kind);
        let ordinal = ordinal_by_base.entry(base.clone()).or_default();
        *ordinal += 1;
        let target = format!(
            "{TICKET_PARTS_DIR}/{}",
            part_slug_filename(&part.kind, *ordinal)
        );
        desired_paths.insert(part.id, target);
        ids_by_base
            .entry(base)
            .or_default()
            .push((part.id, part.kind.clone()));
    }

    let normalized_base_collisions = ids_by_base
        .into_iter()
        .filter_map(|(normalized_base, owners)| {
            (owners.len() > 1).then(|| PartLayoutBaseCollision {
                normalized_base,
                part_ids: owners.iter().map(|(id, _)| *id).collect(),
                kinds: owners
                    .iter()
                    .map(|(_, kind)| kind.clone())
                    .collect::<BTreeSet<_>>()
                    .into_iter()
                    .collect(),
            })
        })
        .collect::<Vec<_>>();

    let mut path_changes = Vec::new();
    let mut desired_owner = HashMap::<String, Uuid>::new();
    for part in &retained {
        let target = desired_paths
            .get(&part.id)
            .ok_or_else(|| StorageError::Other(format!("missing desired path for {}", part.id)))?;
        if let Some(existing) = desired_owner.insert(target.clone(), part.id) {
            conflicts.push(format!(
                "normalized part paths collide for tickets parts {existing} and {} at {target}",
                part.id
            ));
        }
        if part.path != *target {
            let inventory = part_inventory
                .iter()
                .find(|entry| entry.part.id == part.id)
                .ok_or_else(|| {
                    StorageError::Other(format!("missing part inventory {}", part.id))
                })?;
            let (Some(size), Some(sha256)) = (inventory.size, inventory.sha256.clone()) else {
                continue;
            };
            path_changes.push(PartLayoutPathChange {
                part_id: part.id,
                from: part.path.clone(),
                to: target.clone(),
                size,
                sha256,
            });
        }
    }

    let changing_sources = path_changes
        .iter()
        .map(|change| change.from.clone())
        .chain(
            cleanup_candidates
                .iter()
                .map(|candidate| candidate.part.path.clone()),
        )
        .collect::<HashSet<_>>();
    let orphan_paths = snapshot
        .files
        .iter()
        .filter(|file| file.orphan)
        .map(|file| file.path.as_str())
        .collect::<HashSet<_>>();
    let current_part_paths = parts
        .iter()
        .map(|part| part.path.as_str())
        .collect::<HashSet<_>>();
    let mut expected_absent_paths = BTreeSet::new();
    for change in &path_changes {
        if orphan_paths.contains(change.to.as_str()) {
            conflicts.push(format!(
                "foreign orphan occupies desired path {}",
                change.to
            ));
            continue;
        }
        if current_part_paths.contains(change.to.as_str()) {
            if cleanup_candidates
                .iter()
                .any(|candidate| candidate.part.path == change.to)
            {
                conflicts.push(format!(
                    "desired path {} is currently owned by an eligible cleanup candidate",
                    change.to
                ));
            } else if !changing_sources.contains(&change.to) {
                conflicts.push(format!(
                    "foreign manifest part occupies desired path {}",
                    change.to
                ));
            }
            continue;
        }

        let indexed_path = Path::new(&ticket_path).join(&change.to);
        match fs::symlink_metadata(&indexed_path) {
            Ok(_) => conflicts.push(format!("foreign file occupies desired path {}", change.to)),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                expected_absent_paths.insert(change.to.clone());
            }
            Err(error) => conflicts.push(format!(
                "cannot inspect desired path {}: {error}",
                change.to
            )),
        }
    }

    let mut parts_after = parts
        .iter()
        .filter(|part| !removed_ids.contains(&part.id))
        .cloned()
        .collect::<Vec<_>>();
    for part in &mut parts_after {
        if let Some(path) = desired_paths.get(&part.id) {
            part.path = path.clone();
        }
    }
    let mut manifest_after = snapshot.manifest.clone();
    manifest_after.set_parts(parts_after.clone());

    let has_changes = !path_changes.is_empty() || !cleanup_candidates.is_empty();
    let status = if !conflicts.is_empty() {
        PartLayoutTicketStatus::Conflict
    } else if has_changes {
        PartLayoutTicketStatus::Planned
    } else {
        PartLayoutTicketStatus::Unchanged
    };
    let manifest_after =
        has_changes.then(|| crate::model::manifest_format::format_manifest_toml(&manifest_after));
    let manifest_after_sha256 = manifest_after
        .as_deref()
        .map(|manifest| sha256_bytes(manifest.as_bytes()));
    let orphan_files = snapshot
        .files
        .iter()
        .filter(|file| file.orphan)
        .map(public_fingerprint)
        .collect();
    let preservation = snapshot.files.iter().map(public_fingerprint).collect();

    Ok(PartLayoutTicketPlan {
        id: ticket_id,
        title,
        ticket_path,
        status,
        manifest_before: snapshot.manifest_contents,
        manifest_sha256: snapshot.manifest_sha256,
        manifest_after,
        manifest_after_sha256,
        parts: part_inventory,
        parts_after,
        orphan_files,
        preservation,
        path_changes,
        cleanup_candidates,
        normalized_base_collisions,
        skipped_parts,
        conflicts,
        expected_absent_paths: expected_absent_paths.into_iter().collect(),
    })
}

pub(super) fn skip_reason(part: &TicketPart, size: Option<u64>) -> String {
    if !is_planning_part_kind(&part.kind) {
        "part kind is not a planning kind".to_string()
    } else if !part.frozen {
        "planning part is not frozen".to_string()
    } else if size.is_some_and(|size| size > 0) {
        "backing file is non-empty".to_string()
    } else {
        "part does not meet the approved cleanup predicate".to_string()
    }
}
