use super::*;
use crate::error::StorageError;

// ── ticket f9e70385: plan freezing at `ready` (spec 24b3d22b) ────────────────

const PLANNING_KINDS: &[&str] = &[
    "objective",
    "requirements",
    "design",
    "examples",
    "acceptance_criteria",
];

#[path = "update_regression_tests/creation_and_migration.rs"]
mod creation_and_migration;
#[allow(unused_imports)]
pub use creation_and_migration::*;

#[path = "update_regression_tests/state_transitions.rs"]
mod state_transitions;
#[allow(unused_imports)]
pub use state_transitions::*;

#[path = "update_regression_tests/dependency_traversal.rs"]
mod dependency_traversal;
#[allow(unused_imports)]
pub use dependency_traversal::*;

#[path = "update_regression_tests/description_history.rs"]
mod description_history;
#[allow(unused_imports)]
pub use description_history::*;

#[path = "update_regression_tests/part_history.rs"]
mod part_history;
#[allow(unused_imports)]
pub use part_history::*;

#[path = "update_regression_tests/ready_freeze.rs"]
mod ready_freeze;
#[allow(unused_imports)]
pub use ready_freeze::*;

#[path = "update_regression_tests/ready_amendments.rs"]
mod ready_amendments;
#[allow(unused_imports)]
pub use ready_amendments::*;
