#[allow(unused_imports)]
use super::*;

#[test]
pub(super) fn reverse_dependents_collect_transitive_dependents() {
    let root = ticket(
        "Root",
        "planning",
        Utc.with_ymd_and_hms(2026, 5, 16, 12, 0, 0).unwrap(),
    );
    let direct = ticket(
        "Direct dependent",
        "planning",
        Utc.with_ymd_and_hms(2026, 5, 16, 12, 30, 0).unwrap(),
    );
    let transitive = ticket(
        "Transitive dependent",
        "planning",
        Utc.with_ymd_and_hms(2026, 5, 16, 13, 0, 0).unwrap(),
    );
    let now = Utc.with_ymd_and_hms(2026, 5, 16, 13, 30, 0).unwrap();
    let model = build_model(
        vec![root.clone(), direct.clone(), transitive.clone()],
        vec![
            EdgeRecord {
                from: direct.id,
                to: root.id,
                kind: "depends_on".to_string(),
                created_at: now,
            },
            EdgeRecord {
                from: transitive.id,
                to: direct.id,
                kind: "depends_on".to_string(),
                created_at: now,
            },
        ],
        HashMap::new(),
    );

    let dependents = model.reverse_dependents(root.id);

    assert!(dependents.contains(&direct.id));
    assert!(dependents.contains(&transitive.id));
}

#[test]
pub(super) fn dependency_state_inversions_capture_more_advanced_dependents() {
    let prerequisite = ticket(
        "Lagging prerequisite",
        "planning",
        Utc.with_ymd_and_hms(2026, 5, 16, 12, 0, 0).unwrap(),
    );
    let dependent = ticket(
        "Advanced dependent",
        "in-review",
        Utc.with_ymd_and_hms(2026, 5, 17, 12, 0, 0).unwrap(),
    );
    let now = Utc.with_ymd_and_hms(2026, 5, 18, 12, 0, 0).unwrap();
    let model = build_model(
        vec![prerequisite.clone(), dependent.clone()],
        vec![EdgeRecord {
            from: dependent.id,
            to: prerequisite.id,
            kind: "depends_on".to_string(),
            created_at: now,
        }],
        HashMap::new(),
    );

    let inversions = model
        .dependency_state_inversions(&dependent.id)
        .expect("dependency inversion");

    assert_eq!(inversions.len(), 1);
    assert_eq!(inversions[0].prerequisite_id, prerequisite.id);
    assert_eq!(inversions[0].dependent_id, dependent.id);
    assert_eq!(inversions[0].dependency_state_gap, 3);
    assert_eq!(inversions[0].affected_reverse_dependent_reach, 1);
}

#[test]
pub(super) fn blocker_tree_preserves_nested_children_and_orders_closest_frontier_first() {
    let root = ticket(
        "Root",
        "planning",
        Utc.with_ymd_and_hms(2026, 5, 16, 12, 0, 0).unwrap(),
    );
    let direct_leaf = ticket(
        "Direct frontier leaf",
        "planning",
        Utc.with_ymd_and_hms(2026, 5, 16, 12, 5, 0).unwrap(),
    );
    let nested_parent = ticket(
        "Nested parent",
        "planning",
        Utc.with_ymd_and_hms(2026, 5, 16, 12, 10, 0).unwrap(),
    );
    let nested_leaf = ticket(
        "Nested frontier leaf",
        "planning",
        Utc.with_ymd_and_hms(2026, 5, 16, 12, 15, 0).unwrap(),
    );
    let now = Utc.with_ymd_and_hms(2026, 5, 16, 13, 0, 0).unwrap();
    let model = build_model(
        vec![
            root.clone(),
            direct_leaf.clone(),
            nested_parent.clone(),
            nested_leaf.clone(),
        ],
        vec![
            EdgeRecord {
                from: root.id,
                to: nested_parent.id,
                kind: "depends_on".to_string(),
                created_at: now,
            },
            EdgeRecord {
                from: root.id,
                to: direct_leaf.id,
                kind: "depends_on".to_string(),
                created_at: now,
            },
            EdgeRecord {
                from: nested_parent.id,
                to: nested_leaf.id,
                kind: "depends_on".to_string(),
                created_at: now,
            },
        ],
        HashMap::new(),
    );

    let tree = model.blocker_tree(root.id).expect("blocker tree");

    assert_eq!(tree.remaining_blocker_count, 2);
    assert_eq!(
        tree.children
            .iter()
            .map(|child| child.ticket_id)
            .collect::<Vec<_>>(),
        vec![direct_leaf.id, nested_parent.id]
    );
    assert_eq!(tree.frontier_leaf_ids, vec![direct_leaf.id, nested_leaf.id]);
    assert_eq!(tree.unresolved_frontier_leaf_count, 2);
    assert_eq!(tree.blocker_distance, 1);
    assert!(tree.children[0].is_frontier);
    assert!(!tree.children[1].is_frontier);
    assert_eq!(tree.children[1].children.len(), 1);
    assert_eq!(tree.children[1].children[0].ticket_id, nested_leaf.id);
    assert!(tree.children[1].children[0].is_frontier);
}

