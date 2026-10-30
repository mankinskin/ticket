//! Canonical health-finding generation for ticket stores.
//!
//! All three transport surfaces (CLI, HTTP, MCP) delegate to this module so
//! that finding keys, severities, and message text are identical regardless of
//! how they are serialized to their respective envelopes.

use std::{collections::BTreeMap, path::PathBuf};

use serde::Serialize;
use uuid::Uuid;

use memory_kernel::{
    ContentKind,
    cross_store_edges::{
        CrossStoreEdgeClassifier, EdgeReferenceResolution, cross_workspace_edge_instructions,
        cross_workspace_edge_message,
    },
};

use crate::{
    model::edge::EdgeRecord,
    storage::{indexed::IndexedTicket, store::TicketStore, ticket_fs::TicketFs},
    workflow::WorkflowModel,
};

// ─── Types ───────────────────────────────────────────────────────────────────

/// One normalized health finding for a ticket.
#[derive(Debug, Clone, Serialize)]
pub struct HealthFinding {
    pub ticket_id: Uuid,
    pub short_id: String,
    pub title: String,
    pub path: PathBuf,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub state: Option<String>,
    pub r#type: String,
    pub check: String,
    pub severity: String,
    pub message: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub instructions: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prerequisite_id: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prerequisite_title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prerequisite_state: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dependent_id: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dependent_state: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dependency_state_gap: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub affected_reverse_dependent_reach: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transitive_reverse_dependents: Option<usize>,
}

/// Aggregated health findings for a set of tickets.
#[derive(Debug, Default)]
pub struct HealthReport {
    /// Count of findings grouped by check key.
    pub summary: BTreeMap<String, u64>,
    /// Individual findings in ticket-visit order.
    pub findings: Vec<HealthFinding>,
}

impl HealthReport {
    fn record(&mut self, check: &str, finding: HealthFinding) {
        *self.summary.entry(check.to_string()).or_insert(0) += 1;
        self.findings.push(finding);
    }
}

// ─── Public entry point ──────────────────────────────────────────────────────

/// Produce a normalized `HealthReport` for `tickets` using edge and workflow
/// context.
///
/// Graph participation checks run for all tickets to preserve topology parity,
/// while non-topology checks skip done/cancelled tickets.
///
/// This is the single canonical implementation consumed by CLI, HTTP, and MCP.
pub fn collect_findings(
    store: &TicketStore,
    tickets: &[IndexedTicket],
    all_edges: &[EdgeRecord],
    workflow: &WorkflowModel,
) -> HealthReport {
    let policy = crate::workspace_policy::load_workspace_policy(
        &memory_kernel::workspace::resolve_workspace_root_from_store_root(
            &memory_kernel::workspace::resolve_store_root_from(
                &store.index_root,
                memory_kernel::workspace::TICKET_INDEX_DIR,
            ),
            memory_kernel::workspace::TICKET_INDEX_DIR,
        ),
    );
    let edge_classifier =
        CrossStoreEdgeClassifier::for_store(&store.index_root, ContentKind::Ticket, policy);

    let done_ids = tickets
        .iter()
        .filter(|t| matches!(t.state.as_deref(), Some("done") | Some("cancelled")))
        .map(|t| t.id)
        .collect::<std::collections::HashSet<_>>();

    let mut report = HealthReport::default();
    for ticket in tickets {
        append_schema_findings(store, ticket, &mut report);
        append_graph_participation_findings(ticket, all_edges, &mut report);
        if done_ids.contains(&ticket.id) {
            continue;
        }
        append_ticket_findings(
            store,
            ticket,
            all_edges,
            workflow,
            edge_classifier.as_ref(),
            &mut report,
        );
    }
    report
}

// ─── Per-ticket finding generators ───────────────────────────────────────────

fn append_schema_findings(store: &TicketStore, ticket: &IndexedTicket, report: &mut HealthReport) {
    let Some(schema) = store.schema_registry().get(&ticket.type_id) else {
        report.record(
            "unknown_type",
            base_finding(
                ticket,
                "unknown_type",
                "error",
                format!(
                    "Ticket type '{}' has no registered schema; transitions and validation will fail.",
                    ticket.type_id
                ),
                vec![
                    "Register a built-in schema for this type in ticket-api (model/default_schema.rs and SchemaRegistry::with_builtins), or load a matching TOML schema.".to_string(),
                    "If the type is a mistake, retype the ticket to a supported type before continuing.".to_string(),
                ],
            ),
        );
        return;
    };

    if let Some(state) = ticket.state.as_deref() {
        if !schema.states.iter().any(|s| s == state) {
            report.record(
                "off_schema_state",
                base_finding(
                    ticket,
                    "off_schema_state",
                    "error",
                    format!(
                        "Ticket state '{}' is not a member of type '{}' schema's allowed states [{}]; the ticket has zero legal transitions.",
                        state,
                        ticket.type_id,
                        schema.states.join(", ")
                    ),
                    vec![
                        format!(
                            "Set 'state' directly in the ticket's manifest to one of: {}.",
                            schema.states.join(", ")
                        ),
                        "field/field_map patches reject 'state' writes; edit the manifest state field directly or use a supported transition once the state is legal again.".to_string(),
                    ],
                ),
            );
        }
    }
}

