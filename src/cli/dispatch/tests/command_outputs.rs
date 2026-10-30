#[allow(unused_imports)]
use super::*;

#[test]
pub(super) fn dry_run_payload_is_returned_for_mutating_command() {
    let payload = dry_run_command_payload(&TicketCommandCli::Delete(IdArgs {
        id: Uuid::new_v4().to_string(),
        view: None,
        parts: None,
    }))
    .expect("delete should be dry-runnable");
    assert_eq!(payload["dry_run"], json!(true));
    assert_eq!(payload["command"], json!("delete"));
}

#[test]
pub(super) fn dry_run_payload_is_none_for_read_only_command() {
    let payload = dry_run_command_payload(&TicketCommandCli::Leases);
    assert!(payload.is_none());
}
