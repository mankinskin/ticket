#[allow(unused_imports)]
use super::*;

impl WorkflowModel {
    pub fn ticket(&self, ticket_id: &Uuid) -> Option<&IndexedTicket> {
        self.tickets.get(ticket_id)
    }

    pub fn priority(&self, ticket_id: &Uuid) -> Option<&str> {
        self.priorities.get(ticket_id).map(String::as_str)
    }

    pub fn priority_or_none(&self, ticket_id: &Uuid) -> &str {
        self.priority(ticket_id).unwrap_or("none")
    }

    pub fn effort(&self, ticket_id: &Uuid) -> Option<u64> {
        self.efforts.get(ticket_id).copied()
    }

    pub fn metrics(&self, ticket_id: &Uuid) -> Option<&TicketConvergenceMetrics> {
        self.metrics.get(ticket_id)
    }

    pub fn dependency_count(&self, ticket_id: &Uuid) -> usize {
        self.dependency_counts.get(ticket_id).copied().unwrap_or(0)
    }

    pub fn dependee_count(&self, ticket_id: &Uuid) -> usize {
        self.dependee_counts.get(ticket_id).copied().unwrap_or(0)
    }

    pub fn unresolved_dependencies(&self, ticket_id: &Uuid) -> Option<&[Uuid]> {
        self.unresolved_deps.get(ticket_id).map(Vec::as_slice)
    }

    pub fn actionable_candidate_ids(&self, scope: Option<&HashSet<Uuid>>) -> Vec<Uuid> {
        self.actionable_candidate_ids_with_satisfied(scope, &HashSet::new())
    }

    /// Return actionable candidates while treating selected ticket ids as satisfied.
    ///
    /// `unblocked-by <id>` uses this to rank remaining blocker work beneath a
    /// prerequisite without requiring the root ticket to be completed first.
    pub fn actionable_candidate_ids_with_satisfied(
        &self,
        scope: Option<&HashSet<Uuid>>,
        satisfied_ids: &HashSet<Uuid>,
    ) -> Vec<Uuid> {
        self.eligible_candidate_ids(scope)
            .into_iter()
            .filter(|ticket_id| {
                self.unresolved_dependencies_excluding(ticket_id, satisfied_ids)
                    .is_empty()
            })
            .collect()
    }

    pub fn eligible_candidate_ids(&self, scope: Option<&HashSet<Uuid>>) -> Vec<Uuid> {
        self.tickets
            .values()
            .filter(|ticket| scope.is_none_or(|ids| ids.contains(&ticket.id)))
            .filter(|ticket| is_candidate_state(ticket.state.as_deref()))
            .map(|ticket| ticket.id)
            .collect()
    }

    pub fn sort_candidate_ids(&self, candidates: &mut [Uuid]) {
        candidates.sort_by(|left, right| self.compare_candidate_ids(*left, *right));
    }

    /// Return the set of ticket IDs whose title starts with `filter`, or `None`
    /// when no filter is supplied.  Adapters should call this instead of
    /// re-implementing title-prefix filtering locally.
    pub fn filter_scope(tickets: &[IndexedTicket], filter: Option<&str>) -> Option<HashSet<Uuid>> {
        filter.map(|prefix| {
            tickets
                .iter()
                .filter(|t| t.title.as_deref().unwrap_or("").starts_with(prefix))
                .map(|t| t.id)
                .collect()
        })
    }
}