fn append_ticket_findings(
    store: &TicketStore,
    ticket: &IndexedTicket,
    all_edges: &[EdgeRecord],
    workflow: &WorkflowModel,
    edge_classifier: Option<&CrossStoreEdgeClassifier>,
    report: &mut HealthReport,
) {
    append_effort_estimation_findings(ticket, workflow, report);
    append_description_findings(ticket, report);
    append_title_finding(ticket, report);
    append_dependency_state_findings(ticket, workflow, report);
    append_dangling_edge_findings(store, ticket, all_edges, edge_classifier, report);
}

fn append_effort_estimation_findings(
    ticket: &IndexedTicket,
    workflow: &WorkflowModel,
    report: &mut HealthReport,
) {
    if workflow.effort(&ticket.id).is_some() {
        return;
    }

    report.record(
        "missing_effort_estimation",
        base_finding(
            ticket,
            "missing_effort_estimation",
            "warning",
            "Ticket is missing an effort estimation (for example '1200' or '2.5k tokens').".to_string(),
            vec![
                "Set the ticket 'effort' field to a token-budget estimate so planning and ranking remain accurate.".to_string(),
            ],
        ),
    );
}

fn append_graph_participation_findings(
    ticket: &IndexedTicket,
    all_edges: &[EdgeRecord],
    report: &mut HealthReport,
) {
    let dependency_count = all_edges
        .iter()
        .filter(|edge| edge.kind == "depends_on" && edge.from == ticket.id)
        .count();
    let dependee_count = all_edges
        .iter()
        .filter(|edge| edge.kind == "depends_on" && edge.to == ticket.id)
        .count();

    if dependency_count > 0 || dependee_count > 0 {
        return;
    }

    let title = ticket.title.as_deref().unwrap_or("?").to_string();
    let path = ticket.path.to_string_lossy().to_string();
    report.record(
        "graph_participation",
        base_finding(
            ticket,
            "graph_participation",
            "warning",
            format!("{title} is not linked into the depends_on graph."),
            vec![
                format!(
                    "Link {path} to real prerequisites or dependees using depends_on edges."
                ),
                "If this is standalone work, place it under a tracker ticket to keep the graph connected.".to_string(),
            ],
        ),
    );
}

fn append_description_findings(ticket: &IndexedTicket, report: &mut HealthReport) {
    match TicketFs::read_description(&ticket.path) {
        Ok(None) => report.record(
            "missing_description",
            base_finding(
                ticket,
                "missing_description",
                "warning",
                "No description.md file — ticket lacks detailed context.".into(),
                vec![
                    "Add a description.md with goal, scope, acceptance checks, and current status."
                        .to_string(),
                ],
            ),
        ),
        Ok(Some(body)) => {
            let trimmed_len = body.trim().len();
            if trimmed_len < 50 {
                report.record(
                    "short_description",
                    base_finding(
                        ticket,
                        "short_description",
                        "info",
                        format!(
                            "description.md is very short ({trimmed_len} chars) — consider adding more detail."
                        ),
                        vec![
                            "Expand description.md with acceptance criteria, dependencies, and validation evidence.".to_string(),
                        ],
                    ),
                );
            }
        }
        Err(error) => report.record(
            "description_read_error",
            base_finding(
                ticket,
                "description_read_error",
                "error",
                format!("Unable to read ticket description: {error}"),
                vec!["Resolve the ticket filesystem or transaction-recovery error, then rerun health.".to_string()],
            ),
        ),
    }
}

fn append_title_finding(ticket: &IndexedTicket, report: &mut HealthReport) {
    if ticket.title.is_none() || ticket.title.as_deref() == Some("") {
        report.record(
            "missing_title",
            base_finding(
                ticket,
                "missing_title",
                "error",
                "Ticket has no title.".into(),
                vec![
                    "Set a concise ticket title describing the change intent and scope."
                        .to_string(),
                ],
            ),
        );
    }
}

