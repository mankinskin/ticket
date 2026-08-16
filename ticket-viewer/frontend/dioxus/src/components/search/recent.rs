pub(super) fn load_recent(workspace: &str) -> Vec<String> {
    let store = match local_storage() {
        Some(store) => store,
        None => return vec![],
    };
    let raw = match store.get_item(&recent_key(workspace)).ok().flatten() {
        Some(raw) => raw,
        None => return vec![],
    };
    serde_json::from_str::<Vec<String>>(&raw).unwrap_or_default()
}

pub(super) fn save_recent(
    workspace: &str,
    query: &str,
) {
    const MAX_RECENT: usize = 5;

    let store = match local_storage() {
        Some(store) => store,
        None => return,
    };
    let mut recents = load_recent(workspace);
    recents.retain(|recent| recent != query);
    recents.insert(0, query.to_string());
    recents.truncate(MAX_RECENT);
    if let Ok(json) = serde_json::to_string(&recents) {
        let _ = store.set_item(&recent_key(workspace), &json);
    }
}

fn local_storage() -> Option<web_sys::Storage> {
    web_sys::window()?.local_storage().ok().flatten()
}

fn recent_key(workspace: &str) -> String {
    format!("ticket-viewer:{workspace}:recent-searches")
}
