#[allow(unused_imports)]
use super::*;

impl WorkflowModel {
    pub(super) fn build_blocker_tree_node(
        &self,
        ticket_id: Uuid,
        path: &mut HashSet<Uuid>,
    ) -> Option<WorkflowTreeNode> {
        if !self.tickets.contains_key(&ticket_id) {
            return None;
        }
        if !path.insert(ticket_id) {
            return self.finalize_tree_node(ticket_id, 0, false, Vec::new(), 1);
        }

        let child_ids = self.unresolved_dependencies_excluding(&ticket_id, &HashSet::new());
        let remaining_blocker_count = child_ids.len();
        let children = child_ids
            .into_iter()
            .filter_map(|child_id| self.build_blocker_tree_node(child_id, path))
            .collect::<Vec<_>>();

        path.remove(&ticket_id);

        self.finalize_tree_node(
            ticket_id,
            remaining_blocker_count,
            remaining_blocker_count == 0,
            children,
            remaining_blocker_count.max(1),
        )
    }

    pub(super) fn build_unlock_tree_node(
        &self,
        ticket_id: Uuid,
        satisfied_ids: &HashSet<Uuid>,
        allow_frontier: bool,
        path: &mut HashSet<Uuid>,
    ) -> Option<WorkflowTreeNode> {
        let ticket = self.tickets.get(&ticket_id)?;
        if !path.insert(ticket_id) {
            return self.finalize_tree_node(ticket_id, 0, false, Vec::new(), 1);
        }

        let child_ids = self
            .reverse_map
            .get(&ticket_id)
            .into_iter()
            .flatten()
            .filter(|child_id| {
                self.tickets
                    .get(child_id)
                    .map(|child| is_candidate_state(child.state.as_deref()))
                    .unwrap_or(false)
            })
            .copied()
            .collect::<Vec<_>>();
        let remaining_blocker_count = self
            .unresolved_dependencies_excluding(&ticket_id, satisfied_ids)
            .len();
        let is_frontier = allow_frontier
            && remaining_blocker_count == 0
            && is_candidate_state(ticket.state.as_deref());
        let children = child_ids
            .into_iter()
            .filter_map(|child_id| self.build_unlock_tree_node(child_id, satisfied_ids, true, path))
            .collect::<Vec<_>>();

        path.remove(&ticket_id);

        self.finalize_tree_node(
            ticket_id,
            remaining_blocker_count,
            is_frontier,
            children,
            remaining_blocker_count.max(1),
        )
    }

    pub(super) fn finalize_tree_node(
        &self,
        ticket_id: Uuid,
        remaining_blocker_count: usize,
        is_frontier: bool,
        mut children: Vec<WorkflowTreeNode>,
        fallback_distance: usize,
    ) -> Option<WorkflowTreeNode> {
        let ticket = self.tickets.get(&ticket_id)?;
        self.sort_tree_nodes(&mut children);

        let frontier_leaf_ids = if is_frontier {
            vec![ticket_id]
        } else if children.is_empty() {
            vec![ticket_id]
        } else {
            children
                .iter()
                .flat_map(|child| child.frontier_leaf_ids.iter().copied())
                .collect()
        };
        let blocker_distance = if is_frontier {
            0
        } else if children.is_empty() {
            fallback_distance
        } else {
            children
                .iter()
                .map(|child| child.blocker_distance.saturating_add(1))
                .min()
                .unwrap_or(fallback_distance)
        };
        let metrics = self.metrics.get(&ticket_id).cloned().unwrap_or_default();

        Some(WorkflowTreeNode {
            ticket_id,
            title: ticket.title.clone(),
            state: ticket.state.clone(),
            priority: self.priority_or_none(&ticket_id).to_string(),
            children,
            remaining_blocker_count,
            unresolved_frontier_leaf_count: frontier_leaf_ids.len(),
            frontier_leaf_ids,
            blocker_distance,
            is_frontier,
            dependency_count: metrics.dependency_count,
            immediate_dependees: metrics.immediate_dependees,
            transitive_reverse_dependents: metrics.transitive_reverse_dependents,
            affected_reverse_dependent_reach: metrics.affected_reverse_dependent_reach,
            dependency_state_gap: metrics.dependency_state_gap,
        })
    }

    pub(super) fn sort_tree_nodes(&self, nodes: &mut [WorkflowTreeNode]) {
        nodes.sort_by(|left, right| {
            left.unresolved_frontier_leaf_count
                .cmp(&right.unresolved_frontier_leaf_count)
                .then_with(|| left.blocker_distance.cmp(&right.blocker_distance))
                .then_with(|| {
                    effort_sort_key(self.effort(&left.ticket_id))
                        .cmp(&effort_sort_key(self.effort(&right.ticket_id)))
                })
                .then_with(|| right.dependency_state_gap.cmp(&left.dependency_state_gap))
                .then_with(|| {
                    right
                        .affected_reverse_dependent_reach
                        .cmp(&left.affected_reverse_dependent_reach)
                })
                .then_with(|| {
                    right
                        .transitive_reverse_dependents
                        .cmp(&left.transitive_reverse_dependents)
                })
                .then_with(|| {
                    priority_weight(&left.priority).cmp(&priority_weight(&right.priority))
                })
                .then_with(|| {
                    left.title
                        .as_deref()
                        .unwrap_or("")
                        .cmp(right.title.as_deref().unwrap_or(""))
                })
                .then_with(|| left.ticket_id.cmp(&right.ticket_id))
        });
    }
}
