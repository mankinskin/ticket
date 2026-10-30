#[allow(unused_imports)]
use super::*;

pub(super) fn resolve_index_root(
    override_path: Option<&Path>,
    workspace_root_override: Option<&Path>,
) -> Result<PathBuf, ticket_api::workspace::ConsumerWorkspaceError> {
    let cwd = ticket_api::workspace::working_dir();
    let env_root = std::env::var_os("TICKET_INDEX_ROOT").map(PathBuf::from);
    resolve_index_root_from(
        override_path,
        workspace_root_override,
        env_root.as_deref(),
        cwd.as_deref(),
    )
}

pub(super) fn resolve_index_root_from(
    override_path: Option<&Path>,
    workspace_root_override: Option<&Path>,
    env_root: Option<&Path>,
    cwd: Option<&Path>,
) -> Result<PathBuf, ticket_api::workspace::ConsumerWorkspaceError> {
    if let Some(override_path) = override_path {
        return Ok(absolute_path_from(override_path, cwd));
    }

    ticket_api::workspace::resolve_consumer_store_root_from(
        override_path,
        workspace_root_override,
        env_root,
        cwd,
        ticket_api::workspace::TICKET_INDEX_DIR,
    )
}

pub(super) fn absolute_path_from(path: &Path, cwd: Option<&Path>) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else if let Some(cwd) = cwd {
        cwd.join(path)
    } else {
        path.to_path_buf()
    }
}

pub(super) fn resolve_board_index_root(
    command: &TicketCommandCli,
    index_root_override: Option<&Path>,
    workspace_root_override: Option<&Path>,
) -> Result<PathBuf, CliRunError> {
    if !matches!(command, TicketCommandCli::Board(_)) || index_root_override.is_some() {
        return Ok(resolve_index_root(
            index_root_override,
            workspace_root_override,
        )?);
    }

    let Some(workspace_root) = workspace_root_override else {
        return Ok(resolve_index_root(None, None)?);
    };

    let workspace_root = canonical_board_workspace(workspace_root)?;
    let git_path = workspace_root.join(".git");
    if git_path.is_dir() {
        return Ok(workspace_root.join(ticket_api::workspace::TICKET_INDEX_DIR));
    }

    let git_dir = resolve_worktree_git_dir(&workspace_root, &git_path)?;
    let main_checkout = resolve_main_checkout(&workspace_root, &git_dir)?;
    Ok(main_checkout.join(ticket_api::workspace::TICKET_INDEX_DIR))
}

pub(super) fn canonical_board_workspace(path: &Path) -> Result<PathBuf, CliRunError> {
    ticket_api::workspace::canonicalize_workspace_root_strict(path).map_err(|error| {
        CliRunError::BadRequest(format!(
            "invalid board workspace '{}': {error}",
            path.display()
        ))
    })
}

pub(super) fn resolve_workspace_root(
    index_root: &Path,
    workspace_root_override: Option<&Path>,
) -> PathBuf {
    if let Some(path) = workspace_root_override {
        let store_root = ticket_api::workspace::resolve_store_root_at_fixed_workspace(
            path,
            ticket_api::workspace::TICKET_INDEX_DIR,
        );
        return ticket_api::workspace::resolve_workspace_root_from_store_root(
            &store_root,
            ticket_api::workspace::TICKET_INDEX_DIR,
        );
    }

    ticket_api::workspace::resolve_workspace_root_from_store_root(
        index_root,
        ticket_api::workspace::TICKET_INDEX_DIR,
    )
}

pub(super) fn open_store(
    local_workspace: &Path,
    schema_dir_override: Option<&Path>,
) -> Result<TicketStore, CliRunError> {
    let mut registry = SchemaRegistry::with_builtins();
    if let Some(schema_dir) = schema_dir_override {
        registry.load_dir(schema_dir)?;
    }
    TicketStore::open_or_init_in_workspace_with(local_workspace, registry)
        .map_err(CliRunError::from)
}
