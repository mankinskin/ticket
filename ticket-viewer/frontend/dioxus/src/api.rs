//! `TicketBackend` trait and its `HttpTicketBackend` implementation.
//!
//! The trait abstracts the REST API surface served by `ticket-viewer` /
//! `ticket-http`. The HTTP implementation calls those endpoints using the
//! browser Fetch API via `gloo-net`.

use percent_encoding::{
    utf8_percent_encode,
    NON_ALPHANUMERIC,
};
use serde::Deserialize;

mod backend;

pub use self::backend::TicketBackend;

use crate::types::*;

// ── HTTP implementation ───────────────────────────────────────────────────────

/// HTTP client that targets the running `ticket-viewer` server.
/// All paths are relative so they work both in dev (`trunk serve` proxy) and
/// in production (same-origin deployment).
#[derive(Clone)]
pub struct HttpTicketBackend {
    /// Optional bearer token forwarded as `Authorization: Bearer <token>`.
    pub token: Option<String>,
}

impl HttpTicketBackend {
    pub fn new(token: Option<String>) -> Self {
        Self { token }
    }

    /// Returns `true` when a non-empty auth token is stored in `sessionStorage`
    /// under the key `ticketViewerToken`.
    pub fn has_auth_token() -> bool {
        #[cfg(target_arch = "wasm32")]
        {
            web_sys::window()
                .and_then(|w| w.session_storage().ok().flatten())
                .and_then(|s| s.get_item("ticketViewerToken").ok().flatten())
                .map(|t| !t.is_empty())
                .unwrap_or(false)
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            false
        }
    }

    /// Reads the auth token from `sessionStorage`, returning `None` when absent
    /// or empty.
    pub fn read_auth_token() -> Option<String> {
        #[cfg(target_arch = "wasm32")]
        {
            web_sys::window()
                .and_then(|w| w.session_storage().ok().flatten())
                .and_then(|s| s.get_item("ticketViewerToken").ok().flatten())
                .filter(|t| !t.is_empty())
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            None
        }
    }

    async fn fetch<T: for<'de> Deserialize<'de>>(
        &self,
        url: &str,
    ) -> Result<T, String> {
        let mut req = gloo_net::http::Request::get(url)
            .header("Accept", "application/json");
        if let Some(ref t) = self.token {
            req = req.header("Authorization", &format!("Bearer {t}"));
        }
        let resp = req.send().await.map_err(|e| e.to_string())?;
        if !resp.ok() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(format!("{status} {url}: {body}"));
        }
        resp.json::<T>().await.map_err(|e| e.to_string())
    }

    /// Send a JSON body with the given HTTP method.
    async fn send_json<T: for<'de> Deserialize<'de>>(
        &self,
        method: &str,
        url: &str,
        body: &str,
    ) -> Result<T, String> {
        let builder = match method {
            "PATCH" => gloo_net::http::Request::patch(url),
            "POST" => gloo_net::http::Request::post(url),
            "DELETE" => gloo_net::http::Request::delete(url),
            _ => gloo_net::http::Request::post(url),
        };
        let mut req = builder
            .header("Accept", "application/json")
            .header("Content-Type", "application/json");
        if let Some(ref t) = self.token {
            req = req.header("Authorization", &format!("Bearer {t}"));
        }
        let resp = req
            .body(body)
            .map_err(|e| e.to_string())?
            .send()
            .await
            .map_err(|e| e.to_string())?;
        if !resp.ok() {
            let status = resp.status();
            let body_text = resp.text().await.unwrap_or_default();
            return Err(format!("{status} {method} {url}: {body_text}"));
        }
        resp.json::<T>().await.map_err(|e| e.to_string())
    }

    pub async fn ingest_feedback(
        &self,
        workspace: &str,
        target: &str,
        rating: &str,
        note: Option<&str>,
        source: &str,
    ) -> Result<(), String> {
        let workspace_slug = feedback_workspace_slug(workspace);
        let body = serde_json::json!({
            "workspace": workspace,
            "workspace_slug": workspace_slug,
            "source": source,
            "target": target,
            "rating": rating,
            "note": note,
            "note_kind": if note.is_some() { Some("note") } else { None::<&str> },
            "author": "ticket-viewer",
        });
        let body_str = serde_json::to_string(&body).map_err(|e| e.to_string())?;
        self.send_json::<serde_json::Value>("POST", "/api/feedback/ingest", &body_str)
            .await
            .map(|_| ())
    }
}

fn enc(s: &str) -> String {
    utf8_percent_encode(s, NON_ALPHANUMERIC).to_string()
}

