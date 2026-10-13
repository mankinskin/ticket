use std::{fs, path::PathBuf};

use chrono::Utc;
use serde_json::Value;
use tempfile::TempDir;
use uuid::Uuid;

use crate::model::{
    filesystem::{TICKET_LOCK_FILE, TICKET_MANIFEST_FILE},
    manifest_format::format_manifest_toml,
    ticket::{TicketManifest, TicketManifestExt, TicketPart},
};

use super::{FailPoint, Operation, publish, set_failpoint};

#[derive(Clone, Copy)]
enum Scenario {
    Create,
    Overwrite,
    ManifestOnly,
    PathChange,
}

struct Fixture {
    _temp_dir: TempDir,
    ticket_path: PathBuf,
    old_manifest: TicketManifest,
    new_manifest: TicketManifest,
    old_path: Option<String>,
    new_path: Option<String>,
    old_content: Option<String>,
    new_content: Option<String>,
    writes: Vec<(String, Vec<u8>)>,
    removals: Vec<String>,
    operation: Operation,
}

#[test]
fn create_recovers_to_complete_old_or_new_state_at_every_publish_boundary() {
    assert_scenario(Scenario::Create);
}

#[test]
fn overwrite_recovers_to_complete_old_or_new_state_at_every_publish_boundary() {
    assert_scenario(Scenario::Overwrite);
}

#[test]
fn manifest_only_update_recovers_to_complete_old_or_new_state_at_every_publish_boundary() {
    assert_scenario(Scenario::ManifestOnly);
}

#[test]
fn path_change_recovers_to_complete_old_or_new_state_at_every_publish_boundary() {
    assert_scenario(Scenario::PathChange);
}

#[test]
fn recovery_preserves_a_foreign_target_instead_of_deleting_it() {
    let fixture = fixture(Scenario::Create);
    set_failpoint(Some(FailPoint::JournalPublished));
    let lock_path = fixture.ticket_path.join(TICKET_LOCK_FILE);
    let lock = super::super::acquire_lock(&lock_path).unwrap();
    let manifest_bytes = format_manifest_toml(&fixture.new_manifest);
    let result = publish(
        &fixture.ticket_path,
        fixture.operation,
        fixture.writes.clone(),
        fixture.removals.clone(),
        Some(manifest_bytes.as_bytes()),
    );
    assert!(super::super::release_lock(&lock, result).is_err());
    set_failpoint(None);

    write_part(
        &fixture.ticket_path,
        "parts/review.md",
        "created by another writer",
    );
    assert!(super::super::TicketFs::read(&fixture.ticket_path).is_err());
    assert_eq!(
        fs::read_to_string(fixture.ticket_path.join("parts/review.md")).unwrap(),
        "created by another writer"
    );
}

#[test]
fn reads_surface_a_malformed_journal_without_discarding_it() {
    let fixture = fixture(Scenario::ManifestOnly);
    let journal_path = fixture.ticket_path.join(".ticket-transaction.json");
    fs::write(&journal_path, b"{malformed").unwrap();

    assert!(super::super::TicketFs::read(&fixture.ticket_path).is_err());
    assert_eq!(fs::read(&journal_path).unwrap(), b"{malformed");
}

#[test]
fn create_refuses_to_replace_an_existing_unowned_part_path() {
    let fixture = fixture(Scenario::Create);
    write_part(
        &fixture.ticket_path,
        "parts/review.md",
        "pre-existing foreign content",
    );
    let manifest_bytes = format_manifest_toml(&fixture.new_manifest);
    let lock_path = fixture.ticket_path.join(TICKET_LOCK_FILE);
    let lock = super::super::acquire_lock(&lock_path).unwrap();
    let result = publish(
        &fixture.ticket_path,
        Operation::Create,
        fixture.writes.clone(),
        fixture.removals.clone(),
        Some(manifest_bytes.as_bytes()),
    );
    assert!(super::super::release_lock(&lock, result).is_err());
    assert_eq!(
        fs::read_to_string(fixture.ticket_path.join("parts/review.md")).unwrap(),
        "pre-existing foreign content"
    );
    assert_eq!(
        super::super::TicketFs::read(&fixture.ticket_path)
            .unwrap()
            .parts(),
        fixture.old_manifest.parts()
    );
    assert_no_transaction_artifacts(&fixture.ticket_path);
}

