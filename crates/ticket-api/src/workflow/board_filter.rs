#[allow(unused_imports)]
use super::*;

/// Apply board-awareness on top of already-ranked actionable workflow candidates.
///
/// The returned `candidates` preserve the input order, removing tickets covered
/// by active or stale board entries unless `skip_board` is `true`.
/// Excluded tickets are surfaced separately so callers can still explain why a
/// candidate disappeared from `items`.
pub fn apply_board_filter(
    candidates: Vec<Uuid>,
    board_snap: Option<&BoardSnapshot>,
    skip_board: bool,
) -> BoardAwareCandidates {
    let warnings = board_warnings(board_snap);

    if skip_board {
        return BoardAwareCandidates {
            candidates,
            excluded_by_board: Vec::new(),
            warnings,
        };
    }

    let Some(snapshot) = board_snap else {
        return BoardAwareCandidates {
            candidates,
            excluded_by_board: Vec::new(),
            warnings,
        };
    };

    let candidate_ids: HashSet<Uuid> = candidates.iter().copied().collect();
    let excluded_by_board = snapshot
        .entries
        .iter()
        .filter(|entry| tracked_by_board(&entry.status) && candidate_ids.contains(&entry.ticket_id))
        .map(|entry| BoardExcludedCandidate {
            ticket_id: entry.ticket_id,
            agent_id: entry.agent_id.clone(),
            status: board_status(&entry.status).to_string(),
            intent: entry.intent.clone(),
        })
        .collect::<Vec<_>>();

    let blocked_ids: HashSet<Uuid> = snapshot
        .entries
        .iter()
        .filter(|entry| tracked_by_board(&entry.status))
        .map(|entry| entry.ticket_id)
        .collect();

    BoardAwareCandidates {
        candidates: candidates
            .into_iter()
            .filter(|ticket_id| !blocked_ids.contains(ticket_id))
            .collect(),
        excluded_by_board,
        warnings,
    }
}

pub(super) fn board_warnings(board_snap: Option<&BoardSnapshot>) -> Vec<String> {
    let Some(snapshot) = board_snap else {
        return Vec::new();
    };

    let mut warnings = Vec::new();
    let max_wip = snapshot.config.max_wip;

    if snapshot.active_count >= max_wip {
        warnings.push(format!(
            "WIP limit reached: {}/{} active entries — pause new work and reduce the board.",
            snapshot.active_count, max_wip
        ));
    } else if max_wip > 0 && snapshot.active_count + 1 >= max_wip {
        warnings.push(format!(
            "Approaching WIP limit: {}/{} active entries.",
            snapshot.active_count, max_wip
        ));
    }

    if snapshot.stale_count > 0 {
        warnings.push(format!(
            "{} stale board entr{} — heartbeat has expired; run board heartbeat or clean.",
            snapshot.stale_count,
            if snapshot.stale_count == 1 {
                "y"
            } else {
                "ies"
            }
        ));
    }

    warnings
}

pub(super) fn tracked_by_board(status: &BoardEntryStatus) -> bool {
    matches!(status, BoardEntryStatus::Active | BoardEntryStatus::Stale)
}

pub(super) fn board_status(status: &BoardEntryStatus) -> &'static str {
    match status {
        BoardEntryStatus::Active => "active",
        BoardEntryStatus::Stale => "stale",
        BoardEntryStatus::Conflict => "conflict",
        BoardEntryStatus::Completed => "completed",
    }
}
