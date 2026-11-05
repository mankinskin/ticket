use std::{process::Stdio, time::Duration};

use serde_json::{Value, json};
use tempfile::TempDir;
use tokio::{
    io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader},
    process::Command,
    time::timeout,
};

#[tokio::test]
async fn ticket_mcp_discovery_supports_inline_tool_calls() {
    verify_discovery_and_tools(false).await;
}

#[tokio::test]
async fn ticket_mcp_discovery_preserves_legacy_initialize_and_tool_calls() {
    verify_discovery_and_tools(true).await;
}

#[tokio::test]
async fn ticket_mcp_startup_failure_logs_phase_and_elapsed_time() {
    let temp = TempDir::new().expect("temporary workspace");
    let index_root = temp.path().join(".workflow-tools").join("ticket");
    std::fs::create_dir_all(index_root.join("tickets.db")).expect("invalid fixture database");
    let output = timeout(
        Duration::from_secs(5),
        Command::new(env!("CARGO_BIN_EXE_ticket-mcp"))
            .env("TICKET_INDEX_ROOT", index_root)
            .env_remove("RUST_LOG")
            .kill_on_drop(true)
            .output(),
    )
    .await
    .expect("startup failure timed out")
    .expect("run ticket-mcp");
    assert!(!output.status.success());
    assert!(
        output.stdout.is_empty(),
        "diagnostics must not enter MCP stdout"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    for diagnostic in ["ticket_mcp_startup_failed", "phase", "open", "elapsed_ms"] {
        assert!(
            stderr.contains(diagnostic),
            "missing {diagnostic}: {stderr}"
        );
    }
}

async fn verify_discovery_and_tools(legacy: bool) {
    let temp = TempDir::new().expect("temporary workspace");
    let index_root = temp.path().join(".workflow-tools").join("ticket");
    let mut child = Command::new(env!("CARGO_BIN_EXE_ticket-mcp"))
        .env("TICKET_INDEX_ROOT", index_root)
        .env_remove("RUST_LOG")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true)
        .spawn()
        .expect("start ticket-mcp");
    let mut child_stderr = child.stderr.take().expect("child stderr");
    let stderr_task = tokio::spawn(async move {
        let mut stderr = String::new();
        child_stderr
            .read_to_string(&mut stderr)
            .await
            .expect("read stderr");
        stderr
    });
    let mut stdin = child.stdin.take().expect("child stdin");
    let stdout = child.stdout.take().expect("child stdout");
    let mut stdout = BufReader::new(stdout);

    let request = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "server/discover",
        "params": {
            "_meta": {
                "io.modelcontextprotocol/protocolVersion": "2026-07-28",
                "io.modelcontextprotocol/clientInfo": {
                    "name": "ticket-mcp-stdio-test",
                    "version": "0.1.0"
                },
                "io.modelcontextprotocol/clientCapabilities": {}
            }
        }
    });
    stdin
        .write_all(format!("{request}\n").as_bytes())
        .await
        .expect("write discovery request");

    let mut response_line = String::new();
    let bytes_read = timeout(Duration::from_secs(5), stdout.read_line(&mut response_line))
        .await
        .expect("ticket-mcp discovery response timed out")
        .expect("read discovery response");
    if bytes_read == 0 {
        let stderr = timeout(Duration::from_secs(5), stderr_task)
            .await
            .expect("ticket-mcp stderr timed out")
            .expect("stderr reader task");
        panic!("ticket-mcp closed stdout before responding to server/discover; stderr: {stderr}");
    }

    let response: Value = serde_json::from_str(&response_line).expect("discovery response is JSON");
    assert_eq!(response["id"], 1);
    assert!(
        response["result"]["supportedVersions"]
            .as_array()
            .is_some_and(|versions| versions.iter().any(|version| version == "2026-07-28")),
        "discovery response must advertise MCP 2026-07-28: {response}"
    );
    assert!(
        response["result"]["capabilities"]["tools"].is_object(),
        "discovery response must expose tool capability: {response}"
    );
    assert_eq!(
        response["result"]["_meta"]["io.modelcontextprotocol/serverInfo"]["name"],
        "ticket"
    );

    if legacy {
        let initialize = json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "initialize",
            "params": {
                "protocolVersion": "2025-11-25",
                "capabilities": {},
                "clientInfo": {"name": "ticket-mcp-legacy-test", "version": "0.1.0"}
            }
        });
        stdin
            .write_all(format!("{initialize}\n").as_bytes())
            .await
            .expect("write legacy initialize");
        response_line.clear();
        let bytes = timeout(Duration::from_secs(5), stdout.read_line(&mut response_line))
            .await
            .expect("initialize response timed out")
            .expect("read initialize response");
        assert_ne!(bytes, 0, "server closed before initialize response");
        let response: Value = serde_json::from_str(&response_line).expect("initialize JSON");
        assert_eq!(response["id"], 2);
        assert_eq!(response["result"]["protocolVersion"], "2025-11-25");
        stdin
            .write_all(b"{\"jsonrpc\":\"2.0\",\"method\":\"notifications/initialized\"}\n")
            .await
            .expect("write initialized notification");
    }

    for (id, method, mut params) in [
        (3, "tools/list", json!({})),
        (
            4,
            "tools/call",
            json!({"name": "ticket_capabilities", "arguments": {}}),
        ),
    ] {
        if !legacy {
            params["_meta"] = request["params"]["_meta"].clone();
        }
        let tool_request = json!({
            "jsonrpc": "2.0", "id": id, "method": method, "params": params
        });
        stdin
            .write_all(format!("{tool_request}\n").as_bytes())
            .await
            .expect("write tool request");
        response_line.clear();
        let bytes = timeout(Duration::from_secs(5), stdout.read_line(&mut response_line))
            .await
            .expect("tool response timed out")
            .expect("read tool response");
        assert_ne!(bytes, 0, "server closed before {method} response");
        let response: Value = serde_json::from_str(&response_line).expect("tool response JSON");
        assert_eq!(response["id"], id);
        assert!(
            response.get("error").is_none(),
            "{method} failed: {response}"
        );
        if method == "tools/list" {
            assert!(
                response["result"]["tools"]
                    .as_array()
                    .expect("tool list")
                    .iter()
                    .any(|tool| tool["name"] == "ticket_capabilities"),
                "capability tool missing: {response}"
            );
        } else {
            assert_ne!(response["result"]["isError"], true, "{response}");
            let text = response["result"]["content"][0]["text"]
                .as_str()
                .expect("capability text");
            let catalog: Value = serde_json::from_str(text).expect("capability catalog JSON");
            assert!(catalog["domains"].is_array(), "{catalog}");
        }
    }

    child.kill().await.expect("stop ticket-mcp");
    child.wait().await.expect("reap ticket-mcp");
    let stderr = timeout(Duration::from_secs(5), stderr_task)
        .await
        .expect("stderr reader timed out")
        .expect("stderr reader task");
    for event in [
        "ticket_mcp_startup_started",
        "ticket_mcp_store_open_complete",
        "ticket_workspace_policy_roots_ready",
        "ticket_store_scan_complete",
        "ticket_mcp_startup_complete",
        "ticket_mcp_protocol_ready",
        "phase_timings_ms",
        "updated_tickets",
        "handshake_ms",
    ] {
        assert!(
            stderr.contains(event),
            "missing startup diagnostic {event}: {stderr}"
        );
    }
}
