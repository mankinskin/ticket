# Objective
Fix `ticket batch` so multi-edge link creation completes or fails with a bounded error.

# Context
During creation of epic `1b58aaf5`, both stdin and `--file` batch forms stalled indefinitely after partially persisting containment links. The partial-write non-atomicity is the more serious defect because the command is documented as transactional.

# Acceptance Criteria
- Batch either completes or fails with a bounded error.
- A failed batch rolls back partial application.
- A regression test covers a multi-edge batch.

# Out of Scope
Repairing epic `1b58aaf5` graph links, which is being completed manually.