#[allow(unused_imports)]
use super::*;

pub(super) fn resolve_reference_owner_root(
    aggregate_root: &Path,
    workspace: &str,
) -> Result<PathBuf, String> {
    let aggregate_root = aggregate_root
        .canonicalize()
        .map_err(|error| format!("cannot resolve aggregate root: {error}"))?;
    if workspace == "default" {
        return Ok(aggregate_root);
    }

    let workspace_path = Path::new(workspace);
    if workspace_path.is_absolute()
        || workspace_path
            .components()
            .any(|component| !matches!(component, std::path::Component::Normal(_)))
    {
        return Err(format!(
            "workspace '{workspace}' must be a relative path beneath the aggregate root"
        ));
    }

    let owner_root = aggregate_root
        .join(workspace_path)
        .canonicalize()
        .map_err(|error| format!("cannot resolve owner workspace '{workspace}': {error}"))?;
    if !owner_root.starts_with(&aggregate_root) {
        return Err(format!(
            "workspace '{workspace}' resolves outside the aggregate root"
        ));
    }

    Ok(owner_root)
}

pub(super) fn resolve_referenced_spec_root(
    owner_root: &Path,
    store_root: &str,
) -> Result<PathBuf, String> {
    let candidate = Path::new(store_root);
    if candidate.is_absolute() {
        return Ok(candidate.to_path_buf());
    }
    if candidate
        .components()
        .any(|component| !matches!(component, std::path::Component::Normal(_)))
    {
        return Err(format!(
            "relative store_root '{store_root}' must remain beneath its owner workspace"
        ));
    }

    Ok(owner_root.join(candidate))
}

/// Attempt to read a spec at `root` by id. Returns `None` when the store
/// does not exist there or the id is not found.
pub(super) fn try_get_spec(root: &Path, spec_id: Uuid) -> Option<spec_api::SpecManifest> {
    let store = spec_api::SpecStore::open(root).ok()?;
    store.get(&spec_id.to_string()).ok()
}

pub(super) fn count_links_by_kind(findings: &[Value]) -> Value {
    let mut counts = serde_json::Map::new();
    for finding in findings {
        let kind = finding
            .get("kind")
            .and_then(|k| k.as_str())
            .unwrap_or("unknown")
            .to_string();
        let entry = counts.entry(kind).or_insert(json!(0));
        if let Some(n) = entry.as_u64() {
            *entry = json!(n + 1);
        }
    }
    Value::Object(counts)
}

/// Validate `related_specs` links: detect dangling spec refs, wrong-store
/// refs, and bidirectional inconsistencies against the referenced spec
/// store(s). Mirrors `spec validate-links` from the other direction.
pub(crate) fn cmd_validate_links(
    store: &TicketStore,
    aggregate_root: &Path,
) -> Result<Value, CliRunError> {
    let workspace_root = workspace::resolve_workspace_root_from_store_root(
        &store.index_root,
        workspace::TICKET_INDEX_DIR,
    );
    let all = store.list(None, None, None)?;
    let mut findings: Vec<Value> = Vec::new();
    let mut checked = 0usize;

    for indexed in &all {
        let ticket = match store.get(&indexed.id) {
            Ok(ticket) => ticket,
            Err(_) => continue,
        };

        for spec_ref in ticket.related_specs() {
            checked += 1;
            let owner_root = match resolve_reference_owner_root(aggregate_root, &spec_ref.workspace)
            {
                Ok(owner_root) => owner_root,
                Err(message) => {
                    findings.push(json!({
                        "kind": "invalid_owner_workspace",
                        "ticket_id": ticket.id,
                        "spec_id": spec_ref.spec_id,
                        "workspace": spec_ref.workspace,
                        "store_root": spec_ref.store_root,
                        "message": message,
                    }));
                    continue;
                }
            };
            let referenced_root =
                match resolve_referenced_spec_root(&owner_root, &spec_ref.store_root) {
                    Ok(referenced_root) => referenced_root,
                    Err(message) => {
                        findings.push(json!({
                            "kind": "invalid_store_root",
                            "ticket_id": ticket.id,
                            "spec_id": spec_ref.spec_id,
                            "workspace": spec_ref.workspace,
                            "store_root": spec_ref.store_root,
                            "message": message,
                        }));
                        continue;
                    }
                };
            let canonical_spec_root =
                memory_kernel::workspace::canonical_store_root(&owner_root, ".spec");

            if let Some(spec) = try_get_spec(&referenced_root, spec_ref.spec_id) {
                let has_back_ref = spec
                    .related_tickets()
                    .iter()
                    .any(|ticket_ref| ticket_ref.ticket_id == ticket.id);
                if !has_back_ref {
                    findings.push(json!({
                        "kind": "bidirectional_inconsistency",
                        "ticket_id": ticket.id,
                        "spec_id": spec_ref.spec_id,
                        "workspace": spec_ref.workspace,
                        "store_root": spec_ref.store_root,
                        "message": format!(
                            "ticket {} links spec {} but the spec's related_tickets does not link back",
                            ticket.id, spec_ref.spec_id,
                        ),
                    }));
                }
                continue;
            }

            if referenced_root != canonical_spec_root
                && try_get_spec(&canonical_spec_root, spec_ref.spec_id).is_some()
            {
                findings.push(json!({
                    "kind": "wrong_store_ref",
                    "ticket_id": ticket.id,
                    "spec_id": spec_ref.spec_id,
                    "workspace": spec_ref.workspace,
                    "store_root": spec_ref.store_root,
                    "message": format!(
                        "spec {} exists but not under store_root '{}'; found under the workspace's canonical .spec store instead",
                        spec_ref.spec_id, spec_ref.store_root,
                    ),
                }));
                continue;
            }

            findings.push(json!({
                "kind": "dangling_spec_ref",
                "ticket_id": ticket.id,
                "spec_id": spec_ref.spec_id,
                "workspace": spec_ref.workspace,
                "store_root": spec_ref.store_root,
                "message": format!(
                    "spec {} not found under store_root '{}'",
                    spec_ref.spec_id, spec_ref.store_root,
                ),
            }));
        }
    }

    let counts = count_links_by_kind(&findings);

    Ok(json!({
        "command": "validate_links",
        "status": "ok",
        "workspace_root": workspace_root.display().to_string(),
        "checked": checked,
        "valid": findings.is_empty(),
        "counts": counts,
        "findings": findings,
    }))
}
