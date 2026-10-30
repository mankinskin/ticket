#[allow(unused_imports)]
use super::*;

#[tokio::test]
pub(super) async fn workflow_next_candidates_parity_across_http_and_mcp() {
    let fx = ParityFixture::build();

    // ── ticket-api (canonical) ────────────────────────────────────────────
    let api_candidates = fx.api_next_candidates();
    assert!(
        api_candidates.contains(&fx.alpha_id),
        "ticket-api: alpha must be an actionable candidate; got {api_candidates:?}"
    );
    assert!(
        api_candidates.contains(&fx.beta_id),
        "ticket-api: beta must be an actionable candidate; got {api_candidates:?}"
    );
    // Newer candidate (beta) should sort before older (alpha) at equal priority.
    let alpha_pos = api_candidates
        .iter()
        .position(|id| id == &fx.alpha_id)
        .expect("alpha in api candidates");
    let beta_pos = api_candidates
        .iter()
        .position(|id| id == &fx.beta_id)
        .expect("beta in api candidates");
    assert!(
        beta_pos < alpha_pos,
        "ticket-api: beta (newer) must rank before alpha (older); \
         beta_pos={beta_pos} alpha_pos={alpha_pos}"
    );

    // ── HTTP ──────────────────────────────────────────────────────────────
    let app = fx.http_router();
    let ws = &fx.workspace;
    let http = http_get_json(app, format!("/api/workflow/next?workspace={ws}")).await;

    let http_items = http["items"].as_array().expect("items array in HTTP");
    let http_ids: Vec<String> = http_items
        .iter()
        .map(|item| item["id"].as_str().unwrap_or("").to_owned())
        .collect();

    assert!(
        http_ids.contains(&fx.alpha_id),
        "HTTP: alpha must appear in items; got {http_ids:?}"
    );
    assert!(
        http_ids.contains(&fx.beta_id),
        "HTTP: beta must appear in items; got {http_ids:?}"
    );
    let http_alpha_pos = http_ids.iter().position(|id| id == &fx.alpha_id).unwrap();
    let http_beta_pos = http_ids.iter().position(|id| id == &fx.beta_id).unwrap();
    assert!(
        http_beta_pos < http_alpha_pos,
        "HTTP: beta (newer) must rank before alpha (older); \
         http_beta_pos={http_beta_pos} http_alpha_pos={http_alpha_pos}"
    );

    // scope metadata must be present
    assert!(
        http["scope"]["active_index_root"].as_str().is_some(),
        "HTTP: scope.active_index_root must be present"
    );
    assert_eq!(http["scope"]["workspace"].as_str().unwrap(), ws.as_str());
    assert_eq!(http["excluded_by_board"], json!([]));
    assert_eq!(http["warnings"], json!([]));

    // ── MCP ───────────────────────────────────────────────────────────────
    let server = fx.mcp_server();
    let mcp_result = server
        .next_tickets(Parameters(NextTicketsInput {
            workspace: mcp_ws(),
            filter: None,
            root: None,
            limit: None,
        }))
        .await
        .expect("MCP next_tickets");
    let mcp_text = extract_text(&mcp_result);
    let mcp: Value = serde_json::from_str(&mcp_text).expect("valid JSON from MCP");

    let mcp_items = mcp["items"].as_array().expect("items array in MCP");
    let mcp_ids: Vec<String> = mcp_items
        .iter()
        .map(|item| item["id"].as_str().unwrap_or("").to_owned())
        .collect();

    assert!(
        mcp_ids.contains(&fx.alpha_id),
        "MCP: alpha must appear in items (no board entry); got {mcp_ids:?}"
    );
    assert!(
        mcp_ids.contains(&fx.beta_id),
        "MCP: beta must appear in items; got {mcp_ids:?}"
    );
    let mcp_alpha_pos = mcp_ids.iter().position(|id| id == &fx.alpha_id).unwrap();
    let mcp_beta_pos = mcp_ids.iter().position(|id| id == &fx.beta_id).unwrap();
    assert!(
        mcp_beta_pos < mcp_alpha_pos,
        "MCP: beta (newer) must rank before alpha (older); \
         mcp_beta_pos={mcp_beta_pos} mcp_alpha_pos={mcp_alpha_pos}"
    );
    assert_eq!(mcp["excluded_by_board"], json!([]));
    assert_eq!(mcp["warnings"], json!([]));

    // ── Cross-surface ordering agreement ─────────────────────────────────
    // HTTP and MCP must agree on alpha/beta relative order.
    assert_eq!(
        http_beta_pos < http_alpha_pos,
        mcp_beta_pos < mcp_alpha_pos,
        "HTTP and MCP must agree on beta-before-alpha ordering"
    );
    // gamma must not appear in any surface (it is blocked).
    let gamma_id = &fx._gamma_id;
    assert!(
        !http_ids.iter().any(|id| id == gamma_id),
        "HTTP: gamma (blocked) must not appear in items"
    );
    assert!(
        !mcp_ids.iter().any(|id| id == gamma_id),
        "MCP: gamma (blocked) must not appear in items"
    );
}

