use rmcp::handler::server::wrapper::Parameters;
use serde_json::{Value, json};
use std::collections::BTreeMap;
use ticket::server::NextTicketsInput;
use ticket_api::{BoardConfig, model::edge::EdgeRecord, workflow::WorkflowModel};

use super::integration_parity_fixture::*;

fn extract_text(result: &rmcp::model::CallToolResult) -> String {
    result
        .content
        .iter()
        .find_map(|content| {
            if let rmcp::model::RawContent::Text(text) = &content.raw {
                Some(text.text.clone())
            } else {
                None
            }
        })
        .unwrap_or_default()
}

#[path = "tests/workflow_next_parity.rs"]
mod workflow_next_parity;
#[allow(unused_imports)]
pub use workflow_next_parity::*;

#[path = "tests/health_parity.rs"]
mod health_parity;
#[allow(unused_imports)]
pub use health_parity::*;

#[path = "tests/board_and_scope_parity.rs"]
mod board_and_scope_parity;
#[allow(unused_imports)]
pub use board_and_scope_parity::*;
