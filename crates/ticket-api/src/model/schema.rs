pub use memory_kernel::model::schema::*;
// Backward-compatible alias: downstream code uses TicketTypeSchema.
pub use memory_kernel::model::schema::EntityTypeSchema as TicketTypeSchema;

pub trait TicketTypeSchemaExt {
    fn entry_state(&self) -> Option<&str>;
}

impl TicketTypeSchemaExt for TicketTypeSchema {
    fn entry_state(&self) -> Option<&str> {
        self.states
            .iter()
            .find(|state| state.as_str() == "planning")
            .map(String::as_str)
            .or_else(|| memory_kernel::lifecycle::LifecycleDefinition::entry_state(self))
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::{TicketTypeSchema, TicketTypeSchemaExt};

    fn schema(states: &[&str]) -> TicketTypeSchema {
        TicketTypeSchema {
            type_id: "ticket".to_owned(),
            fields: BTreeMap::new(),
            states: states.iter().map(|state| (*state).to_owned()).collect(),
            transitions: Vec::new(),
            edge_rules: BTreeMap::new(),
            required_states: Vec::new(),
            terminal_states: vec!["done".to_owned()],
        }
    }

    #[test]
    fn entry_state_prefers_ticket_planning_state_over_schema_default() {
        assert_eq!(
            schema(&["ready", "planning"]).entry_state(),
            Some("planning")
        );
    }

    #[test]
    fn entry_state_falls_back_to_kernel_schema_default() {
        assert_eq!(schema(&["ready", "done"]).entry_state(), Some("ready"));
        assert_eq!(schema(&[]).entry_state(), None);
    }
}
