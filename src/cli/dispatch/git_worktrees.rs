#[allow(unused_imports)]
use super::*;

pub(super) fn resolve_worktree_git_dir(
    workspace_root: &Path,
    git_path: &Path,
) -> Result<PathBuf, CliRunError> {
    let contents = fs::read_to_string(git_path).map_err(|error| {
        CliRunError::BadRequest(format!(
            "invalid board workspace '{}': expected a Git worktree .git file: {error}",
            workspace_root.display()
        ))
    })?;
    let git_dir = contents
        .strip_prefix("gitdir: ")
        .and_then(|value| value.lines().next())
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            CliRunError::BadRequest(format!(
                "invalid board workspace '{}': malformed Git worktree .git file",
                workspace_root.display()
            ))
        })?;
    let git_dir = Path::new(git_dir);
    let git_dir = if git_dir.is_absolute() {
        git_dir.to_path_buf()
    } else {
        workspace_root.join(git_dir)
    };
    canonical_board_workspace(&git_dir)
}

pub(super) fn resolve_main_checkout(
    workspace_root: &Path,
    git_dir: &Path,
) -> Result<PathBuf, CliRunError> {
    let common_dir = match fs::read_to_string(git_dir.join("commondir")) {
        Ok(contents) => contents,
        Err(error)
            if error.kind() == std::io::ErrorKind::NotFound
                && git_dir.join("HEAD").is_file()
                && git_dir.join("objects").is_dir()
                && git_dir.join("refs").is_dir()
                && !git_dir.join("gitdir").exists() =>
        {
            // Submodules and separate Git directories own their objects, unlike linked worktrees.
            return Ok(workspace_root.to_path_buf());
        }
        Err(error) => {
            return Err(CliRunError::BadRequest(format!(
                "invalid board workspace '{}': Git worktree has no commondir: {error}",
                workspace_root.display()
            )));
        }
    };
    let common_dir = common_dir
        .lines()
        .next()
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            CliRunError::BadRequest(format!(
                "invalid board workspace '{}': empty Git commondir",
                workspace_root.display()
            ))
        })?;
    let common_dir = Path::new(common_dir);
    let common_dir = if common_dir.is_absolute() {
        common_dir.to_path_buf()
    } else {
        git_dir.join(common_dir)
    };
    let common_dir = canonical_board_workspace(&common_dir)?;
    let main_checkout = common_dir.parent().filter(|parent| {
        common_dir.file_name().is_some_and(|name| name == ".git")
            && parent.join(".worktrees").is_dir()
    }).ok_or_else(|| {
        CliRunError::BadRequest(format!(
            "invalid board workspace '{}': Git common directory does not identify a managed main checkout",
            workspace_root.display()
        ))
    })?;
    let relative = workspace_root
        .strip_prefix(main_checkout.join(".worktrees"))
        .map_err(|_| {
            CliRunError::BadRequest(format!(
                "invalid board workspace '{}': not beneath the main checkout .worktrees directory",
                workspace_root.display()
            ))
        })?;
    if relative.components().count() != 2
        || relative
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        return Err(CliRunError::BadRequest(format!(
            "invalid board workspace '{}': expected .worktrees/<session>/<slug>",
            workspace_root.display()
        )));
    }
    Ok(main_checkout.to_path_buf())
}

pub(super) fn command_uses_descendant_scan_roots(command: &TicketCommandCli) -> bool {
    matches!(
        command,
        TicketCommandCli::Describe(_)
            | TicketCommandCli::List(_)
            | TicketCommandCli::Scan(_)
            | TicketCommandCli::Leases
            | TicketCommandCli::Search(_)
            | TicketCommandCli::Query(_)
            | TicketCommandCli::History(_)
            | TicketCommandCli::Diff(_)
            | TicketCommandCli::Links(_)
            | TicketCommandCli::PruneDangling(_)
            | TicketCommandCli::Subgraph(_)
            | TicketCommandCli::Topgraph(_)
            | TicketCommandCli::Status(_)
            | TicketCommandCli::ReadyOverview(_)
            | TicketCommandCli::Next(_)
            | TicketCommandCli::Blockers(_)
            | TicketCommandCli::UnblockedBy(_)
            | TicketCommandCli::Move(_)
            | TicketCommandCli::Assets(_)
            | TicketCommandCli::Transitions(_)
            | TicketCommandCli::Health(_)
            | TicketCommandCli::StoreIndex(_)
            | TicketCommandCli::Audit
            | TicketCommandCli::ValidateLinks
            | TicketCommandCli::ListParts(_)
            | TicketCommandCli::GetPart(_)
    )
}

pub(super) fn register_descendant_scan_roots(
    store: &TicketStore,
    workspace_root: &Path,
) -> Result<bool, CliRunError> {
    let mut known_scan_roots = store
        .list_scan_roots()?
        .into_iter()
        .map(|root| root.path)
        .collect::<BTreeSet<_>>();
    let mut reindex = false;

    for root in ticket_api::workspace::discover_workspace_scan_roots(
        workspace_root,
        ticket_api::workspace::TICKET_INDEX_DIR,
        "tickets",
    ) {
        if known_scan_roots.insert(root.path.clone()) {
            reindex = true;
        }
        store.add_scan_root(root)?;
    }

    Ok(reindex)
}
