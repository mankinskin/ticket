#[allow(unused_imports)]
use super::*;

impl WorkflowModel {
    /// Build the shared workflow model from indexed tickets and dependency edges.
    pub fn build(
        store: &TicketStore,
        tickets: Vec<IndexedTicket>,
        all_edges: Vec<EdgeRecord>,
    ) -> Result<Self, StorageError> {
        let state_index = build_state_index(store);
        let (priorities, efforts) = read_priority_and_effort(&tickets)?;
        let workflow_facts = store
            .get_workflow_facts_many(&tickets.iter().map(|ticket| ticket.id).collect::<Vec<_>>())?;
        Ok(Self::build_from_parts(
            tickets,
            all_edges,
            state_index,
            priorities,
            efforts,
            workflow_facts,
        ))
    }

    pub(super) fn build_from_parts(
        tickets: Vec<IndexedTicket>,
        all_edges: Vec<EdgeRecord>,
        state_index: HashMap<String, usize>,
        priorities: HashMap<Uuid, String>,
        efforts: HashMap<Uuid, u64>,
        workflow_facts: HashMap<Uuid, WorkflowFacts>,
    ) -> Self {
        let tickets: HashMap<Uuid, IndexedTicket> = tickets
            .into_iter()
            .map(|ticket| (ticket.id, ticket))
            .collect();
        let dependency_counts = dependency_counts(&tickets, &all_edges);
        let dependee_counts = dependee_counts(&tickets, &all_edges);
        let unresolved_deps = unresolved_dependency_map(&tickets, &all_edges);
        let reverse_map = reverse_map(&tickets, &all_edges);
        let metrics = compute_metrics(
            &tickets,
            &state_index,
            &dependency_counts,
            &dependee_counts,
            &reverse_map,
            &workflow_facts,
        );
        let inversions_by_dependent =
            compute_dependency_state_inversions(&tickets, &all_edges, &state_index, &metrics);

        Self {
            tickets,
            state_index,
            priorities,
            efforts,
            dependency_counts,
            dependee_counts,
            unresolved_deps,
            reverse_map,
            metrics,
            inversions_by_dependent,
        }
    }

    pub(super) fn compare_candidate_ids(&self, left: Uuid, right: Uuid) -> Ordering {
        let Some(left_ticket) = self.tickets.get(&left) else {
            return Ordering::Greater;
        };
        let Some(right_ticket) = self.tickets.get(&right) else {
            return Ordering::Less;
        };
        let left_metrics = self.metrics.get(&left).cloned().unwrap_or_default();
        let right_metrics = self.metrics.get(&right).cloned().unwrap_or_default();

        right_metrics
            .max_affected_dependent_state_index
            .unwrap_or(0)
            .cmp(&left_metrics.max_affected_dependent_state_index.unwrap_or(0))
            .then_with(|| {
                right_metrics
                    .dependency_state_gap
                    .cmp(&left_metrics.dependency_state_gap)
            })
            .then_with(|| {
                right_metrics
                    .affected_reverse_dependent_reach
                    .cmp(&left_metrics.affected_reverse_dependent_reach)
            })
            .then_with(|| {
                effort_sort_key(self.effort(&left)).cmp(&effort_sort_key(self.effort(&right)))
            })
            .then_with(|| {
                right_metrics
                    .became_actionable_at
                    .cmp(&left_metrics.became_actionable_at)
            })
            .then_with(|| {
                priority_weight(self.priority_or_none(&left))
                    .cmp(&priority_weight(self.priority_or_none(&right)))
            })
            .then_with(|| {
                self.state_rank(right_ticket.state.as_deref())
                    .cmp(&self.state_rank(left_ticket.state.as_deref()))
            })
            .then_with(|| {
                right_metrics
                    .transitive_reverse_dependents
                    .cmp(&left_metrics.transitive_reverse_dependents)
            })
            .then_with(|| {
                right_metrics
                    .immediate_dependees
                    .cmp(&left_metrics.immediate_dependees)
            })
            .then_with(|| right_ticket.created_at.cmp(&left_ticket.created_at))
            .then_with(|| ticket_title(left_ticket).cmp(ticket_title(right_ticket)))
            .then_with(|| left.cmp(&right))
    }
}
