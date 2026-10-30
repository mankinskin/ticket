use std::collections::BTreeMap;

use serde_json::Value;

use super::{HistoryRevision, TicketFs};
use crate::model::ticket::{
    SpecRef, TicketManifest, TicketManifestExt, TicketPart, TicketRefEntry,
};

// ── ticket parts: model + storage read path (spec 24b3d22b, AC1-4, 7-9) ──────

fn make_ticket_dir() -> (tempfile::TempDir, std::path::PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let id = uuid::Uuid::new_v4();
    let ticket_path = dir.path().join(id.to_string());
    std::fs::create_dir_all(&ticket_path).unwrap();
    let manifest = TicketManifest::new(id, chrono::Utc::now());
    std::fs::write(
        ticket_path.join("ticket.toml"),
        crate::model::manifest_format::format_manifest_toml(&manifest),
    )
    .unwrap();
    (dir, ticket_path)
}

fn write_part_file(ticket_path: &std::path::Path, part: &TicketPart, content: &str) {
    std::fs::create_dir_all(ticket_path.join("parts")).unwrap();
    std::fs::write(ticket_path.join(&part.path), content).unwrap();
}

// ── regression: read-path errors must name the failing path (bugfix-3ce26d30 W3) ──

fn assert_error_names(err: &crate::error::StorageError, path: &std::path::Path) {
    let rendered = err.to_string();
    let needle = path.display().to_string();
    assert!(
        rendered.contains(&needle),
        "error `{rendered}` must name path `{needle}`"
    );
}

fn manifest_with_part(ticket_path: &std::path::Path, part: &TicketPart) -> TicketManifest {
    let mut manifest = TicketFs::read(ticket_path).unwrap();
    manifest.set_parts(vec![part.clone()]);
    std::fs::write(
        ticket_path.join("ticket.toml"),
        crate::model::manifest_format::format_manifest_toml(&manifest),
    )
    .unwrap();
    TicketFs::read(ticket_path).unwrap()
}

#[path = "tests/history_serialization.rs"]
mod history_serialization;
#[allow(unused_imports)]
pub use history_serialization::*;

#[path = "tests/parts_and_manifest.rs"]
mod parts_and_manifest;
#[allow(unused_imports)]
pub use parts_and_manifest::*;

#[path = "tests/reference_files.rs"]
mod reference_files;
#[allow(unused_imports)]
pub use reference_files::*;

#[path = "tests/filesystem_errors.rs"]
mod filesystem_errors;
#[allow(unused_imports)]
pub use filesystem_errors::*;