fn append_dependency_state_findings(
    ticket: &IndexedTicket,
    workflow: &WorkflowModel,
    report: &mut HealthReport,
) {
    let state = ticket.state.as_deref().unwrap_or("");
    if state == "planning" {
        return;
    }

    for inversion in workflow
        .dependency_state_inversions(&ticket.id)
        .into_iter()
        .flatten()
    {
        report.record(
            "dependency_convergence",
            HealthFinding {
                message: format!(
                    "Ticket depends on {} in earlier state '{}' while this ticket is '{}'.",
                    short_id(inversion.prerequisite_id),
                    inversion.prerequisite_state.as_deref().unwrap_or("?"),
                    inversion.dependent_state.as_deref().unwrap_or(state),
                ),
                instructions: vec![
                    "Advance the prerequisite ticket before continuing this dependent ticket when dependency order is still valid.".to_string(),
                    "If out-of-order progress is intentional, document the exception and update states to make it explicit.".to_string(),
                ],
                prerequisite_id: Some(inversion.prerequisite_id),
                prerequisite_title: inversion.prerequisite_title.clone(),
                prerequisite_state: inversion.prerequisite_state.clone(),
                dependent_id: Some(inversion.dependent_id),
                dependent_state: inversion.dependent_state.clone(),
                dependency_state_gap: Some(inversion.dependency_state_gap),
                affected_reverse_dependent_reach: Some(
                    inversion.affected_reverse_dependent_reach,
                ),
                transitive_reverse_dependents: Some(inversion.transitive_reverse_dependents),
                ..base_finding(
                    ticket,
                    "dependency_convergence",
                    "warning",
                    String::new(),
                    Vec::new(),
                )
            },
        );
    }
}

fn append_dangling_edge_findings(
    store: &TicketStore,
    ticket: &IndexedTicket,
    all_edges: &[EdgeRecord],
    edge_classifier: Option<&CrossStoreEdgeClassifier>,
    report: &mut HealthReport,
) {
    for edge in all_edges {
        if edge.from != ticket.id || edge.kind != "depends_on" {
            continue;
        }
        let target_exists = store.get_indexed(&edge.to).ok().flatten().is_some();
        if target_exists {
            continue;
        }

        if let Some(classifier) = edge_classifier {
            match classifier.classify(edge.to) {
                EdgeReferenceResolution::Ok => continue,
                EdgeReferenceResolution::CrossWorkspaceEdge {
                    target_workspace_root,
                    ..
                } => {
                    report.record(
                        "cross_workspace_edge",
                        base_finding(
                            ticket,
                            "cross_workspace_edge",
                            "warning",
                            cross_workspace_edge_message(edge.to, &target_workspace_root),
                            cross_workspace_edge_instructions(),
                        ),
                    );
                    continue;
                }
                EdgeReferenceResolution::DanglingEdge => {}
            }
        }

        report.record(
            "dangling_edge",
            base_finding(
                ticket,
                "dangling_edge",
                "error",
                format!(
                    "depends_on edge points to {} which is missing.",
                    short_id(edge.to)
                ),
                vec![
                    "Remove or retarget the stale depends_on edge to an existing prerequisite ticket.".to_string(),
                ],
            ),
        );
    }
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

fn short_id(id: Uuid) -> String {
    id.to_string()[..8].to_string()
}

fn base_finding(
    ticket: &IndexedTicket,
    check: &str,
    severity: &str,
    message: String,
    instructions: Vec<String>,
) -> HealthFinding {
    HealthFinding {
        ticket_id: ticket.id,
        short_id: short_id(ticket.id),
        title: ticket.title.as_deref().unwrap_or("?").to_string(),
        path: ticket.path.clone(),
        state: ticket.state.clone(),
        r#type: ticket.type_id.clone(),
        check: check.to_string(),
        severity: severity.to_string(),
        message,
        instructions,
        ..Default::default()
    }
}

impl Default for HealthFinding {
    fn default() -> Self {
        Self {
            ticket_id: Uuid::nil(),
            short_id: String::new(),
            title: String::new(),
            path: PathBuf::new(),
            state: None,
            r#type: String::new(),
            check: String::new(),
            severity: String::new(),
            message: String::new(),
            instructions: Vec::new(),
            prerequisite_id: None,
            prerequisite_title: None,
            prerequisite_state: None,
            dependent_id: None,
            dependent_state: None,
            dependency_state_gap: None,
            affected_reverse_dependent_reach: None,
            transitive_reverse_dependents: None,
        }
    }
}

#[cfg(test)]
#[path = "health/tests.rs"]
mod tests;