#[test]
pub(super) fn apply_board_filter_excludes_tracked_candidates_and_surfaces_warnings() {
    let active_candidate = Uuid::new_v4();
    let free_candidate = Uuid::new_v4();
    let now = Utc.with_ymd_and_hms(2026, 6, 2, 10, 0, 0).unwrap();
    let snapshot = BoardSnapshot {
        captured_at: now,
        entries: vec![BoardEntry {
            entry_id: Uuid::new_v4(),
            ticket_id: active_candidate,
            agent_id: "parity-agent".to_string(),
            previous_attempt: None,
            checked_in_at: now,
            last_heartbeat: now,
            ttl_secs: 3600,
            intent: "in flight".to_string(),
            owned_files: Vec::new(),
            status: BoardEntryStatus::Active,
            handoff_reason: None,
            completed_at: None,
            session_id: None,
            worktree_path: None,
            branch: None,
        }],
        caller_entries: Vec::new(),
        config: BoardConfig {
            max_wip: 1,
            stale_after_secs: 3600,
            completed_audit_window_secs: 3600,
        },
        active_count: 1,
        stale_count: 0,
        conflict_count: 0,
        wip_limit_reached: true,
        file_ownership: BTreeMap::new(),
        active_worktrees: Vec::new(),
        warnings: Vec::new(),
    };

    let result = apply_board_filter(
        vec![active_candidate, free_candidate],
        Some(&snapshot),
        false,
    );

    assert_eq!(result.candidates, vec![free_candidate]);
    assert_eq!(result.excluded_by_board.len(), 1);
    assert_eq!(result.excluded_by_board[0].ticket_id, active_candidate);
    assert_eq!(result.excluded_by_board[0].agent_id, "parity-agent");
    assert_eq!(result.excluded_by_board[0].status, "active");
    assert!(
        result
            .warnings
            .iter()
            .any(|warning| warning.contains("WIP limit reached")),
        "expected WIP warning, got {:?}",
        result.warnings
    );
}

