use std::{
    collections::{BTreeMap, BTreeSet, HashMap, HashSet},
    fs,
    path::Path,
};

use uuid::Uuid;

use crate::{
    error::StorageError,
    model::filesystem::TICKET_PARTS_DIR,
    model::{
        parts::{is_planning_part_kind, part_kind_slug, part_slug_filename},
        ticket::TicketManifestExt,
    },
    storage::ticket_fs::{PartFileFingerprint, sha256_bytes},
};

use super::{planning::skip_reason, reports::*};

pub(super) fn validate_ticket_plan_integrity(plan: &PartLayoutTicketPlan) -> Result<(), String> {
    if plan.status != PartLayoutTicketStatus::Planned || !plan.conflicts.is_empty() {
        return Err("completed report contains a non-planned or conflicting ticket".to_string());
    }
    if sha256_bytes(plan.manifest_before.as_bytes()) != plan.manifest_sha256 {
        return Err("original manifest fingerprint does not match report".to_string());
    }
    let manifest_path = Path::new(&plan.ticket_path).join("ticket.toml");
    let before_manifest = crate::model::filesystem::parse_ticket_manifest_toml(
        manifest_path.clone(),
        &plan.manifest_before,
    )
    .map_err(|diagnostic| diagnostic.reason)?;
    if before_manifest.id != plan.id {
        return Err("original manifest ticket id does not match report".to_string());
    }

    let original_parts = before_manifest.parts();
    let reported_parts = plan
        .parts
        .iter()
        .map(|part| part.part.clone())
        .collect::<Vec<_>>();
    if original_parts != reported_parts
        || plan
            .parts
            .iter()
            .enumerate()
            .any(|(index, part)| part.original_index != index)
    {
        return Err("original part metadata does not match report".to_string());
    }

    let mut removed_ids = HashSet::new();
    let mut expected_cleanup = Vec::new();
    let mut expected_skips = Vec::new();
    for inventory in &plan.parts {
        if is_planning_part_kind(&inventory.part.kind)
            && inventory.part.frozen
            && inventory.size == Some(0)
        {
            let empty_hash = sha256_bytes(b"");
            if inventory.sha256.as_deref() != Some(empty_hash.as_str()) {
                return Err(format!(
                    "eligible empty part {} has an invalid fingerprint",
                    inventory.part.id
                ));
            }
            removed_ids.insert(inventory.part.id);
            expected_cleanup.push(PartLayoutCleanupCandidate {
                part: inventory.part.clone(),
                original_index: inventory.original_index,
                size: 0,
                sha256: empty_hash,
            });
        } else {
            expected_skips.push(PartLayoutSkip {
                part_id: inventory.part.id,
                path: inventory.part.path.clone(),
                reason: skip_reason(&inventory.part, inventory.size),
            });
        }
    }
    if expected_cleanup != plan.cleanup_candidates || expected_skips != plan.skipped_parts {
        return Err("cleanup eligibility or skip records do not match part metadata".to_string());
    }

    let mut ordinals = BTreeMap::<String, usize>::new();
    let mut expected_paths = HashMap::<Uuid, String>::new();
    let mut by_base = BTreeMap::<String, Vec<(Uuid, String)>>::new();
    for part in original_parts
        .iter()
        .filter(|part| !removed_ids.contains(&part.id))
    {
        let base = part_kind_slug(&part.kind);
        let ordinal = ordinals.entry(base.clone()).or_default();
        *ordinal += 1;
        expected_paths.insert(
            part.id,
            format!(
                "{TICKET_PARTS_DIR}/{}",
                part_slug_filename(&part.kind, *ordinal)
            ),
        );
        by_base
            .entry(base)
            .or_default()
            .push((part.id, part.kind.clone()));
    }
    let expected_collisions = by_base
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
    if expected_collisions != plan.normalized_base_collisions {
        return Err("normalized-base collision records do not match part kinds".to_string());
    }

    let mut expected_after_parts = original_parts
        .iter()
        .filter(|part| !removed_ids.contains(&part.id))
        .cloned()
        .collect::<Vec<_>>();
    for part in &mut expected_after_parts {
        part.path = expected_paths
            .get(&part.id)
            .ok_or_else(|| format!("missing normalized path for part {}", part.id))?
            .clone();
    }
    if expected_after_parts != plan.parts_after {
        return Err("post-migration part metadata does not match normalized paths".to_string());
    }

    let mut expected_changes = Vec::new();
    for part in original_parts
        .iter()
        .filter(|part| !removed_ids.contains(&part.id))
    {
        let target = expected_paths
            .get(&part.id)
            .ok_or_else(|| format!("missing normalized path for part {}", part.id))?;
        if &part.path != target {
            let inventory = plan
                .parts
                .iter()
                .find(|inventory| inventory.part.id == part.id)
                .ok_or_else(|| format!("missing inventory for part {}", part.id))?;
            let (Some(size), Some(sha256)) = (inventory.size, inventory.sha256.clone()) else {
                return Err(format!(
                    "retained part {} is missing its backing file fingerprint",
                    part.id
                ));
            };
            expected_changes.push(PartLayoutPathChange {
                part_id: part.id,
                from: part.path.clone(),
                to: target.clone(),
                size,
                sha256,
            });
        }
    }
    if expected_changes != plan.path_changes {
        return Err("path-change list does not match normalized part paths".to_string());
    }

    let mut expected_preservation = plan
        .parts
        .iter()
        .map(|part| PartLayoutFileFingerprint {
            path: part.part.path.clone(),
            size: part.size,
            sha256: part.sha256.clone(),
            part_id: Some(part.part.id),
            orphan: false,
        })
        .collect::<Vec<_>>();
    expected_preservation.extend(plan.orphan_files.iter().cloned());
    if !public_fingerprints_match(&expected_preservation, &plan.preservation) {
        return Err("preservation inventory does not match manifest parts and orphans".to_string());
    }

    let manifest_after = plan
        .manifest_after
        .as_deref()
        .ok_or_else(|| "post-migration manifest is missing".to_string())?;
    if plan.manifest_after_sha256.as_deref()
        != Some(sha256_bytes(manifest_after.as_bytes()).as_str())
    {
        return Err("post-migration manifest fingerprint does not match report".to_string());
    }
    let mut expected_manifest = before_manifest;
    expected_manifest.set_parts(expected_after_parts);
    let serialized_after = crate::model::manifest_format::format_manifest_toml(&expected_manifest);
    if serialized_after != manifest_after {
        return Err("post-migration manifest contains changes beyond the layout plan".to_string());
    }
    let parsed_after =
        crate::model::filesystem::parse_ticket_manifest_toml(manifest_path, manifest_after)
            .map_err(|diagnostic| diagnostic.reason)?;
    if parsed_after.parts() != plan.parts_after {
        return Err("post-migration manifest parts do not match report".to_string());
    }
    Ok(())
}

