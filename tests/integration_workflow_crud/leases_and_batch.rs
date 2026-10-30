#[allow(unused_imports)]
use super::*;

// ---------------------------------------------------------------------------
// Lease / claim / unclaim
// ---------------------------------------------------------------------------

#[test]
pub(super) fn claim_conflict_and_unclaim_cycle() {
    let s = Sandbox::new();
    let id = create_ticket(&s, "Work to claim");

    // Agent-1 claims the ticket successfully.
    let claim = s.ticket_json(&["claim", &id, "--agent", "agent-1", "--ttl-secs", "300"]);
    assert_eq!(claim["status"], "ok");
    assert_eq!(claim["working_by"], "agent-1");

    // Agent-2 attempts to claim the same ticket — must fail (lease conflict).
    let (_code, stderr) = s.ticket_fail(&["claim", &id, "--agent", "agent-2"]);
    assert!(
        stderr.contains("agent-1") || stderr.contains("lease") || stderr.contains("conflict"),
        "expected a lease-conflict error mentioning agent-1, got: {stderr}"
    );

    // The leases listing should show exactly one active lease.
    let leases = s.ticket_json(&["leases"]);
    assert_eq!(leases["count"].as_u64().unwrap(), 1);
    assert_eq!(leases["leases"][0]["working_by"], "agent-1");

    // Agent-1 releases the lease.
    let unclaim = s.ticket_json(&["unclaim", &id]);
    assert_eq!(unclaim["status"], "ok");

    // Agent-2 can now claim successfully.
    let reclaim = s.ticket_json(&["claim", &id, "--agent", "agent-2"]);
    assert_eq!(reclaim["status"], "ok");
    assert_eq!(reclaim["working_by"], "agent-2");
}

#[test]
pub(super) fn unclaim_clears_stale_orphaned_lease_without_board_entry() {
    let s = Sandbox::new();
    let id = create_ticket(&s, "Orphaned lease cleanup");

    let claim = s.ticket_json(&["claim", &id, "--agent", "agent-1", "--ttl-secs", "0"]);
    assert_eq!(claim["status"], "ok");

    let preview = s.ticket_json(&["board", "clean", "preview", "--include-stale"]);
    let token = preview["token"]
        .as_str()
        .expect("clean preview token must be present")
        .to_string();

    let apply = s.ticket_json(&["board", "clean", "apply", &token, "--include-stale"]);
    assert_eq!(apply["status"], "ok");

    let leases_before = s.ticket_json(&["leases"]);
    assert_eq!(leases_before["count"].as_u64().unwrap(), 1);

    let unclaim = s.ticket_json(&["unclaim", &id]);
    assert_eq!(unclaim["status"], "ok");

    let leases_after = s.ticket_json(&["leases"]);
    assert_eq!(leases_after["count"].as_u64().unwrap(), 0);
}

// ---------------------------------------------------------------------------
// Batch
// ---------------------------------------------------------------------------

#[test]
pub(super) fn batch_reads_cli_lines_from_stdin() {
    let s = Sandbox::new();

    let input = concat!(
        "create --title \"Batch stdin A\" --type tracker-improvement\n",
        "create --title \"Batch stdin B\" --type tracker-improvement",
    );
    let result = s.ticket_json_stdin(&["batch"], input);
    assert_eq!(result["status"], "ok");
    assert_eq!(result["count"].as_u64().unwrap(), 2);

    let list = s.ticket_json(&["list"]);
    assert_eq!(list["count"].as_u64().unwrap(), 2);
}

#[test]
pub(super) fn batch_cli_rolls_back_on_error() {
    let s = Sandbox::new();

    // First create succeeds; deleting a non-existent UUID fails; create is rolled back.
    let input = concat!(
        "create --title \"Should be rolled back\" --type tracker-improvement\n",
        "delete 00000000-0000-0000-0000-000000000000",
    );
    let result = s.ticket_json_stdin(&["batch"], input);
    assert_eq!(result["status"], "error");
    assert_eq!(result["completed"].as_u64().unwrap(), 1);
    assert_eq!(
        result["rolled_back"].as_bool().unwrap_or(false),
        true,
        "batch must report rolled_back=true after successful rollback"
    );

    let list = s.ticket_json(&["list"]);
    assert_eq!(
        list["count"].as_u64().unwrap(),
        0,
        "the rolled-back create must not appear in the store"
    );
}
