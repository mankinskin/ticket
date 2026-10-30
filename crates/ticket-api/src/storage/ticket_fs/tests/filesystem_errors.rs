#[allow(unused_imports)]
use super::*;

#[test]
pub(super) fn read_description_on_deleted_ticket_dir_names_path_and_creates_nothing() {
    let (_dir, ticket_path) = make_ticket_dir();
    std::fs::remove_dir_all(&ticket_path).unwrap();

    let err = TicketFs::read_description(&ticket_path).unwrap_err();

    assert_error_names(&err, &ticket_path);
    assert!(
        matches!(&err, crate::error::StorageError::Io(io) if io.kind() == std::io::ErrorKind::NotFound),
        "missing ticket dir must be a NotFound io error, got {err:?}"
    );
    assert!(
        !ticket_path.exists(),
        "read must not recreate the ticket dir"
    );
}

#[test]
pub(super) fn load_parts_on_deleted_ticket_dir_names_path_and_creates_nothing() {
    let (_dir, ticket_path) = make_ticket_dir();
    let manifest = TicketFs::read(&ticket_path).unwrap();
    std::fs::remove_dir_all(&ticket_path).unwrap();

    let err = TicketFs::load_parts(&ticket_path, &manifest).unwrap_err();

    assert_error_names(&err, &ticket_path);
    assert!(
        !ticket_path.exists(),
        "load_parts must not recreate the ticket dir"
    );
}

#[test]
pub(super) fn read_description_non_utf8_names_description_file() {
    let (_dir, ticket_path) = make_ticket_dir();
    let description = ticket_path.join("description.md");
    std::fs::write(&description, [0xff, 0xfe, 0x00, 0x80]).unwrap();

    let err = TicketFs::read_description(&ticket_path).unwrap_err();

    assert_error_names(&err, &description);
    assert!(
        matches!(&err, crate::error::StorageError::Io(io) if io.kind() == std::io::ErrorKind::InvalidData),
        "non-UTF-8 content must keep the InvalidData kind, got {err:?}"
    );
}

#[test]
pub(super) fn read_description_missing_file_is_still_none() {
    let (_dir, ticket_path) = make_ticket_dir();
    assert_eq!(TicketFs::read_description(&ticket_path).unwrap(), None);
}

#[test]
pub(super) fn legacy_load_parts_non_utf8_description_names_file() {
    let (_dir, ticket_path) = make_ticket_dir();
    let description = ticket_path.join("description.md");
    std::fs::write(&description, [0xc3, 0x28]).unwrap();
    let manifest = TicketFs::read(&ticket_path).unwrap();

    let err = TicketFs::load_parts(&ticket_path, &manifest).unwrap_err();

    assert_error_names(&err, &description);
}

#[test]
pub(super) fn load_parts_missing_part_file_names_part_path() {
    let (_dir, ticket_path) = make_ticket_dir();
    let part = TicketPart::new("design", "parts/design.md");
    let manifest = manifest_with_part(&ticket_path, &part);

    let err = TicketFs::load_parts(&ticket_path, &manifest).unwrap_err();

    assert_error_names(&err, &ticket_path.join(&part.path));
    assert!(
        matches!(&err, crate::error::StorageError::Io(io) if io.kind() == std::io::ErrorKind::NotFound),
        "missing part file must keep the NotFound kind, got {err:?}"
    );
}

#[test]
pub(super) fn load_parts_non_utf8_part_file_names_part_path() {
    let (_dir, ticket_path) = make_ticket_dir();
    let part = TicketPart::new("notes", "parts/notes.md");
    std::fs::create_dir_all(ticket_path.join("parts")).unwrap();
    std::fs::write(ticket_path.join(&part.path), [0x80, 0x81, 0x82]).unwrap();
    let manifest = manifest_with_part(&ticket_path, &part);

    let err = TicketFs::load_parts(&ticket_path, &manifest).unwrap_err();

    assert_error_names(&err, &ticket_path.join(&part.path));
}

#[test]
pub(super) fn ticket_path_that_is_a_file_is_rejected_with_path() {
    let dir = tempfile::tempdir().unwrap();
    let not_a_dir = dir.path().join("ticket-as-file");
    std::fs::write(&not_a_dir, "x").unwrap();

    let err = TicketFs::read_description(&not_a_dir).unwrap_err();

    assert_error_names(&err, &not_a_dir);
    assert_eq!(std::fs::read_to_string(&not_a_dir).unwrap(), "x");
}
