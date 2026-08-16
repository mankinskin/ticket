use std::collections::BTreeMap;

use crate::types::TicketSummary;

const DRAFT_KEY: &str = "draft_new_ticket";
pub(crate) const PRIORITY_OPTIONS: &[&str] =
    &["", "none", "low", "medium", "high", "critical"];

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub(crate) struct DraftState {
    pub type_id: String,
    pub title: String,
    pub description: String,
    pub priority: String,
    pub component: String,
    pub extra: BTreeMap<String, String>,
}

impl Default for DraftState {
    fn default() -> Self {
        Self {
            type_id: String::new(),
            title: String::new(),
            description: String::new(),
            priority: String::new(),
            component: String::new(),
            extra: BTreeMap::new(),
        }
    }
}

impl DraftState {
    pub(crate) fn from_prefill(prefill: &TicketSummary) -> Self {
        let fields = &prefill.fields;
        Self {
            type_id: fields
                .get("type")
                .and_then(|value| value.as_str())
                .unwrap_or("")
                .to_string(),
            title: prefill.title.clone().unwrap_or_default(),
            description: String::new(),
            priority: fields
                .get("priority")
                .and_then(|value| value.as_str())
                .unwrap_or("")
                .to_string(),
            component: fields
                .get("component")
                .and_then(|value| value.as_str())
                .unwrap_or("")
                .to_string(),
            extra: fields
                .as_object()
                .map(|object| {
                    object
                        .iter()
                        .filter(|(key, _)| !is_core_field(key))
                        .map(|(key, value)| {
                            (
                                key.clone(),
                                value.as_str().unwrap_or("").to_string(),
                            )
                        })
                        .collect()
                })
                .unwrap_or_default(),
        }
    }
}

pub(crate) fn save_draft(draft: &DraftState) {
    if let Some(store) = local_storage() {
        if let Ok(json) = serde_json::to_string(draft) {
            let _ = store.set_item(DRAFT_KEY, &json);
        }
    }
}

pub(crate) fn load_draft() -> Option<DraftState> {
    let store = local_storage()?;
    let raw = store.get_item(DRAFT_KEY).ok().flatten()?;
    serde_json::from_str(&raw).ok()
}

pub(crate) fn clear_draft() {
    if let Some(store) = local_storage() {
        let _ = store.remove_item(DRAFT_KEY);
    }
}

fn local_storage() -> Option<web_sys::Storage> {
    web_sys::window()?.local_storage().ok().flatten()
}

fn is_core_field(key: &str) -> bool {
    matches!(key, "type" | "priority" | "component" | "title" | "state")
}
