#[allow(unused_imports)]
use super::*;

pub(crate) fn cmd_link(args: LinkArgs, store: &TicketStore) -> Result<Value, CliRunError> {
    let from = resolve_uuid_prefix(&args.from, store)?;
    let to = resolve_uuid_prefix(&args.to, store)?;
    let from_title = store
        .get(&from)
        .ok()
        .and_then(|m| {
            m.extra
                .get("title")
                .and_then(Value::as_str)
                .map(String::from)
        })
        .unwrap_or_else(|| from.to_string());
    let to_title = store
        .get(&to)
        .ok()
        .and_then(|m| {
            m.extra
                .get("title")
                .and_then(Value::as_str)
                .map(String::from)
        })
        .unwrap_or_else(|| to.to_string());
    let edge = EdgeRecord {
        from,
        to,
        kind: args.kind.clone(),
        created_at: Utc::now(),
    };
    store.add_edge(edge)?;
    Ok(json!({
        "command": "link",
        "status": "ok",
        "from": from,
        "from_title": from_title,
        "to": to,
        "to_title": to_title,
        "kind": args.kind,
        "reason": args.reason,
    }))
}

pub(crate) fn cmd_unlink(args: UnlinkArgs, store: &TicketStore) -> Result<Value, CliRunError> {
    let from = resolve_unlink_from(&args.from, store)?;
    let to = resolve_unlink_to(from, &args.to, &args.kind, store)?;
    let from_title = store
        .get(&from)
        .ok()
        .and_then(|m| {
            m.extra
                .get("title")
                .and_then(Value::as_str)
                .map(String::from)
        })
        .unwrap_or_else(|| from.to_string());
    let to_title = store
        .get(&to)
        .ok()
        .and_then(|m| {
            m.extra
                .get("title")
                .and_then(Value::as_str)
                .map(String::from)
        })
        .unwrap_or_else(|| to.to_string());
    let edge = EdgeRecord {
        from,
        to,
        kind: args.kind.clone(),
        created_at: Utc::now(),
    };
    store.remove_edge(edge)?;
    Ok(json!({
        "command": "unlink",
        "status": "ok",
        "from": from,
        "from_title": from_title,
        "to": to,
        "to_title": to_title,
        "kind": args.kind,
        "reason": args.reason,
    }))
}

pub(super) fn resolve_unlink_from(
    selector: &str,
    store: &TicketStore,
) -> Result<Uuid, CliRunError> {
    let trimmed = selector.trim();
    if let Ok(id) = trimmed.parse::<Uuid>() {
        return Ok(id);
    }
    if trimmed.len() >= 8 && trimmed.chars().all(|ch| ch.is_ascii_hexdigit()) {
        return resolve_uuid_prefix(trimmed, store).map_err(|_| {
            CliRunError::BadRequest(format!(
                "cannot resolve source prefix '{trimmed}'; provide full UUID when source ticket is missing"
            ))
        });
    }

    Err(CliRunError::BadRequest(format!(
        "invalid UUID '{selector}': expected full UUID or hex prefix (>= 8 chars)"
    )))
}

pub(super) fn resolve_unlink_to(
    from: Uuid,
    selector: &str,
    kind: &str,
    store: &TicketStore,
) -> Result<Uuid, CliRunError> {
    let trimmed = selector.trim();
    if let Ok(parsed) = trimmed.parse::<Uuid>() {
        return Ok(parsed);
    }

    if !(trimmed.len() >= 8 && trimmed.chars().all(|ch| ch.is_ascii_hexdigit())) {
        return Err(CliRunError::BadRequest(format!(
            "invalid UUID '{selector}': expected full UUID or hex prefix (>= 8 chars)"
        )));
    }

    let prefix = trimmed.to_ascii_lowercase();
    let matches: Vec<Uuid> = store
        .edges_from(&from)?
        .into_iter()
        .filter(|edge| edge.kind == kind)
        .map(|edge| edge.to)
        .filter(|to| to.simple().to_string().starts_with(&prefix))
        .collect();

    match matches.len() {
        0 => Err(CliRunError::BadRequest(format!(
            "edge not found: kind='{kind}' from='{from}' to='{selector}'"
        ))),
        1 => Ok(matches[0]),
        count => Err(CliRunError::BadRequest(format!(
            "ambiguous target selector '{selector}' for from='{from}' kind='{kind}' (matches {count}); use full UUID"
        ))),
    }
}

pub(crate) fn cmd_links(args: LinksArgs, store: &TicketStore) -> Result<Value, CliRunError> {
    let raw_edges = if args.all {
        store.list_all_edges()?
    } else {
        let id_str = args
            .id
            .as_ref()
            .expect("clap ensures id is present when --all is not set");
        let id = resolve_uuid_prefix(id_str, store)?;
        store.edges_from(&id)?
    };

    let items: Vec<Value> = raw_edges
        .iter()
        .filter(|e| match &args.kind {
            Some(k) => e.kind == *k,
            None => true,
        })
        .map(|e| json!({ "from": e.from, "to": e.to, "kind": e.kind }))
        .collect();
    Ok(json!({
        "command": "links",
        "status": "ok",
        "id": args.id,
        "all": args.all,
        "count": items.len(),
        "edges": items,
    }))
}

pub(crate) fn cmd_prune_dangling(
    args: PruneDanglingArgs,
    store: &TicketStore,
) -> Result<Value, CliRunError> {
    let strategy = args.strategy;
    let all_edges = store.list_all_edges()?;

    let root = if args.all {
        None
    } else {
        let root_str = args.root.as_deref().ok_or_else(|| {
            CliRunError::BadRequest("root ticket id is required unless --all is set".to_string())
        })?;
        Some(resolve_uuid_prefix(root_str, store)?)
    };

    let mut candidates = Vec::new();
    for edge in all_edges {
        if edge.kind != args.kind {
            continue;
        }
        if let Some(root_id) = root {
            if edge.from != root_id {
                continue;
            }
        }
        if !ticket_exists(store, edge.to) {
            candidates.push(edge);
        }
    }

    let mut removed = 0usize;
    if strategy.mutates() {
        for edge in &candidates {
            store.remove_edge(edge.clone())?;
            removed += 1;
        }
    }

    let preview: Vec<Value> = candidates
        .iter()
        .map(|edge| {
            json!({
                "from": edge.from,
                "to": edge.to,
                "kind": edge.kind,
            })
        })
        .collect();

    let scope = if args.all {
        json!({ "all": true })
    } else {
        json!({
            "all": false,
            "root": root,
        })
    };

    Ok(json!({
        "command": "prune-dangling",
        "status": "ok",
        "scope": scope,
        "kind": args.kind,
        "strategy": strategy.as_str(),
        "mutated": strategy.mutates(),
        "candidate_count": preview.len(),
        "removed_count": removed,
        "reason": args.reason,
        "edges": preview,
    }))
}
