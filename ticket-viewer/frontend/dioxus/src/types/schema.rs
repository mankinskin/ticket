#[allow(unused_imports)]
use super::*;

// ── Schema ────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct FieldDef {
    pub field_type: String,
    pub required: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TransitionDef {
    pub from: String,
    pub to: String,
}

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct EdgeRuleDef {
    pub directed: bool,
    pub acyclic_enforced: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct TypeSchema {
    pub type_id: String,
    pub states: Vec<String>,
    pub transitions: Vec<TransitionDef>,
    pub fields: std::collections::BTreeMap<String, FieldDef>,
    pub edge_rules: std::collections::BTreeMap<String, EdgeRuleDef>,
    pub required_states: Vec<String>,
    pub terminal_states: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct SchemaListResponse {
    #[serde(default)]
    pub active_workspace: Option<String>,
    pub workspace: String,
    pub types: Vec<TypeSchema>,
}

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct SchemaDetailResponse {
    #[serde(default)]
    pub active_workspace: Option<String>,
    pub workspace: String,
    pub schema: TypeSchema,
}
