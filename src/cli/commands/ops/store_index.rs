#[allow(unused_imports)]
use super::*;

pub(crate) fn cmd_store_index(
    args: StoreIndexArgs,
    store: &TicketStore,
) -> Result<Value, CliRunError> {
    let workspace_root = workspace::resolve_workspace_root_from_store_root(
        &store.index_root,
        workspace::TICKET_INDEX_DIR,
    );

    let indexed = store.list(None, None, None)?;
    let mut sources = Vec::with_capacity(indexed.len());

    for ticket in indexed {
        let manifest = TicketFs::read(&ticket.path)?;
        let description = TicketFs::read_description(&ticket.path)?.unwrap_or_default();
        let source_path = memory_kernel::index_generator::to_relative_slash(
            &workspace_root,
            &ticket.path.join("ticket.toml"),
        );

        let title = manifest
            .extra
            .get("title")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
        let state = manifest
            .extra
            .get("state")
            .and_then(Value::as_str)
            .unwrap_or("unknown")
            .to_string();
        let priority = manifest
            .extra
            .get("priority")
            .and_then(Value::as_str)
            .map(str::to_string);
        let component = manifest
            .extra
            .get("component")
            .and_then(Value::as_str)
            .map(str::to_string);

        sources.push(TicketCatalogSource {
            id: ticket.id,
            source_path,
            title,
            state,
            priority,
            component,
            description,
        });
    }

    let artifacts = generate_ticket_catalog(&sources, STORE_DIR);

    let readme_path = workspace_root.join(STORE_DIR).join("README.md");
    let sidecar_path = workspace_root.join(STORE_DIR).join("index.toon");
    let agent_hook_path = workspace_root.join(TICKET_INDEX_AGENT_HOOK_PATH);

    let sidecar_toon = artifacts
        .sidecar
        .encode_toon()
        .map_err(|e| CliRunError::BadRequest(e.to_string()))?;

    let readme_out = memory_kernel::generated_markdown::prepare_generated_output(
        &artifacts.readme_markdown,
        read_existing(&readme_path).as_deref(),
    );
    let sidecar_out = memory_kernel::generated_markdown::prepare_generated_output(
        &sidecar_toon,
        read_existing(&sidecar_path).as_deref(),
    );
    let agent_hook_out = memory_kernel::generated_markdown::prepare_generated_output(
        &artifacts.agent_hook_markdown,
        read_existing(&agent_hook_path).as_deref(),
    );

    let planned = [
        (&readme_path, &readme_out),
        (&sidecar_path, &sidecar_out),
        (&agent_hook_path, &agent_hook_out),
    ];

    if args.check {
        let drifted: Vec<String> = planned
            .iter()
            .filter(|(path, content)| read_existing(path).as_deref() != Some(content.as_str()))
            .map(|(path, _)| display_path(path))
            .collect();

        if !drifted.is_empty() {
            return Err(CliRunError::BadRequest(format!(
                "ticket store-index is out of date; regenerate and re-stage: {}",
                drifted.join(", ")
            )));
        }

        return Ok(json!({
            "command": "store-index",
            "status": "ok",
            "check": true,
            "drift": false,
            "tickets": sources.len(),
        }));
    }

    let mut written = Vec::new();
    for (path, content) in planned {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(StorageError::Io)?;
        }
        fs::write(path, content).map_err(StorageError::Io)?;
        written.push(display_path(path));
    }

    Ok(json!({
        "command": "store-index",
        "status": "ok",
        "check": false,
        "tickets": sources.len(),
        "written": written,
    }))
}

pub(super) fn read_existing(path: &Path) -> Option<String> {
    fs::read_to_string(path).ok()
}

pub(super) fn display_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}
