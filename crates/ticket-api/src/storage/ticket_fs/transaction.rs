use std::{
    collections::BTreeSet,
    fs::{self, OpenOptions},
    io::Write,
    path::{Component, Path},
};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::error::StorageError;

const JOURNAL_FILE: &str = ".ticket-transaction.json";
const GENERATION_FILE: &str = ".ticket-generation";
const JOURNAL_VERSION: u32 = 1;

// The generation marker is the durable commit point for content-only writes,
// whose manifest bytes can remain unchanged.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(super) enum Operation {
    Create,
    Overwrite,
    ManifestOnly,
    PathMigration,
    RemoveParts,
}

#[derive(Debug, Serialize, Deserialize)]
struct Journal {
    version: u32,
    transaction_id: Uuid,
    operation: Operation,
    old_generation: u64,
    new_generation: u64,
    changes: Vec<FileChange>,
}

#[derive(Debug, Serialize, Deserialize)]
struct FileChange {
    target: String,
    stage: Option<String>,
    backup: String,
    old_sha256: Option<String>,
    new_sha256: Option<String>,
}

struct PlannedChange {
    target: String,
    contents: Option<Vec<u8>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum FailPoint {
    JournalPublished,
    Staged(usize),
    BackedUp(usize),
    Published(usize),
}

#[cfg(test)]
thread_local! {
    static FAIL_AT: std::cell::Cell<Option<FailPoint>> = const { std::cell::Cell::new(None) };
}

#[cfg(test)]
#[path = "transaction/tests.rs"]
mod tests;

#[path = "transaction/publication.rs"]
mod publication;
#[allow(unused_imports)]
pub use publication::*;

#[path = "transaction/rollback.rs"]
mod rollback;
#[allow(unused_imports)]
pub use rollback::*;

#[path = "transaction/journal_validation.rs"]
mod journal_validation;
#[allow(unused_imports)]
pub use journal_validation::*;

#[path = "transaction/durability.rs"]
mod durability;
#[allow(unused_imports)]
pub use durability::*;