#[test]
fn recovery_restores_the_previous_generation_when_its_marker_is_backed_up() {
    let fixture = fixture(Scenario::Overwrite);
    let manifest_bytes = format_manifest_toml(&fixture.new_manifest);
    let lock_path = fixture.ticket_path.join(TICKET_LOCK_FILE);
    let lock = super::super::acquire_lock(&lock_path).unwrap();
    let first_result = publish(
        &fixture.ticket_path,
        Operation::Overwrite,
        fixture.writes.clone(),
        Vec::new(),
        Some(manifest_bytes.as_bytes()),
    );
    assert!(super::super::release_lock(&lock, first_result).is_ok());
    assert_eq!(
        fs::read_to_string(fixture.ticket_path.join(".ticket-generation")).unwrap(),
        "1\n"
    );

    set_failpoint(Some(FailPoint::BackedUp(2)));
    let lock = super::super::acquire_lock(&lock_path).unwrap();
    let second_result = publish(
        &fixture.ticket_path,
        Operation::Overwrite,
        vec![("parts/review.md".to_string(), b"second review".to_vec())],
        Vec::new(),
        Some(manifest_bytes.as_bytes()),
    );
    assert!(super::super::release_lock(&lock, second_result).is_err());
    set_failpoint(None);

    let snapshot = TicketFsSnapshot::read(&fixture.ticket_path);
    assert_eq!(snapshot.part_content.as_deref(), Some("new review"));
    assert_eq!(
        fs::read_to_string(fixture.ticket_path.join(".ticket-generation")).unwrap(),
        "1\n"
    );
    assert_no_transaction_artifacts(&fixture.ticket_path);
}

#[test]
fn load_parts_uses_the_manifest_read_under_its_recovery_lock() {
    let fixture = fixture(Scenario::PathChange);
    let stale_manifest = super::super::TicketFs::read(&fixture.ticket_path).unwrap();
    let manifest_bytes = format_manifest_toml(&fixture.new_manifest);
    let lock_path = fixture.ticket_path.join(TICKET_LOCK_FILE);
    let lock = super::super::acquire_lock(&lock_path).unwrap();
    let result = publish(
        &fixture.ticket_path,
        Operation::PathMigration,
        fixture.writes.clone(),
        fixture.removals.clone(),
        Some(manifest_bytes.as_bytes()),
    );
    assert!(super::super::release_lock(&lock, result).is_ok());

    let report = super::super::TicketFs::load_parts(&fixture.ticket_path, &stale_manifest).unwrap();
    assert_eq!(report.parts[0].path, "parts/after.md");
    assert_eq!(report.parts[0].content, "same review");
}

#[test]
fn path_change_can_reuse_paths_vacated_by_other_parts() {
    let temp_dir = tempfile::tempdir().unwrap();
    let ticket_id = Uuid::new_v4();
    let ticket_path = temp_dir.path().join(ticket_id.to_string());
    fs::create_dir_all(ticket_path.join("parts")).unwrap();
    let first = TicketPart::new("review", "parts/first.md");
    let second = TicketPart::new("review", "parts/second.md");
    write_part(&ticket_path, "parts/first.md", "first contents");
    write_part(&ticket_path, "parts/second.md", "second contents");

    let mut old_manifest = TicketManifest::new(ticket_id, Utc::now());
    old_manifest.set_parts(vec![first.clone(), second.clone()]);
    fs::write(
        ticket_path.join(TICKET_MANIFEST_FILE),
        format_manifest_toml(&old_manifest),
    )
    .unwrap();

    let mut moved_first = first;
    moved_first.path = "parts/second.md".to_string();
    let mut moved_second = second;
    moved_second.path = "parts/first.md".to_string();
    let mut new_manifest = old_manifest;
    new_manifest.set_parts(vec![moved_first, moved_second]);
    let manifest_bytes = format_manifest_toml(&new_manifest);

    let lock_path = ticket_path.join(TICKET_LOCK_FILE);
    let lock = super::super::acquire_lock(&lock_path).unwrap();
    let result = publish(
        &ticket_path,
        Operation::PathMigration,
        vec![
            ("parts/second.md".to_string(), b"first contents".to_vec()),
            ("parts/first.md".to_string(), b"second contents".to_vec()),
        ],
        vec!["parts/first.md".to_string(), "parts/second.md".to_string()],
        Some(manifest_bytes.as_bytes()),
    );
    assert!(super::super::release_lock(&lock, result).is_ok());

    let manifest = super::super::TicketFs::read(&ticket_path).unwrap();
    let report = super::super::TicketFs::load_parts(&ticket_path, &manifest).unwrap();
    assert_eq!(report.parts[0].path, "parts/second.md");
    assert_eq!(report.parts[0].content, "first contents");
    assert_eq!(report.parts[1].path, "parts/first.md");
    assert_eq!(report.parts[1].content, "second contents");
    assert!(report.orphans.is_empty());
}

