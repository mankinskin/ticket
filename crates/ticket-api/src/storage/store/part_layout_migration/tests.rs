use std::{fs, path::PathBuf};

use tempfile::TempDir;
use uuid::Uuid;

use crate::{
    model::{filesystem::ScanRoot, ticket::TicketManifestExt},
    storage::{TicketStore, ticket_fs::TicketFs},
};

use super::*;

type PartSpec<'a> = (&'a str, &'a str, bool, bool);

fn setup_store() -> (TempDir, TicketStore) {
    let directory = tempfile::tempdir().unwrap();
    let store = TicketStore::init(directory.path()).unwrap();
    (directory, store)
}

fn create_ticket_with_parts(
    store: &TicketStore,
    title: &str,
    specs: &[PartSpec<'_>],
) -> (Uuid, PathBuf, Vec<Uuid>, String) {
    let id = store
        .create(
            None,
            "tracker-improvement",
            Some(title),
            Some("planning"),
            Default::default(),
            None,
            None,
        )
        .unwrap();
    let mut ids = Vec::new();
    for (kind, content, _, _) in specs {
        let part_id = Uuid::new_v4();
        store.write_part(&id, part_id, kind, content, None).unwrap();
        ids.push(part_id);
    }

    let ticket_path = store.get_indexed(&id).unwrap().unwrap().path;
    let mut manifest = TicketFs::read(&ticket_path).unwrap();
    let mut parts = manifest.parts();
    for (index, part) in parts.iter_mut().enumerate() {
        let (kind, _, frozen, empty) = specs[index];
        assert_eq!(part.kind, kind);
        let opaque_path = format!("parts/{}.md", part.id);
        fs::rename(ticket_path.join(&part.path), ticket_path.join(&opaque_path)).unwrap();
        part.path = opaque_path.clone();
        part.frozen = frozen;
        if empty {
            fs::write(ticket_path.join(opaque_path), b"").unwrap();
        }
    }
    manifest.set_parts(parts);
    let manifest_contents = crate::model::manifest_format::format_manifest_toml(&manifest);
    fs::write(
        ticket_path.join("ticket.toml"),
        manifest_contents.as_bytes(),
    )
    .unwrap();
    let original_manifest = fs::read_to_string(ticket_path.join("ticket.toml")).unwrap();
    (id, ticket_path, ids, original_manifest)
}

#[path = "tests/planning_and_apply.rs"]
mod planning_and_apply;
#[allow(unused_imports)]
pub use planning_and_apply::*;

#[path = "tests/recovery_and_layouts.rs"]
mod recovery_and_layouts;
#[allow(unused_imports)]
pub use recovery_and_layouts::*;
