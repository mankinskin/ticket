#[allow(unused_imports)]
use super::*;

#[test]
pub(super) fn sort_candidates_prefers_newer_tickets_before_older_ones_without_pressure() {
    let older = ticket(
        "Older ticket",
        "planning",
        Utc.with_ymd_and_hms(2026, 5, 16, 12, 0, 0).unwrap(),
    );
    let newer = ticket(
        "Newer ticket",
        "planning",
        Utc.with_ymd_and_hms(2026, 5, 17, 12, 0, 0).unwrap(),
    );
    let mut candidates = vec![older.id, newer.id];
    let model = build_model(
        vec![older.clone(), newer.clone()],
        Vec::new(),
        HashMap::from([
            (older.id, "high".to_string()),
            (newer.id, "high".to_string()),
        ]),
    );

    model.sort_candidate_ids(&mut candidates);

    assert_eq!(candidates, vec![newer.id, older.id]);
}

#[test]
pub(super) fn sort_candidates_prefers_more_dependees_before_newer_tickets_without_pressure() {
    let older = ticket(
        "Older blocker",
        "planning",
        Utc.with_ymd_and_hms(2026, 5, 16, 12, 0, 0).unwrap(),
    );
    let newer = ticket(
        "Newer blocker",
        "planning",
        Utc.with_ymd_and_hms(2026, 5, 17, 12, 0, 0).unwrap(),
    );
    let dependent_one = ticket(
        "Dependent one",
        "planning",
        Utc.with_ymd_and_hms(2026, 5, 18, 12, 0, 0).unwrap(),
    );
    let dependent_two = ticket(
        "Dependent two",
        "planning",
        Utc.with_ymd_and_hms(2026, 5, 18, 12, 30, 0).unwrap(),
    );
    let now = Utc.with_ymd_and_hms(2026, 5, 18, 13, 0, 0).unwrap();
    let mut candidates = vec![newer.id, older.id];
    let model = build_model(
        vec![
            older.clone(),
            newer.clone(),
            dependent_one.clone(),
            dependent_two.clone(),
        ],
        vec![
            EdgeRecord {
                from: dependent_one.id,
                to: older.id,
                kind: "depends_on".to_string(),
                created_at: now,
            },
            EdgeRecord {
                from: dependent_two.id,
                to: older.id,
                kind: "depends_on".to_string(),
                created_at: now,
            },
        ],
        HashMap::from([
            (older.id, "high".to_string()),
            (newer.id, "high".to_string()),
        ]),
    );

    model.sort_candidate_ids(&mut candidates);

    assert_eq!(candidates, vec![older.id, newer.id]);
}

#[test]
pub(super) fn sort_candidates_prefers_more_recent_actionable_time_before_creation_time() {
    let older_created = ticket(
        "Older created but recently actionable",
        "planning",
        Utc.with_ymd_and_hms(2026, 5, 16, 12, 0, 0).unwrap(),
    );
    let newer_created = ticket(
        "Newer created but stale actionable",
        "planning",
        Utc.with_ymd_and_hms(2026, 5, 18, 12, 0, 0).unwrap(),
    );
    let recent_actionable_at = Utc.with_ymd_and_hms(2026, 5, 19, 12, 0, 0).unwrap();
    let stale_actionable_at = Utc.with_ymd_and_hms(2026, 5, 17, 12, 0, 0).unwrap();
    let mut candidates = vec![newer_created.id, older_created.id];
    let model = build_model_with_facts(
        vec![older_created.clone(), newer_created.clone()],
        Vec::new(),
        HashMap::from([
            (older_created.id, "high".to_string()),
            (newer_created.id, "high".to_string()),
        ]),
        HashMap::new(),
        HashMap::from([
            (
                older_created.id,
                WorkflowFacts {
                    unresolved_dependency_count: 0,
                    became_actionable_at: Some(recent_actionable_at),
                    last_blocker_progress_at: None,
                },
            ),
            (
                newer_created.id,
                WorkflowFacts {
                    unresolved_dependency_count: 0,
                    became_actionable_at: Some(stale_actionable_at),
                    last_blocker_progress_at: None,
                },
            ),
        ]),
    );

    model.sort_candidate_ids(&mut candidates);

    assert_eq!(candidates, vec![older_created.id, newer_created.id]);
    let metrics = model.metrics(&older_created.id).expect("metrics");
    assert_eq!(metrics.became_actionable_at, Some(recent_actionable_at));
}

