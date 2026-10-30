use std::{collections::BTreeMap, path::PathBuf};

use chrono::{TimeZone, Utc};

use super::*;
use crate::{BoardConfig, BoardEntry, BoardEntryStatus, BoardSnapshot};

fn ticket(title: &str, state: &str, created_at: chrono::DateTime<Utc>) -> IndexedTicket {
    IndexedTicket {
        id: Uuid::new_v4(),
        path: PathBuf::from(title),
        type_id: "tracker-improvement".to_string(),
        title: Some(title.to_string()),
        state: Some(state.to_string()),
        created_at,
        updated_at: created_at,
    }
}

fn build_model(
    tickets: Vec<IndexedTicket>,
    edges: Vec<EdgeRecord>,
    priorities: HashMap<Uuid, String>,
) -> WorkflowModel {
    build_model_with_facts(tickets, edges, priorities, HashMap::new(), HashMap::new())
}

fn build_model_with_facts(
    tickets: Vec<IndexedTicket>,
    edges: Vec<EdgeRecord>,
    priorities: HashMap<Uuid, String>,
    efforts: HashMap<Uuid, u64>,
    workflow_facts: HashMap<Uuid, WorkflowFacts>,
) -> WorkflowModel {
    WorkflowModel::build_from_parts(
        tickets,
        edges,
        HashMap::from([
            ("planning".to_string(), 0usize),
            ("ready".to_string(), 1usize),
            ("in-implementation".to_string(), 2usize),
            ("in-review".to_string(), 3usize),
            ("done".to_string(), 4usize),
        ]),
        priorities,
        efforts,
        workflow_facts,
    )
}

#[path = "tests/candidate_ranking.rs"]
mod candidate_ranking;
#[allow(unused_imports)]
pub use candidate_ranking::*;

#[path = "tests/dependency_and_board_views.rs"]
mod dependency_and_board_views;
#[allow(unused_imports)]
pub use dependency_and_board_views::*;
