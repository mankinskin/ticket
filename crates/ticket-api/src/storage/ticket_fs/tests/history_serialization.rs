#[allow(unused_imports)]
use super::*;

#[test]
pub(super) fn history_revision_backward_compat_no_author() {
    let json = r#"{"rev":1,"ts":"2025-01-01T00:00:00Z","fields":{"state":"planning","title":"Old entry"}}"#;
    let rev: HistoryRevision = serde_json::from_str(json)
        .expect("should deserialize legacy revision without author field");
    assert_eq!(rev.rev, 1);
    assert_eq!(rev.author, None, "author should be None for legacy entries");
}

#[test]
pub(super) fn history_revision_with_author() {
    let json = r#"{"rev":2,"ts":"2025-01-02T00:00:00Z","fields":{},"author":"alice"}"#;
    let rev: HistoryRevision =
        serde_json::from_str(json).expect("should deserialize revision with author");
    assert_eq!(rev.author, Some("alice".to_string()));
}

#[test]
pub(super) fn history_revision_none_author_is_skipped_in_serialization() {
    let rev = HistoryRevision {
        rev: 1,
        ts: "2025-01-01T00:00:00Z".to_string(),
        fields: BTreeMap::new(),
        author: None,
    };
    let json = serde_json::to_string(&rev).expect("serialize");
    let value: Value = serde_json::from_str(&json).unwrap();
    assert!(
        value.get("author").is_none(),
        "author key should be absent when None"
    );
}

#[test]
pub(super) fn read_history_skips_malformed_line_instead_of_erroring() {
    let dir = tempfile::tempdir().unwrap();
    let ticket_path = dir.path().join("ticket");
    std::fs::create_dir_all(&ticket_path).unwrap();
    std::fs::write(
        ticket_path.join("history.ndjson"),
        "[]\n{\"rev\":1,\"ts\":\"2025-01-01T00:00:00Z\",\"fields\":{}}\n",
    )
    .unwrap();

    let revisions =
        TicketFs::read_history(&ticket_path).expect("a malformed line must not fail the read");
    assert_eq!(revisions.len(), 1, "the one valid revision should survive");
    assert_eq!(revisions[0].rev, 1);
}

#[test]
pub(super) fn append_history_still_works_after_a_malformed_line() {
    let dir = tempfile::tempdir().unwrap();
    let ticket_path = dir.path().join("ticket");
    std::fs::create_dir_all(&ticket_path).unwrap();
    // Simulates the real corruption found in production: a stray `[]` line.
    std::fs::write(ticket_path.join("history.ndjson"), "[]\n").unwrap();

    let rev = TicketFs::append_history(&ticket_path, BTreeMap::new(), None).expect(
        "append must still succeed even though history.ndjson has a \
             malformed line",
    );
    assert_eq!(
        rev, 1,
        "malformed line is skipped, not counted as a revision"
    );

    let revisions = TicketFs::read_history(&ticket_path).unwrap();
    assert_eq!(
        revisions.len(),
        1,
        "the newly appended revision is readable"
    );
}