fn assert_scenario(scenario: Scenario) {
    for failpoint in boundaries(scenario).into_iter().map(Some).chain([None]) {
        let fixture = fixture(scenario);
        let new_state_was_published = matches!(
            failpoint,
            Some(FailPoint::Published(index)) if index >= commit_index(scenario)
        ) || failpoint.is_none();
        set_failpoint(failpoint);
        let lock_path = fixture.ticket_path.join(TICKET_LOCK_FILE);
        let lock = super::super::acquire_lock(&lock_path).unwrap();
        let manifest_bytes = format_manifest_toml(&fixture.new_manifest);
        let result = publish(
            &fixture.ticket_path,
            fixture.operation,
            fixture.writes.clone(),
            fixture.removals.clone(),
            Some(manifest_bytes.as_bytes()),
        );
        let result = super::super::release_lock(&lock, result);
        set_failpoint(None);
        if failpoint.is_some() {
            assert!(
                result.is_err(),
                "injected boundary must interrupt publication"
            );
        } else {
            assert!(result.is_ok(), "uninterrupted publication must complete");
        }

        let actual = TicketFsSnapshot::read(&fixture.ticket_path);
        let expect_new = new_state_was_published;
        assert_eq!(
            actual.manifest,
            if expect_new {
                fixture.new_manifest.clone()
            } else {
                fixture.old_manifest.clone()
            }
        );

        let expected_path = if expect_new {
            fixture.new_path.as_deref()
        } else {
            fixture.old_path.as_deref()
        };
        let expected_content = if expect_new {
            fixture.new_content.as_deref()
        } else {
            fixture.old_content.as_deref()
        };
        assert_eq!(actual.part_path.as_deref(), expected_path);
        assert_eq!(actual.part_content.as_deref(), expected_content);
        assert!(actual.orphans.is_empty());
        assert_no_transaction_artifacts(&fixture.ticket_path);
    }
}

struct TicketFsSnapshot {
    manifest: TicketManifest,
    part_path: Option<String>,
    part_content: Option<String>,
    orphans: Vec<PathBuf>,
}

impl TicketFsSnapshot {
    fn read(ticket_path: &std::path::Path) -> Self {
        let manifest = super::super::TicketFs::read(ticket_path).unwrap();
        let report = super::super::TicketFs::load_parts(ticket_path, &manifest).unwrap();
        let part = report.parts.iter().find(|part| part.kind == "review");
        Self {
            manifest,
            part_path: part.map(|part| part.path.clone()),
            part_content: part.map(|part| part.content.clone()),
            orphans: report.orphans,
        }
    }
}

