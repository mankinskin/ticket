//! Shared dependency-convergence model for ticket ranking, health, and audit.
//!
//! `WorkflowModel` is the canonical place to derive reverse-dependency pressure
//! and dependency-state inversion evidence. Consumers should use this module
//! instead of reimplementing graph traversal or state-gap logic so `ticket
//! next`, `ticket-mcp next_tickets`, ticket health surfaces, and repo audit
//! stay aligned.

use std::{
    cmp::Ordering,
    collections::{HashMap, HashSet, VecDeque},
};

use chrono::{DateTime, Utc};
use serde::Serialize;
use uuid::Uuid;

use crate::{
    BoardEntryStatus, BoardSnapshot,
    error::StorageError,
    model::edge::EdgeRecord,
    storage::{
        indexed::{IndexedTicket, WorkflowFacts},
        store::TicketStore,
        ticket_fs::TicketFs,
    },
};

const DONE_STATES: &[&str] = &["done", "cancelled"];
const PAUSED_STATES: &[&str] = &["on-hold"];

/// Derived ranking and explainability fields for one ticket candidate.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TicketConvergenceMetrics {
    pub dependency_count: usize,
    pub immediate_dependees: usize,
    pub transitive_reverse_dependents: usize,
    pub affected_reverse_dependent_reach: usize,
    pub max_affected_dependent_state: Option<String>,
    pub max_affected_dependent_state_index: Option<usize>,
    pub dependency_state_gap: usize,
    pub became_actionable_at: Option<DateTime<Utc>>,
    pub last_blocker_progress_at: Option<DateTime<Utc>>,
}

/// Evidence that a prerequisite is lagging behind a more advanced dependent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DependencyStateInversion {
    pub dependent_id: Uuid,
    pub dependent_title: Option<String>,
    pub dependent_state: Option<String>,
    pub prerequisite_id: Uuid,
    pub prerequisite_title: Option<String>,
    pub prerequisite_state: Option<String>,
    pub dependency_state_gap: usize,
    pub affected_reverse_dependent_reach: usize,
    pub transitive_reverse_dependents: usize,
}

/// Nested workflow tree node used by blocker and unlock exploration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkflowTreeNode {
    pub ticket_id: Uuid,
    pub title: Option<String>,
    pub state: Option<String>,
    pub priority: String,
    pub children: Vec<WorkflowTreeNode>,
    pub remaining_blocker_count: usize,
    pub unresolved_frontier_leaf_count: usize,
    pub frontier_leaf_ids: Vec<Uuid>,
    pub blocker_distance: usize,
    pub is_frontier: bool,
    pub dependency_count: usize,
    pub immediate_dependees: usize,
    pub transitive_reverse_dependents: usize,
    pub affected_reverse_dependent_reach: usize,
    pub dependency_state_gap: usize,
}

/// Shared root-scoped blocker view for `next <id>` style commands.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RootBlockerScope {
    pub tree: WorkflowTreeNode,
    pub remaining_blockers: HashSet<Uuid>,
    pub reachable_dependencies: usize,
    pub blocked_dependencies: usize,
}

/// Board-owned ticket surfaced separately from visible workflow candidates.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct BoardExcludedCandidate {
    pub ticket_id: Uuid,
    pub agent_id: String,
    pub status: String,
    pub intent: String,
}

/// Board-aware candidate view used by `next` surfaces.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BoardAwareCandidates {
    pub candidates: Vec<Uuid>,
    pub excluded_by_board: Vec<BoardExcludedCandidate>,
    pub warnings: Vec<String>,
}

/// Canonical dependency-convergence graph model shared across ticket surfaces.
pub struct WorkflowModel {
    tickets: HashMap<Uuid, IndexedTicket>,
    state_index: HashMap<String, usize>,
    priorities: HashMap<Uuid, String>,
    efforts: HashMap<Uuid, u64>,
    dependency_counts: HashMap<Uuid, usize>,
    dependee_counts: HashMap<Uuid, usize>,
    unresolved_deps: HashMap<Uuid, Vec<Uuid>>,
    reverse_map: HashMap<Uuid, Vec<Uuid>>,
    metrics: HashMap<Uuid, TicketConvergenceMetrics>,
    inversions_by_dependent: HashMap<Uuid, Vec<DependencyStateInversion>>,
}

#[path = "workflow_metrics.rs"]
mod workflow_metrics;
pub use workflow_metrics::parse_effort;
use workflow_metrics::*;

impl WorkflowModel {}

#[cfg(test)]
#[path = "workflow/tests.rs"]
mod tests;

fn collect_blocker_descendants(root: &WorkflowTreeNode) -> HashSet<Uuid> {
    let mut ids = HashSet::new();
    for child in &root.children {
        collect_tree_node_ids(child, &mut ids);
    }
    ids
}

fn collect_tree_node_ids(node: &WorkflowTreeNode, ids: &mut HashSet<Uuid>) {
    ids.insert(node.ticket_id);
    for child in &node.children {
        collect_tree_node_ids(child, ids);
    }
}

mod model_build;

mod candidate_queries;

mod dependency_graph;

mod tree_builders;

mod board_filter;
#[allow(unused_imports)]
pub use board_filter::*;