fn feedback_workspace_slug(workspace: &str) -> String {
    let trimmed = workspace.trim();
    if trimmed.is_empty() || trimmed.contains('/') || trimmed.contains('\\') {
        "default".to_string()
    } else {
        trimmed.to_string()
    }
}

impl TicketBackend for HttpTicketBackend {
    async fn list_workspaces(&self) -> Result<WorkspacesResponse, String> {
        self.fetch("/api/workspaces").await
    }

    async fn list_tickets(
        &self,
        workspace: &str,
        state: Option<&str>,
        query: Option<&str>,
        limit: Option<u32>,
    ) -> Result<TicketsResponse, String> {
        let mut url = format!("/api/tickets?workspace={}", enc(workspace));
        if let Some(s) = state {
            url.push_str(&format!("&state={}", enc(s)));
        }
        if let Some(q) = query {
            url.push_str(&format!("&query={}", enc(q)));
        }
        if let Some(l) = limit {
            url.push_str(&format!("&limit={l}"));
        }
        self.fetch(&url).await
    }

    async fn get_ticket(
        &self,
        workspace: &str,
        id: &str,
    ) -> Result<TicketDetailResponse, String> {
        self.fetch(&format!("/api/tickets/{id}?workspace={}", enc(workspace)))
            .await
    }

    async fn get_ticket_description(
        &self,
        workspace: &str,
        id: &str,
    ) -> Result<TicketDescriptionResponse, String> {
        self.fetch(&format!(
            "/api/tickets/{id}/description?workspace={}",
            enc(workspace)
        ))
        .await
    }

    async fn get_ticket_full(
        &self,
        workspace: &str,
        id: &str,
    ) -> Result<TicketFullResponse, String> {
        self.fetch(&format!(
            "/api/tickets/{id}?workspace={}&view=full",
            enc(workspace)
        ))
        .await
    }

    async fn get_subgraph(
        &self,
        workspace: &str,
        root: &str,
        depth: u32,
    ) -> Result<GraphSubgraphResponse, String> {
        self.fetch(&format!(
            "/api/graph/subgraph?workspace={}&root={}&depth={}",
            enc(workspace),
            enc(root),
            depth,
        ))
        .await
    }

    async fn get_workspace_graph(
        &self,
        workspace: &str,
    ) -> Result<GraphSubgraphResponse, String> {
        self.fetch(&format!(
            "/api/graph/workspace?workspace={}",
            enc(workspace),
        ))
        .await
    }

    async fn get_workflow_next(
        &self,
        workspace: &str,
    ) -> Result<WorkflowNextResponse, String> {
        self.fetch(&format!(
            "/api/workflow/next?workspace={}&limit=50",
            enc(workspace)
        ))
        .await
    }

    async fn get_workflow_blockers(
        &self,
        workspace: &str,
        root: &str,
    ) -> Result<WorkflowTreeResponse, String> {
        self.fetch(&format!(
            "/api/workflow/blockers?workspace={}&root={}",
            enc(workspace),
            enc(root),
        ))
        .await
    }

    async fn get_workflow_unblocked_by(
        &self,
        workspace: &str,
        root: &str,
    ) -> Result<WorkflowTreeResponse, String> {
        self.fetch(&format!(
            "/api/workflow/unblocked-by?workspace={}&root={}",
            enc(workspace),
            enc(root),
        ))
        .await
    }

    async fn patch_ticket(
        &self,
        workspace: &str,
        id: &str,
        patch: &TicketPatch,
    ) -> Result<TicketDetailResponse, String> {
        let url = format!("/api/tickets/{id}?workspace={}", enc(workspace));
        let body = serde_json::to_string(patch).map_err(|e| e.to_string())?;
        self.send_json("PATCH", &url, &body).await
    }

    async fn list_schemas(
        &self,
        workspace: &str,
    ) -> Result<SchemaListResponse, String> {
        self.fetch(&format!("/api/schema?workspace={}", enc(workspace)))
            .await
    }

    async fn create_edge(
        &self,
        workspace: &str,
        body: &EdgeMutationBody,
    ) -> Result<(), String> {
        let url = format!("/api/edges?workspace={}", enc(workspace));
        let json_body =
            serde_json::to_string(body).map_err(|e| e.to_string())?;
        let mut req = gloo_net::http::Request::post(&url)
            .header("Accept", "application/json")
            .header("Content-Type", "application/json");
        if let Some(ref t) = self.token {
            req = req.header("Authorization", &format!("Bearer {t}"));
        }
        let resp = req
            .body(json_body)
            .map_err(|e| e.to_string())?
            .send()
            .await
            .map_err(|e| e.to_string())?;
        if resp.status() == 422 {
            return Err("cycle_detected: Adding this edge would create a dependency cycle".to_string());
        }
        if !resp.ok() {
            let status = resp.status();
            let body_text = resp.text().await.unwrap_or_default();
            return Err(format!("{status} POST {url}: {body_text}"));
        }
        Ok(())
    }

