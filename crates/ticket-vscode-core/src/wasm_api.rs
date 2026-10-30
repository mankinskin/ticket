use super::*;
use js_sys::Array;
#[allow(unused_imports)]
use wasm_bindgen::prelude::*;

// ── helpers ─────────────────────────────────────────────────────────────

/// Extract a Vec<String> from a js_sys::Array of JS string values.
fn str_vec(arr: &Array) -> Vec<String> {
    arr.iter().filter_map(|v| v.as_string()).collect()
}

/// Build TicketSummary list from parallel id / title / state arrays.
fn tickets_from_arrays(ids: &Array, titles: &Array, states: &Array) -> Vec<TicketSummary> {
    let n = ids.length() as usize;
    (0..n)
        .map(|i| {
            let i = i as u32;
            TicketSummary::new(
                ids.get(i).as_string().unwrap_or_default(),
                String::new(), // ticket_type not needed for domain logic
                titles.get(i).as_string().unwrap_or_default(),
                states.get(i).as_string().unwrap_or_default(),
            )
        })
        .collect()
}

/// Build EdgeRecord list from parallel from / to / kind arrays.
fn edges_from_arrays(froms: &Array, tos: &Array, kinds: &Array) -> Vec<EdgeRecord> {
    let n = froms.length() as usize;
    (0..n)
        .map(|i| {
            let i = i as u32;
            EdgeRecord::new(
                froms.get(i).as_string().unwrap_or_default(),
                tos.get(i).as_string().unwrap_or_default(),
                kinds.get(i).as_string().unwrap_or_default(),
            )
        })
        .collect()
}

// ── WasmDependencyMaps ───────────────────────────────────────────────────

/// WASM-exposed DependencyMaps wrapper.
///
/// Inputs are parallel `Array<string>` values rather than typed struct arrays.
#[wasm_bindgen(js_name = "WasmDependencyMaps")]
pub struct WasmDependencyMaps {
    inner: super::DependencyMaps,
}

#[wasm_bindgen(js_class = "WasmDependencyMaps")]
impl WasmDependencyMaps {
    /// Build the dependency maps.
    ///
    /// - `ticket_ids`: Array of ticket ID strings.
    /// - `edge_froms`, `edge_tos`, `edge_kinds`: parallel edge arrays.
    #[wasm_bindgen(js_name = "build")]
    pub fn build_js(
        ticket_ids: &Array,
        edge_froms: &Array,
        edge_tos: &Array,
        edge_kinds: &Array,
    ) -> WasmDependencyMaps {
        // DependencyMaps only needs IDs; create minimal TicketSummary stubs.
        let tickets: Vec<TicketSummary> = str_vec(ticket_ids)
            .into_iter()
            .map(|id| TicketSummary::new(id, String::new(), String::new(), String::new()))
            .collect();
        let edges = edges_from_arrays(edge_froms, edge_tos, edge_kinds);
        WasmDependencyMaps {
            inner: super::DependencyMaps::build(&tickets, &edges),
        }
    }

    /// Returns an Array of dependency IDs for the given ticket ID.
    #[wasm_bindgen(js_name = "depsOf")]
    pub fn deps_of_js(&self, id: &str) -> Array {
        self.inner.deps_of.get(id).map_or_else(Array::new, |deps| {
            deps.iter().map(|d| JsValue::from_str(d)).collect::<Array>()
        })
    }

    /// Returns an Array of parent ticket IDs for the given ticket ID.
    #[wasm_bindgen(js_name = "parentOf")]
    pub fn parent_of_js(&self, id: &str) -> Array {
        self.inner.parent_of.get(id).map_or_else(Array::new, |ps| {
            ps.iter().map(|p| JsValue::from_str(p)).collect::<Array>()
        })
    }

    /// Returns true when the ticket has at least one parent.
    #[wasm_bindgen(js_name = "hasParent")]
    pub fn has_parent_js(&self, id: &str) -> bool {
        self.inner.parent_of.contains_key(id)
    }
}

// ── WasmStateGroup ───────────────────────────────────────────────────────

/// WASM-friendly state group returned by `buildStateGroups`.
#[wasm_bindgen(js_name = "WasmStateGroup")]
pub struct WasmStateGroup {
    state: String,
    pub total: usize,
    root_ids: Vec<String>,
}

#[wasm_bindgen(js_class = "WasmStateGroup")]
impl WasmStateGroup {
    #[wasm_bindgen(getter)]
    pub fn state(&self) -> String {
        self.state.clone()
    }

    /// Returns a JS Array of root ticket IDs for this state group.
    #[wasm_bindgen(js_name = "rootIds")]
    pub fn root_ids_js(&self) -> Array {
        self.root_ids
            .iter()
            .map(|id| JsValue::from_str(id))
            .collect::<Array>()
    }
}

// ── buildStateGroups ─────────────────────────────────────────────────────

/// Compute state groups from parallel string arrays.
///
/// - `ticket_ids`, `ticket_titles`, `ticket_states`: parallel ticket arrays.
/// - `edge_froms`, `edge_tos`, `edge_kinds`: parallel edge arrays.
/// - `state_order`: schema-defined state ordering (empty = alphabetical).
/// - `state_filter`: server-side state filter already applied (pass `""` if pre-filtered).
/// - `query`: client-side search string (case-insensitive substring match on title + id).
///
/// Returns a JS `Array` of `WasmStateGroup` objects.
#[wasm_bindgen(js_name = "buildStateGroups")]
pub fn build_state_groups_wasm(
    ticket_ids: &Array,
    ticket_titles: &Array,
    ticket_states: &Array,
    edge_froms: &Array,
    edge_tos: &Array,
    edge_kinds: &Array,
    state_order: &Array,
    state_filter: &str,
    query: &str,
) -> Array {
    let tickets = tickets_from_arrays(ticket_ids, ticket_titles, ticket_states);
    let edges = edges_from_arrays(edge_froms, edge_tos, edge_kinds);
    let state_order_vec = str_vec(state_order);

    super::build_state_groups(&tickets, &edges, &state_order_vec, state_filter, query)
        .into_iter()
        .map(|g| {
            let wsg = WasmStateGroup {
                state: g.state,
                total: g.total,
                root_ids: g.root_ids,
            };
            JsValue::from(wsg)
        })
        .collect::<Array>()
}