#[test]
pub(super) fn convergence_pressure_promotes_earlier_state_prerequisite() {
    let prerequisite = ticket(
        "Lagging prerequisite",
        "planning",
        Utc.with_ymd_and_hms(2026, 5, 16, 12, 0, 0).unwrap(),
    );
    let unrelated = ticket(
        "Unrelated ready work",
        "planning",
        Utc.with_ymd_and_hms(2026, 5, 17, 12, 0, 0).unwrap(),
    );
    let dependent = ticket(
        "Advanced dependent",
        "in-review",
        Utc.with_ymd_and_hms(2026, 5, 18, 12, 0, 0).unwrap(),
    );
    let now = Utc.with_ymd_and_hms(2026, 5, 18, 13, 0, 0).unwrap();
    let mut candidates = vec![unrelated.id, prerequisite.id];
    let model = build_model(
        vec![prerequisite.clone(), unrelated.clone(), dependent.clone()],
        vec![EdgeRecord {
            from: dependent.id,
            to: prerequisite.id,
            kind: "depends_on".to_string(),
            created_at: now,
        }],
        HashMap::from([
            (prerequisite.id, "high".to_string()),
            (unrelated.id, "high".to_string()),
        ]),
    );

    model.sort_candidate_ids(&mut candidates);

    assert_eq!(candidates, vec![prerequisite.id, unrelated.id]);
    let metrics = model.metrics(&prerequisite.id).expect("metrics");
    assert_eq!(metrics.affected_reverse_dependent_reach, 1);
    assert_eq!(
        metrics.max_affected_dependent_state.as_deref(),
        Some("in-review")
    );
    assert_eq!(metrics.dependency_state_gap, 3);
}

#[test]
pub(super) fn convergence_pressure_still_beats_recently_actionable_unrelated_work() {
    let prerequisite = ticket(
        "Lagging prerequisite",
        "planning",
        Utc.with_ymd_and_hms(2026, 5, 16, 12, 0, 0).unwrap(),
    );
    let recently_actionable = ticket(
        "Recently actionable unrelated",
        "planning",
        Utc.with_ymd_and_hms(2026, 5, 18, 12, 0, 0).unwrap(),
    );
    let dependent = ticket(
        "Advanced dependent",
        "in-review",
        Utc.with_ymd_and_hms(2026, 5, 18, 13, 0, 0).unwrap(),
    );
    let now = Utc.with_ymd_and_hms(2026, 5, 18, 14, 0, 0).unwrap();
    let mut candidates = vec![recently_actionable.id, prerequisite.id];
    let model = build_model_with_facts(
        vec![
            prerequisite.clone(),
            recently_actionable.clone(),
            dependent.clone(),
        ],
        vec![EdgeRecord {
            from: dependent.id,
            to: prerequisite.id,
            kind: "depends_on".to_string(),
            created_at: now,
        }],
        HashMap::from([
            (prerequisite.id, "high".to_string()),
            (recently_actionable.id, "high".to_string()),
        ]),
        HashMap::new(),
        HashMap::from([(
            recently_actionable.id,
            WorkflowFacts {
                unresolved_dependency_count: 0,
                became_actionable_at: Some(now),
                last_blocker_progress_at: None,
            },
        )]),
    );

    model.sort_candidate_ids(&mut candidates);

    assert_eq!(candidates, vec![prerequisite.id, recently_actionable.id]);
}

#[test]
pub(super) fn sort_candidates_prefers_lower_effort_before_newer_tickets() {
    let lower_effort = ticket(
        "Lower effort ticket",
        "planning",
        Utc.with_ymd_and_hms(2026, 5, 16, 12, 0, 0).unwrap(),
    );
    let higher_effort = ticket(
        "Higher effort ticket",
        "planning",
        Utc.with_ymd_and_hms(2026, 5, 17, 12, 0, 0).unwrap(),
    );
    let mut candidates = vec![higher_effort.id, lower_effort.id];
    let model = build_model_with_facts(
        vec![lower_effort.clone(), higher_effort.clone()],
        Vec::new(),
        HashMap::from([
            (lower_effort.id, "high".to_string()),
            (higher_effort.id, "high".to_string()),
        ]),
        HashMap::from([(lower_effort.id, 1_200_u64), (higher_effort.id, 8_000_u64)]),
        HashMap::new(),
    );

    model.sort_candidate_ids(&mut candidates);

    assert_eq!(candidates, vec![lower_effort.id, higher_effort.id]);
}

#[test]
pub(super) fn parse_effort_accepts_numeric_token_budgets() {
    assert_eq!(parse_effort("1500"), Some(1_500));
    assert_eq!(parse_effort("2.5k tokens"), Some(2_500));
    assert_eq!(parse_effort("budget: 1_250"), Some(1_250));
    assert_eq!(parse_effort("unknown"), None);
}
