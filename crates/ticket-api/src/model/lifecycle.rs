use memory_kernel::lifecycle::{LifecycleError, LifecycleState, NativeLifecycle};

pub struct TicketLifecycle;

impl NativeLifecycle for TicketLifecycle {
    fn shared_state(&self, native_state: &str) -> Result<LifecycleState, LifecycleError> {
        let state = match native_state {
            "planning" | "ready" => LifecycleState::Planned,
            "in-implementation" | "in-review" => LifecycleState::Active,
            "on-hold" => LifecycleState::Paused,
            "blocked" => LifecycleState::Blocked,
            "done" => LifecycleState::Completed,
            "cancelled" => LifecycleState::Cancelled,
            unmapped => return Err(LifecycleError::StateUnmapped(unmapped.to_owned())),
        };
        Ok(state)
    }
}

#[cfg(test)]
mod tests {
    use memory_kernel::lifecycle::{LifecycleError, LifecycleState, NativeLifecycle};

    use super::TicketLifecycle;

    #[test]
    fn maps_ticket_states_to_shared_lifecycle_states() {
        let mappings = [
            ("planning", LifecycleState::Planned),
            ("ready", LifecycleState::Planned),
            ("in-implementation", LifecycleState::Active),
            ("in-review", LifecycleState::Active),
            ("on-hold", LifecycleState::Paused),
            ("blocked", LifecycleState::Blocked),
            ("done", LifecycleState::Completed),
            ("cancelled", LifecycleState::Cancelled),
        ];

        for (ticket_state, expected) in mappings {
            assert_eq!(TicketLifecycle.shared_state(ticket_state), Ok(expected));
        }
    }

    #[test]
    fn rejects_unmapped_ticket_states_without_normalizing_them() {
        assert_eq!(
            TicketLifecycle.shared_state("unknown"),
            Err(LifecycleError::StateUnmapped("unknown".to_owned()))
        );
        assert_eq!(
            TicketLifecycle.shared_state("Ready"),
            Err(LifecycleError::StateUnmapped("Ready".to_owned()))
        );
    }
}
