#[allow(unused_imports)]
use super::*;

#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowName {
    List,
    TriageOpenTickets,
    FetchTicketContext,
    InspectDependencies,
}

// Wire shape for the `update_ticket` MCP tool: the raw `description` +
// `description_mode` pair as received over JSON. Drives both deserialization
// and the advertised JSON Schema for `UpdateTicketInput` so the tool's schema
// is unchanged; never used past decoding into a single `DescriptionUpdate` so
// "content without a mode" cannot be constructed downstream (AC5 of ticket
// 3d952036). Not a doc comment: schemars would otherwise fold this into the
// root schema's `description`, changing the advertised schema.

#[derive(Debug, Deserialize, JsonSchema)]
pub(super) struct UpdateTicketInputWire {
    pub workspace: String,
    pub id: String,
    #[serde(default)]
    pub transition_states: Vec<String>,
    #[serde(default)]
    pub to_state: Option<String>,
    #[serde(default)]
    pub fields: Option<Vec<String>>,
    #[serde(default)]
    pub field_map: Option<BTreeMap<String, Value>>,
    #[serde(default)]
    pub undo: bool,
    #[serde(default)]
    pub description: Option<String>,
    /// How to apply `description`: `"replace"` (overwrites) or `"append"`
    /// (preserves existing content, concatenating onto it). Required
    /// whenever `description` is set — there is no default, and omitting it
    /// is rejected rather than silently applying `replace`.
    #[serde(default)]
    pub description_mode: Option<String>,
    #[serde(default)]
    pub author: Option<String>,
    /// Opt out of auto-walking multi-hop transitions. When true, a `to_state`
    /// that would skip a required waypoint is rejected with recovery guidance
    /// instead of traversing the intermediate states.
    #[serde(default)]
    pub single_hop: bool,
}

#[derive(Debug, Deserialize)]
#[serde(try_from = "UpdateTicketInputWire")]
pub struct UpdateTicketInput {
    pub workspace: String,
    pub id: String,
    pub transition_states: Vec<String>,
    pub to_state: Option<String>,
    pub fields: Option<Vec<String>>,
    pub field_map: Option<BTreeMap<String, Value>>,
    pub undo: bool,
    /// How `description` (if any) applies to `description.md`. There is no
    /// separate `description_mode` field: [`DescriptionUpdate::Unchanged`]
    /// means no description change, and content only exists paired with an
    /// explicit mode (AC5 of ticket 3d952036).
    pub description_update: DescriptionUpdate,
    pub author: Option<String>,
    pub single_hop: bool,
}

impl TryFrom<UpdateTicketInputWire> for UpdateTicketInput {
    type Error = String;

    fn try_from(wire: UpdateTicketInputWire) -> Result<Self, Self::Error> {
        let description_update =
            DescriptionUpdate::decode(wire.description, wire.description_mode.as_deref())?;
        Ok(UpdateTicketInput {
            workspace: wire.workspace,
            id: wire.id,
            transition_states: wire.transition_states,
            to_state: wire.to_state,
            fields: wire.fields,
            field_map: wire.field_map,
            undo: wire.undo,
            description_update,
            author: wire.author,
            single_hop: wire.single_hop,
        })
    }
}

// Field layout, docs, and required set delegate to `UpdateTicketInputWire` so
// the advertised MCP tool schema keeps the `description` / `description_mode`
// JSON fields exactly as before this type stopped deriving `JsonSchema`
// directly. `schema_name`/`schema_id` are NOT delegated: `root_schema_for`
// uses `T::schema_name()` (the outer type, i.e. `UpdateTicketInput`) for the
// root `"title"`, so these are set to the literal values the original
// `#[derive(JsonSchema)]` on `UpdateTicketInput` itself would have produced.
impl JsonSchema for UpdateTicketInput {
    fn schema_name() -> Cow<'static, str> {
        Cow::Borrowed("UpdateTicketInput")
    }

    fn schema_id() -> Cow<'static, str> {
        Cow::Borrowed(concat!(module_path!(), "::UpdateTicketInput"))
    }

    fn json_schema(generator: &mut SchemaGenerator) -> Schema {
        UpdateTicketInputWire::json_schema(generator)
    }

    fn inline_schema() -> bool {
        false
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct CloseTicketInput {
    pub workspace: String,
    pub id: String,
    #[serde(default = "default_close_state")]
    pub to_state: String,
    #[serde(default)]
    pub author: Option<String>,
}

pub(super) fn default_close_state() -> String {
    "done".to_string()
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct CancelTicketInput {
    pub workspace: String,
    pub id: String,
    #[serde(default)]
    pub author: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct CreateTicketInput {
    pub workspace: String,
    #[serde(rename = "type")]
    pub type_id: String,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub state: Option<String>,
    #[serde(default)]
    pub fields: Vec<String>,
    #[serde(default)]
    pub description: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct DeleteTicketInput {
    pub workspace: String,
    pub id: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct AddEdgeInput {
    pub workspace: String,
    pub from: String,
    pub to: String,
    pub kind: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct RemoveEdgeInput {
    pub workspace: String,
    pub from: String,
    pub to: String,
    pub kind: String,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum DanglingStrategy {
    Unlink,
    ReconcileOnly,
}

impl DanglingStrategy {
    pub fn mutates(&self) -> bool {
        matches!(self, Self::Unlink)
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Unlink => "unlink",
            Self::ReconcileOnly => "reconcile_only",
        }
    }
}

pub(super) fn default_dangling_kind() -> String {
    "depends_on".to_string()
}

pub(super) fn default_dangling_strategy() -> DanglingStrategy {
    DanglingStrategy::Unlink
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct PruneDanglingEdgesInput {
    pub workspace: String,
    #[serde(default)]
    pub root: Option<String>,
    #[serde(default)]
    pub all: bool,
    #[serde(default = "default_dangling_kind")]
    pub kind: String,
    #[serde(default = "default_dangling_strategy")]
    pub strategy: DanglingStrategy,
    #[serde(default)]
    pub reason: Option<String>,
}
