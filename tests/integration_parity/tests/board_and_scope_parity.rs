#[allow(unused_imports)]
use super::*;

// ── Board-aware next parity ───────────────────────────────────────────────────

/// HTTP and MCP must both apply the shared board-aware `next` semantics:
/// active board tickets leave `items`, appear in `excluded_by_board`, and
/// still surface board warnings.
#[tokio::test]
pub(super) async fn board_aware_next_parity_across_http_and_mcp() {
    let fx = ParityFixture::build();

    fx.store
        .board_configure(Some(BoardConfig {
            max_wip: 1,
            stale_after_secs: 3600,
            completed_audit_window_secs: 3600,
        }))
        .expect("configure board");

    let alpha_uuid: uuid::Uuid = fx.alpha_id.parse().expect("uuid");
    fx.store
        .board_check_in(
            &alpha_uuid,
            "parity-agent",
            3600,
            "in-flight work",
            vec!["parity.rs".to_string()],
            None,
            None,
            None,
        )
        .expect("board check-in");

    let (api_ids, api_excluded, api_warnings) = fx.api_board_filtered_candidates();
    assert_eq!(api_ids, vec![fx.beta_id.clone()]);
    assert_eq!(api_excluded, vec![fx.alpha_id.clone()]);
    assert!(
        api_warnings
            .iter()
            .any(|warning| warning.contains("WIP limit reached")),
        "ticket-api helper must surface WIP warning; got {api_warnings:?}"
    );

    // ── HTTP ──────────────────────────────────────────────────────────────
    let app = fx.http_router();
    let ws = &fx.workspace;
    let http = http_get_json(app, format!("/api/workflow/next?workspace={ws}")).await;
    let http_items = http["items"].as_array().expect("items");
    let http_ids: Vec<String> = http_items
        .iter()
        .map(|item| item["id"].as_str().unwrap_or("").to_owned())
        .collect();
    let http_excluded = http["excluded_by_board"].as_array().expect("excluded");
    let http_warning_strings: Vec<String> = http["warnings"]
        .as_array()
        .expect("warnings")
        .iter()
        .filter_map(|warning| warning.as_str().map(ToOwned::to_owned))
        .collect();
    assert_eq!(
        http_ids, api_ids,
        "HTTP visible items must match shared helper"
    );
    assert_eq!(
        http_excluded[0]["ticket_id"].as_str(),
        Some(fx.alpha_id.as_str()),
        "HTTP excluded_by_board must match shared helper"
    );
    assert!(
        http_warning_strings
            .iter()
            .any(|warning| warning.contains("WIP limit reached")),
        "HTTP warnings must include WIP limit warning; got {http_warning_strings:?}"
    );

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

    let mcp_items = mcp["items"].as_array().expect("items");
    let mcp_ids: Vec<String> = mcp_items
        .iter()
        .map(|item| item["id"].as_str().unwrap_or("").to_owned())
        .collect();
    let excluded = mcp["excluded_by_board"]
        .as_array()
        .expect("excluded_by_board");
    let mcp_warning_strings: Vec<String> = mcp["warnings"]
        .as_array()
        .expect("warnings")
        .iter()
        .filter_map(|warning| warning.as_str().map(ToOwned::to_owned))
        .collect();
    assert_eq!(
        mcp_ids, api_ids,
        "MCP visible items must match shared helper"
    );
    assert_eq!(
        excluded[0]["ticket_id"].as_str(),
        Some(fx.alpha_id.as_str()),
        "MCP excluded_by_board must match shared helper"
    );
    assert!(
        mcp_warning_strings
            .iter()
            .any(|warning| warning.contains("WIP limit reached")),
        "MCP warnings must include WIP limit warning; got {mcp_warning_strings:?}"
    );

    assert_eq!(http_ids, mcp_ids, "HTTP and MCP visible items must match");
    assert_eq!(
        http_excluded[0]["ticket_id"], excluded[0]["ticket_id"],
        "HTTP and MCP excluded_by_board must match"
    );
}

// ── scope metadata parity ──────────────────────────────────────────────────────

/// HTTP and MCP must both emit `scope.active_index_root` pointing to the
/// same store root path.
#[tokio::test]
pub(super) async fn scope_active_index_root_parity_http_and_mcp() {
    let fx = ParityFixture::build();

    // HTTP scope
    let app = fx.http_router();
    let ws = &fx.workspace;
    let http = http_get_json(app, format!("/api/workflow/next?workspace={ws}")).await;
    let http_index_root = http["scope"]["active_index_root"]
        .as_str()
        .expect("HTTP scope.active_index_root must be a string")
        .to_owned();
    assert!(
        !http_index_root.is_empty(),
        "HTTP scope.active_index_root must not be empty"
    );

    // MCP scope (embedded in next_tickets response)
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
    let mcp_index_root = mcp["scope"]["active_index_root"]
        .as_str()
        .expect("MCP scope.active_index_root must be a string")
        .to_owned();
    assert!(
        !mcp_index_root.is_empty(),
        "MCP scope.active_index_root must not be empty"
    );

    // Both must point to the same store (path may differ by separator style,
    // so normalise to forward slashes before comparing).
    let normalise = |p: &str| p.replace('\\', "/");
    assert_eq!(
        normalise(&http_index_root),
        normalise(&mcp_index_root),
        "HTTP and MCP scope.active_index_root must point to the same store root"
    );
}
