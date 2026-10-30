#[allow(unused_imports)]
use super::*;

impl WorkflowModel {
    /// Collect all transitive reverse dependents that directly or indirectly
    /// rely on the supplied ticket.
    pub fn reverse_dependents(&self, root_id: Uuid) -> HashSet<Uuid> {
        let mut visited = HashSet::new();
        let mut dependents = HashSet::new();
        let mut queue = VecDeque::from([root_id]);

        while let Some(current) = queue.pop_front() {
            if !visited.insert(current) {
                continue;
            }

            for dependent in self.reverse_map.get(&current).into_iter().flatten() {
                if dependents.insert(*dependent) {
                    queue.push_back(*dependent);
                }
            }
        }

        dependents.remove(&root_id);
        dependents
    }

    pub fn remaining_blockers_for_dependents(
        &self,
        dependent_ids: &HashSet<Uuid>,
    ) -> HashSet<Uuid> {
        self.remaining_blockers_for_dependents_with_satisfied(dependent_ids, &HashSet::new())
    }

    /// Return the unresolved prerequisite ids for a dependent set while
    /// treating selected tickets as already satisfied.
    pub fn remaining_blockers_for_dependents_with_satisfied(
        &self,
        dependent_ids: &HashSet<Uuid>,
        satisfied_ids: &HashSet<Uuid>,
    ) -> HashSet<Uuid> {
        dependent_ids
            .iter()
            .flat_map(|ticket_id| self.unresolved_dependencies_excluding(ticket_id, satisfied_ids))
            .collect()
    }

    pub fn unresolved_dependencies_excluding(
        &self,
        ticket_id: &Uuid,
        satisfied_ids: &HashSet<Uuid>,
    ) -> Vec<Uuid> {
        self.unresolved_deps
            .get(ticket_id)
            .into_iter()
            .flatten()
            .filter(|dependency_id| !satisfied_ids.contains(dependency_id))
            .copied()
            .collect()
    }

    /// Build the canonical root-scoped blocker scope used by `next <id>`.
    ///
    /// This returns the unresolved blocker tree beneath the supplied root,
    /// plus descendant blocker ids and summary counts.
    pub fn root_blocker_scope(&self, root_id: Uuid) -> Option<RootBlockerScope> {
        let tree = self.blocker_tree(root_id)?;
        let remaining_blockers = collect_blocker_descendants(&tree);
        let blocked_dependencies = remaining_blockers
            .iter()
            .filter(|ticket_id| {
                !self
                    .unresolved_dependencies_excluding(ticket_id, &HashSet::new())
                    .is_empty()
            })
            .count();

        Some(RootBlockerScope {
            reachable_dependencies: remaining_blockers.len(),
            blocked_dependencies,
            remaining_blockers,
            tree,
        })
    }

    /// Build an upstream blocker tree from unresolved `depends_on` edges.
    pub fn blocker_tree(&self, root_id: Uuid) -> Option<WorkflowTreeNode> {
        let mut path = HashSet::new();
        self.build_blocker_tree_node(root_id, &mut path)
    }

    /// Return the frontier leaf ids for an upstream blocker tree.
    pub fn blocker_frontier_leaf_ids(&self, root_id: Uuid) -> Vec<Uuid> {
        self.blocker_tree(root_id)
            .map(|tree| tree.frontier_leaf_ids)
            .unwrap_or_default()
    }

    /// Build a downstream unlock tree while treating the supplied ids as satisfied.
    pub fn unlock_tree_with_satisfied(
        &self,
        root_id: Uuid,
        satisfied_ids: &HashSet<Uuid>,
    ) -> Option<WorkflowTreeNode> {
        let mut path = HashSet::new();
        self.build_unlock_tree_node(root_id, satisfied_ids, false, &mut path)
    }

    /// Return the frontier leaf ids for a downstream unlock tree while
    /// treating the supplied ids as satisfied.
    pub fn unlock_frontier_leaf_ids_with_satisfied(
        &self,
        root_id: Uuid,
        satisfied_ids: &HashSet<Uuid>,
    ) -> Vec<Uuid> {
        self.unlock_tree_with_satisfied(root_id, satisfied_ids)
            .map(|tree| tree.frontier_leaf_ids)
            .unwrap_or_default()
    }

    /// Build a downstream unlock tree while treating the root id as satisfied.
    pub fn unlock_tree(&self, root_id: Uuid) -> Option<WorkflowTreeNode> {
        let satisfied_ids = HashSet::from([root_id]);
        self.unlock_tree_with_satisfied(root_id, &satisfied_ids)
    }

    /// Return the frontier leaf ids for a downstream unlock tree while
    /// treating the root id as satisfied.
    pub fn unlock_frontier_leaf_ids(&self, root_id: Uuid) -> Vec<Uuid> {
        let satisfied_ids = HashSet::from([root_id]);
        self.unlock_frontier_leaf_ids_with_satisfied(root_id, &satisfied_ids)
    }

    /// Return the direct dependency-state inversions for one dependent ticket.
    pub fn dependency_state_inversions(
        &self,
        dependent_id: &Uuid,
    ) -> Option<&[DependencyStateInversion]> {
        self.inversions_by_dependent
            .get(dependent_id)
            .map(Vec::as_slice)
    }

    pub fn state_rank(&self, state: Option<&str>) -> usize {
        state
            .and_then(|value| self.state_index.get(value).copied())
            .unwrap_or(0)
    }
}