#[tokio::test]
pub(super) async fn workflow_next_root_scope_parity_across_http_and_mcp() {
    let fx = ParityFixture::build();

    let root = fx
        .store
        .create(
            None,
            "tracker-improvement",
            Some("[parity] Root unblock target"),
            Some("planned"),
            BTreeMap::new(),
            None,
            None,
        )
        .expect("create root");
    let direct = fx
        .store
        .create(
            None,
            "tracker-improvement",
            Some("[parity] Direct blocker"),
            Some("planned"),
            BTreeMap::new(),
            None,
            None,
        )
        .expect("create direct blocker");
    let intermediate = fx
        .store
        .create(
            None,
            "tracker-improvement",
            Some("[parity] Intermediate blocker"),
            Some("open"),
            BTreeMap::new(),
            None,
            None,
        )
        .expect("create intermediate blocker");
    let nested_leaf = fx
        .store
        .create(
            None,
            "tracker-improvement",
            Some("[parity] Nested leaf blocker"),
            Some("planned"),
            BTreeMap::new(),
            None,
            None,
        )
        .expect("create nested blocker");
    let unrelated = fx
        .store
        .create(
            None,
            "tracker-improvement",
            Some("[parity] Unrelated ready"),
            Some("planned"),
            BTreeMap::new(),
            None,
            None,
        )
        .expect("create unrelated");

    for (from, to) in [
        (root, direct),
        (root, intermediate),
        (intermediate, nested_leaf),
    ] {
        fx.store
            .add_edge(EdgeRecord {
                from,
                to,
                kind: String::from("depends_on"),
                created_at: chrono::Utc::now(),
            })
            .expect("add depends_on edge");
    }

    let tickets = fx.store.list(None, None, None).expect("list");
    let edges = fx.store.list_all_edges().expect("edges");
    let model = WorkflowModel::build(&fx.store, tickets, edges).expect("build model");
    let scope = model.root_blocker_scope(root).expect("root blocker scope");
    let mut expected_ids = model.actionable_candidate_ids(Some(&scope.remaining_blockers));
    model.sort_candidate_ids(&mut expected_ids);
    let expected_ids: Vec<String> = expected_ids.into_iter().map(|id| id.to_string()).collect();

    // HTTP
    let app = fx.http_router();
    let ws = &fx.workspace;
    let http = http_get_json(
        app,
        format!("/api/workflow/next?workspace={ws}&root={root}"),
    )
    .await;
    let http_ids: Vec<String> = http["items"]
        .as_array()
        .expect("http items")
        .iter()
        .map(|item| item["id"].as_str().unwrap_or("").to_owned())
        .collect();
    assert_eq!(
        http_ids, expected_ids,
        "HTTP root-scoped items must match API model"
    );
    assert_eq!(http["reachable_dependencies"], scope.reachable_dependencies);
    assert_eq!(http["blocked_dependencies"], scope.blocked_dependencies);
    assert_eq!(
        http["remaining_blocker_count"],
        scope.remaining_blockers.len()
    );
    assert_eq!(http["frontier_count"], expected_ids.len());
    assert_eq!(http["blocker_tree"]["id"], root.to_string());

    // MCP
    let server = fx.mcp_server();
    let mcp_result = server
        .next_tickets(Parameters(NextTicketsInput {
            workspace: mcp_ws(),
            filter: None,
            root: Some(root.to_string()),
            limit: None,
        }))
        .await
        .expect("MCP next_tickets root scope");
    let mcp_text = extract_text(&mcp_result);
    let mcp: Value = serde_json::from_str(&mcp_text).expect("valid JSON from MCP");
    let mcp_ids: Vec<String> = mcp["items"]
        .as_array()
        .expect("mcp items")
        .iter()
        .map(|item| item["id"].as_str().unwrap_or("").to_owned())
        .collect();

    assert_eq!(
        mcp_ids, expected_ids,
        "MCP root-scoped items must match API model"
    );
    assert_eq!(mcp["reachable_dependencies"], scope.reachable_dependencies);
    assert_eq!(mcp["blocked_dependencies"], scope.blocked_dependencies);
    assert_eq!(
        mcp["remaining_blocker_count"],
        scope.remaining_blockers.len()
    );
    assert_eq!(mcp["frontier_count"], expected_ids.len());
    assert_eq!(mcp["blocker_tree"]["id"], root.to_string());

    assert!(
        !http_ids.contains(&unrelated.to_string()),
        "root scope must exclude unrelated actionable tickets"
    );
    assert_eq!(
        http_ids, mcp_ids,
        "HTTP and MCP root-scoped next must match"
    );
}