pub(super) fn public_fingerprint(file: &PartFileFingerprint) -> PartLayoutFileFingerprint {
    PartLayoutFileFingerprint {
        path: file.path.clone(),
        size: file.size,
        sha256: file.sha256.clone(),
        part_id: file.part_id,
        orphan: file.orphan,
    }
}

pub(super) fn internal_fingerprints(
    files: &[PartLayoutFileFingerprint],
) -> Vec<PartFileFingerprint> {
    files
        .iter()
        .map(|file| PartFileFingerprint {
            path: file.path.clone(),
            size: file.size,
            sha256: file.sha256.clone(),
            part_id: file.part_id,
            orphan: file.orphan,
        })
        .collect()
}

pub(super) fn expected_after_fingerprints(plan: &PartLayoutTicketPlan) -> Vec<PartFileFingerprint> {
    let changes = plan
        .path_changes
        .iter()
        .map(|change| (change.part_id, change.to.as_str()))
        .collect::<HashMap<_, _>>();
    let removed = plan
        .cleanup_candidates
        .iter()
        .map(|candidate| candidate.part.id)
        .collect::<HashSet<_>>();
    let mut expected = Vec::new();
    for part in &plan.parts {
        if removed.contains(&part.part.id) {
            continue;
        }
        expected.push(PartFileFingerprint {
            path: changes
                .get(&part.part.id)
                .map_or(part.part.path.as_str(), |path| path)
                .to_string(),
            size: part.size,
            sha256: part.sha256.clone(),
            part_id: Some(part.part.id),
            orphan: false,
        });
    }
    expected.extend(internal_fingerprints(&plan.orphan_files));
    expected
}

pub(super) fn expected_absent_after_paths(plan: &PartLayoutTicketPlan) -> Vec<String> {
    let after_paths = plan
        .parts_after
        .iter()
        .map(|part| part.path.as_str())
        .chain(plan.orphan_files.iter().map(|file| file.path.as_str()))
        .collect::<HashSet<_>>();
    plan.path_changes
        .iter()
        .map(|change| change.from.as_str())
        .chain(
            plan.cleanup_candidates
                .iter()
                .map(|candidate| candidate.part.path.as_str()),
        )
        .filter(|path| !after_paths.contains(path))
        .map(str::to_string)
        .collect()
}

pub(super) fn fingerprints_match(
    actual: &[PartFileFingerprint],
    expected: &[PartFileFingerprint],
) -> bool {
    let key = |file: &PartFileFingerprint| {
        (
            file.path.clone(),
            file.part_id,
            file.orphan,
            file.size,
            file.sha256.clone(),
        )
    };
    let mut actual = actual.iter().map(key).collect::<Vec<_>>();
    let mut expected = expected.iter().map(key).collect::<Vec<_>>();
    actual.sort();
    expected.sort();
    actual == expected
}

pub(super) fn canonical_path(path: &Path) -> Result<String, StorageError> {
    Ok(fs::canonicalize(path)?.to_string_lossy().replace('\\', "/"))
}

pub(super) fn ticket_path_is_within_store(ticket_path: &str, tickets_root: &str) -> bool {
    let prefix = format!("{}/", tickets_root.trim_end_matches('/'));
    ticket_path.starts_with(&prefix)
}