    async fn delete_edge(
        &self,
        workspace: &str,
        body: &EdgeMutationBody,
    ) -> Result<(), String> {
        let url = format!("/api/edges?workspace={}", enc(workspace));
        let json_body =
            serde_json::to_string(body).map_err(|e| e.to_string())?;
        let mut req = gloo_net::http::Request::delete(&url)
            .header("Accept", "application/json")
            .header("Content-Type", "application/json");
        if let Some(ref t) = self.token {
            req = req.header("Authorization", &format!("Bearer {t}"));
        }
        let resp = req
            .body(json_body)
            .map_err(|e| e.to_string())?
            .send()
            .await
            .map_err(|e| e.to_string())?;
        if !resp.ok() {
            let status = resp.status();
            let body_text = resp.text().await.unwrap_or_default();
            return Err(format!("{status} DELETE {url}: {body_text}"));
        }
        Ok(())
    }

    async fn create_ticket(
        &self,
        workspace: &str,
        body: &CreateTicketRequest,
    ) -> Result<CreateTicketResponse, String> {
        let url = format!("/api/tickets?workspace={}", enc(workspace));
        let json_body =
            serde_json::to_string(body).map_err(|e| e.to_string())?;
        self.send_json("POST", &url, &json_body).await
    }

    async fn get_schema_by_type(
        &self,
        workspace: &str,
        type_id: &str,
    ) -> Result<SchemaDetailResponse, String> {
        self.fetch(&format!(
            "/api/schema/{}?workspace={}",
            enc(type_id),
            enc(workspace)
        ))
        .await
    }

    async fn get_ticket_history(
        &self,
        workspace: &str,
        id: &str,
    ) -> Result<TicketHistoryResponse, String> {
        self.fetch(&format!(
            "/api/tickets/{id}/history?workspace={}",
            enc(workspace)
        ))
        .await
    }

    async fn revert_ticket(
        &self,
        workspace: &str,
        id: &str,
        revision: u64,
    ) -> Result<TicketDetailResponse, String> {
        let url =
            format!("/api/tickets/{id}/revert?workspace={}", enc(workspace));
        let body = serde_json::json!({ "revision": revision }).to_string();
        self.send_json("POST", &url, &body).await
    }

    async fn undo_ticket(
        &self,
        workspace: &str,
        id: &str,
    ) -> Result<TicketDetailResponse, String> {
        let url =
            format!("/api/tickets/{id}/undo?workspace={}", enc(workspace));
        let mut req = gloo_net::http::Request::post(&url)
            .header("Accept", "application/json");
        if let Some(ref t) = self.token {
            req = req.header("Authorization", &format!("Bearer {t}"));
        }
        let resp = req.send().await.map_err(|e| e.to_string())?;
        if !resp.ok() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(format!("{status} POST {url}: {body}"));
        }
        resp.json::<TicketDetailResponse>()
            .await
            .map_err(|e| e.to_string())
    }

    async fn update_ticket_description(
        &self,
        workspace: &str,
        id: &str,
        text: &str,
    ) -> Result<(), String> {
        let url = format!("/api/tickets/{id}?workspace={}", enc(workspace));
        // The editor always saves the full textarea contents, so this is a
        // whole-file overwrite: `replace` is the correct explicit mode, not
        // an omitted default (description_mode has no default; see ticket
        // 3d952036).
        let body = serde_json::json!({
            "description": text,
            "description_mode": "replace",
        });
        let body_str =
            serde_json::to_string(&body).map_err(|e| e.to_string())?;
        self.send_json::<serde_json::Value>("PATCH", &url, &body_str)
            .await
            .map(|_| ())
    }

    async fn list_ticket_files(
        &self,
        workspace: &str,
        id: &str,
    ) -> Result<crate::types::TicketFilesResponse, String> {
        self.fetch(&format!(
            "/api/tickets/{id}/files?workspace={}",
            enc(workspace)
        ))
        .await
    }

    async fn get_ticket_asset(
        &self,
        workspace: &str,
        id: &str,
        path: &str,
    ) -> Result<crate::types::TicketAssetResponse, String> {
        self.fetch(&format!(
            "/api/tickets/{id}/asset?workspace={}&path={}",
            enc(workspace),
            enc(path),
        ))
        .await
    }

    async fn batch_tickets(
        &self,
        body: &crate::types::BatchRequest,
    ) -> Result<crate::types::BatchResponse, String> {
        let body_str =
            serde_json::to_string(body).map_err(|e| e.to_string())?;
        self.send_json("POST", "/api/batch", &body_str).await
    }
}
