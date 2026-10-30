#[allow(unused_imports)]
use super::*;

// ── health findings parity ────────────────────────────────────────────────────

/// The `missing_description` check and its severity must be identical across
/// ticket-api, HTTP, and MCP when checked against the same fixture store.
#[tokio::test]
pub(super) async fn health_findings_parity_across_http_and_mcp() {
    let fx = ParityFixture::build();

    // ── ticket-api (canonical) ────────────────────────────────────────────
    let api_findings = fx.api_health_findings();
    // alpha has no description → exactly one missing_description warning
    let api_alpha_missing: Vec<_> = api_findings
        .iter()
        .filter(|(id, check, _sev)| id == &fx.alpha_id && check == "missing_description")
        .collect();
    assert_eq!(
        api_alpha_missing.len(),
        1,
        "ticket-api: alpha must have exactly one missing_description finding; got {api_findings:?}"
    );
    assert_eq!(
        api_alpha_missing[0].2, "warning",
        "ticket-api: missing_description severity must be 'warning'"
    );
    // beta has a good description → no missing_description
    let api_beta_missing: Vec<_> = api_findings
        .iter()
        .filter(|(id, check, _)| id == &fx.beta_id && check == "missing_description")
        .collect();
    assert!(
        api_beta_missing.is_empty(),
        "ticket-api: beta must not have missing_description finding; got {api_findings:?}"
    );

    // ── HTTP ──────────────────────────────────────────────────────────────
    let app = fx.http_router();
    let ws = &fx.workspace;
    let http = http_get_json(app, format!("/api/graph/health?workspace={ws}&all=true")).await;

    let http_findings = http["findings"].as_array().expect("findings array");
    let http_alpha_missing: Vec<_> = http_findings
        .iter()
        .filter(|f| {
            f["ticket_id"].as_str() == Some(&fx.alpha_id)
                && f["check"].as_str() == Some("missing_description")
        })
        .collect();
    assert_eq!(
        http_alpha_missing.len(),
        1,
        "HTTP: alpha must have exactly one missing_description finding; got {http_findings:?}"
    );
    assert_eq!(
        http_alpha_missing[0]["severity"].as_str().unwrap_or(""),
        "warning",
        "HTTP: missing_description severity must be 'warning'"
    );
    let http_beta_missing: Vec<_> = http_findings
        .iter()
        .filter(|f| {
            f["ticket_id"].as_str() == Some(&fx.beta_id)
                && f["check"].as_str() == Some("missing_description")
        })
        .collect();
    assert!(
        http_beta_missing.is_empty(),
        "HTTP: beta must not have missing_description finding; got {http_findings:?}"
    );

    // summary must count the finding (at least 1; gamma also lacks a description)
    assert!(
        http["summary"]["missing_description"].as_u64().unwrap_or(0) >= 1,
        "HTTP: summary.missing_description must be ≥ 1"
    );

    // ── MCP ───────────────────────────────────────────────────────────────
    let server = fx.mcp_server();
    let mcp_result = server
        .run_health_checks(
            &mcp_ws(),
            None, // root
            true, // all
            &[],  // ids
            None, // depth
            None, // direction
            &[],  // where
        )
        .await
        .expect("MCP run_health_checks");
    let mcp_text = extract_text(&mcp_result);
    let mcp: Value = serde_json::from_str(&mcp_text).expect("valid JSON from MCP");

    let mcp_findings = mcp["findings"].as_array().expect("findings array in MCP");
    let mcp_alpha_missing: Vec<_> = mcp_findings
        .iter()
        .filter(|f| {
            f["ticket_id"].as_str() == Some(&fx.alpha_id)
                && f["check"].as_str() == Some("missing_description")
        })
        .collect();
    assert_eq!(
        mcp_alpha_missing.len(),
        1,
        "MCP: alpha must have exactly one missing_description finding; got {mcp_findings:?}"
    );
    assert_eq!(
        mcp_alpha_missing[0]["severity"].as_str().unwrap_or(""),
        "warning",
        "MCP: missing_description severity must be 'warning'"
    );
    let mcp_beta_missing: Vec<_> = mcp_findings
        .iter()
        .filter(|f| {
            f["ticket_id"].as_str() == Some(&fx.beta_id)
                && f["check"].as_str() == Some("missing_description")
        })
        .collect();
    assert!(
        mcp_beta_missing.is_empty(),
        "MCP: beta must not have missing_description finding; got {mcp_findings:?}"
    );
    assert!(
        mcp["summary"]["missing_description"].as_u64().unwrap_or(0) >= 1,
        "MCP: summary.missing_description must be ≥ 1"
    );

    // ── Cross-surface finding agreement ───────────────────────────────────
    // Both HTTP and MCP must agree on the summary count.
    assert_eq!(
        http["summary"]["missing_description"], mcp["summary"]["missing_description"],
        "HTTP and MCP must agree on missing_description summary count"
    );
    // finding_count must be ≥ 1 on both surfaces.
    assert!(
        http["finding_count"].as_u64().unwrap_or(0) >= 1,
        "HTTP: finding_count must be ≥ 1"
    );
    assert!(
        mcp["finding_count"].as_u64().unwrap_or(0) >= 1,
        "MCP: finding_count must be ≥ 1"
    );
    assert_eq!(
        http["finding_count"], mcp["finding_count"],
        "HTTP and MCP must agree on total finding_count"
    );
}