fn fixture(scenario: Scenario) -> Fixture {
    let temp_dir = tempfile::tempdir().unwrap();
    let id = Uuid::new_v4();
    let ticket_path = temp_dir.path().join(id.to_string());
    fs::create_dir_all(&ticket_path).unwrap();
    let mut old_manifest = TicketManifest::new(id, Utc::now());
    let mut new_manifest = old_manifest.clone();
    let (old_path, new_path, old_content, new_content, writes, removals, operation) = match scenario
    {
        Scenario::Create => {
            let part = TicketPart::new("review", "parts/review.md");
            new_manifest.set_parts(vec![part]);
            (
                None,
                Some("parts/review.md".to_string()),
                None,
                Some("new review".to_string()),
                vec![("parts/review.md".to_string(), b"new review".to_vec())],
                Vec::new(),
                Operation::Create,
            )
        }
        Scenario::Overwrite => {
            let part = TicketPart::new("review", "parts/review.md");
            old_manifest.set_parts(vec![part]);
            new_manifest = old_manifest.clone();
            write_part(&ticket_path, "parts/review.md", "old review");
            (
                Some("parts/review.md".to_string()),
                Some("parts/review.md".to_string()),
                Some("old review".to_string()),
                Some("new review".to_string()),
                vec![("parts/review.md".to_string(), b"new review".to_vec())],
                Vec::new(),
                Operation::Overwrite,
            )
        }
        Scenario::ManifestOnly => {
            old_manifest
                .extra
                .insert("title".to_string(), Value::String("before".to_string()));
            new_manifest = old_manifest.clone();
            new_manifest
                .extra
                .insert("title".to_string(), Value::String("after".to_string()));
            (
                None,
                None,
                None,
                None,
                Vec::new(),
                Vec::new(),
                Operation::ManifestOnly,
            )
        }
        Scenario::PathChange => {
            let part = TicketPart::new("review", "parts/before.md");
            old_manifest.set_parts(vec![part.clone()]);
            let mut moved_part = part;
            moved_part.path = "parts/after.md".to_string();
            new_manifest.set_parts(vec![moved_part]);
            write_part(&ticket_path, "parts/before.md", "same review");
            (
                Some("parts/before.md".to_string()),
                Some("parts/after.md".to_string()),
                Some("same review".to_string()),
                Some("same review".to_string()),
                vec![("parts/after.md".to_string(), b"same review".to_vec())],
                vec!["parts/before.md".to_string()],
                Operation::PathMigration,
            )
        }
    };

    fs::write(
        ticket_path.join(TICKET_MANIFEST_FILE),
        format_manifest_toml(&old_manifest),
    )
    .unwrap();
    Fixture {
        _temp_dir: temp_dir,
        ticket_path,
        old_manifest,
        new_manifest,
        old_path,
        new_path,
        old_content,
        new_content,
        writes,
        removals,
        operation,
    }
}

fn write_part(ticket_path: &std::path::Path, path: &str, content: &str) {
    let path = ticket_path.join(path);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, content).unwrap();
}

fn boundaries(scenario: Scenario) -> Vec<FailPoint> {
    let mut points = vec![FailPoint::JournalPublished];
    let (staged, backed_up, generation) = match scenario {
        Scenario::Create => (vec![0, 1, 2], vec![1], 2),
        Scenario::Overwrite => (vec![0, 1, 2], vec![0, 1], 2),
        Scenario::ManifestOnly => (vec![0, 1], vec![0], 1),
        Scenario::PathChange => (vec![0, 2, 3], vec![1, 2], 3),
    };
    points.extend(staged.iter().copied().map(FailPoint::Staged));
    points.extend(backed_up.into_iter().map(FailPoint::BackedUp));
    points.extend(staged.into_iter().map(FailPoint::Published));
    points
}

fn commit_index(scenario: Scenario) -> usize {
    match scenario {
        Scenario::Create => 1,
        Scenario::Overwrite => 2,
        Scenario::ManifestOnly => 0,
        Scenario::PathChange => 2,
    }
}

fn assert_no_transaction_artifacts(ticket_path: &std::path::Path) {
    let mut entries = fs::read_dir(ticket_path)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .filter(|name| name.starts_with(".ticket-txn-") || name == ".ticket-transaction.json")
        .collect::<Vec<_>>();
    if let Ok(parts) = fs::read_dir(ticket_path.join("parts")) {
        entries.extend(
            parts
                .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
                .filter(|name| name.starts_with(".ticket-txn-")),
        );
    }
    assert!(
        entries.is_empty(),
        "transaction artifacts remain: {entries:?}"
    );
}
