use std::{collections::HashSet, fs, path::Path, time::Instant};

use chrono::Utc;
use tracing::field::Empty;
use uuid::Uuid;

use crate::{
    error::StorageError,
    model::{
        edge::EdgeRecord,
        filesystem::{
            ParseDiagnostic, PersistedScanRoot, PolicyDecision, ScanRoot, ScanRootMetadata,
            ScanRootSource, TICKET_MANIFEST_FILE,
        },
    },
    storage::{
        index::RedbIndexStore,
        indexed::IndexedTicket,
        search::SearchDocumentInput,
        ticket_fs::{TicketFs, TicketScanEntry},
    },
};

use super::TicketStore;

const FILE_BACKED_EDGE_KINDS: &[&str] = &["depends_on", "linked"];
const STORE_TRACE_TARGET: &str = "ticket_api::storage::store";

#[derive(Debug, Clone, Default)]
pub struct ScanReport {
    pub integrated: usize,
    pub pruned: usize,
    pub diagnostics: Vec<ParseDiagnostic>,
    pub phase_timings_ms: std::collections::BTreeMap<String, u64>,
    pub root_entry_counts: std::collections::BTreeMap<String, usize>,
    /// Labels of scan roots skipped because policy marked them `ignored`.
    pub skipped_roots: Vec<String>,
}

impl TicketStore {}

#[path = "scan_helpers.rs"]
mod scan_helpers;
use scan_helpers::*;

#[path = "scan/scan_roots.rs"]
mod scan_roots;

#[path = "scan/scan_execution.rs"]
mod scan_execution;