#[test]
pub(super) fn apply_board_filter_respects_skip_board_but_keeps_warnings() {
    let tracked_candidate = Uuid::new_v4();
    let now = Utc.with_ymd_and_hms(2026, 6, 2, 10, 0, 0).unwrap();
    let snapshot = BoardSnapshot {
        captured_at: now,
        entries: vec![BoardEntry {
            entry_id: Uuid::new_v4(),
            ticket_id: tracked_candidate,
            agent_id: "parity-agent".to_string(),
            previous_attempt: None,
            checked_in_at: now,
            last_heartbeat: now,
            ttl_secs: 3600,
            intent: "in flight".to_string(),
            owned_files: Vec::new(),
            status: BoardEntryStatus::Stale,
            handoff_reason: None,
            completed_at: None,
            session_id: None,
            worktree_path: None,
            branch: None,
        }],
        caller_entries: Vec::new(),
        config: BoardConfig {
            max_wip: 5,
            stale_after_secs: 3600,
            completed_audit_window_secs: 3600,
        },
        active_count: 0,
        stale_count: 1,
        conflict_count: 0,
        wip_limit_reached: false,
        file_ownership: BTreeMap::new(),
        active_worktrees: Vec::new(),
        warnings: Vec::new(),
    };

    let result = apply_board_filter(vec![tracked_candidate], Some(&snapshot), true);

    assert_eq!(result.candidates, vec![tracked_candidate]);
    assert!(result.excluded_by_board.is_empty());
    assert!(
        result
            .warnings
            .iter()
            .any(|warning| warning.contains("stale board entry")),
        "expected stale warning, got {:?}",
        result.warnings
    );
}

#[test]
pub(super) fn unlock_tree_marks_actionable_parents_as_frontier_and_preserves_children() {
    let root = ticket(
        "Satisfied prerequisite",
        "done",
        Utc.with_ymd_and_hms(2026, 5, 16, 12, 0, 0).unwrap(),
    );
    let actionable_parent = ticket(
        "Actionable parent",
        "planning",
        Utc.with_ymd_and_hms(2026, 5, 16, 12, 5, 0).unwrap(),
    );
    let blocked_parent = ticket(
        "Blocked parent",
        "planning",
        Utc.with_ymd_and_hms(2026, 5, 16, 12, 10, 0).unwrap(),
    );
    let external_blocker = ticket(
        "External blocker",
        "planning",
        Utc.with_ymd_and_hms(2026, 5, 16, 12, 15, 0).unwrap(),
    );
    let grandchild = ticket(
        "Grandchild",
        "planning",
        Utc.with_ymd_and_hms(2026, 5, 16, 12, 20, 0).unwrap(),
    );
    let now = Utc.with_ymd_and_hms(2026, 5, 16, 13, 0, 0).unwrap();
    let model = build_model(
        vec![
            root.clone(),
            actionable_parent.clone(),
            blocked_parent.clone(),
            external_blocker.clone(),
            grandchild.clone(),
        ],
        vec![
            EdgeRecord {
                from: actionable_parent.id,
                to: root.id,
                kind: "depends_on".to_string(),
                created_at: now,
            },
            EdgeRecord {
                from: blocked_parent.id,
                to: root.id,
                kind: "depends_on".to_string(),
                created_at: now,
            },
            EdgeRecord {
                from: blocked_parent.id,
                to: external_blocker.id,
                kind: "depends_on".to_string(),
                created_at: now,
            },
            EdgeRecord {
                from: grandchild.id,
                to: actionable_parent.id,
                kind: "depends_on".to_string(),
                created_at: now,
            },
        ],
        HashMap::new(),
    );

    let tree = model.unlock_tree(root.id).expect("unlock tree");

    assert_eq!(
        tree.children
            .iter()
            .map(|child| child.ticket_id)
            .collect::<Vec<_>>(),
        vec![actionable_parent.id, blocked_parent.id]
    );

    let actionable = &tree.children[0];
    assert!(actionable.is_frontier);
    assert_eq!(actionable.frontier_leaf_ids, vec![actionable_parent.id]);
    assert_eq!(actionable.blocker_distance, 0);
    assert_eq!(actionable.children.len(), 1);
    assert_eq!(actionable.children[0].ticket_id, grandchild.id);

    let blocked = &tree.children[1];
    assert!(!blocked.is_frontier);
    assert_eq!(blocked.remaining_blocker_count, 1);
    assert_eq!(blocked.frontier_leaf_ids, vec![blocked_parent.id]);
    assert_eq!(blocked.blocker_distance, 1);
    assert_eq!(
        model.unlock_frontier_leaf_ids(root.id),
        tree.frontier_leaf_ids
    );
    assert_eq!(model.blocker_frontier_leaf_ids(root.id), vec![root.id]);
}
