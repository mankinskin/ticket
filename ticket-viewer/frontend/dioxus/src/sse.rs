//! `use_sse` — Dioxus hook for live ticket updates via Server-Sent Events.
//!
//! Connects to `GET /api/stream?workspace={workspace}` and keeps the provided
//! list view authoritative under incoming events:
//!
//! | SSE event       | Action                                             |
//! |-----------------|----------------------------------------------------|
//! | `ticket.upsert` | Trigger a silent list refetch.                     |
//! | `ticket.delete` | Trigger a silent list refetch.                     |
//! | `snapshot.ready`| Trigger a background list refetch.                 |
//! | `edge.*`        | Acknowledged, no ticket-list mutation.             |
//! | `open`          | Reset backoff to the initial value.                |
//! | `error`         | Close the connection, schedule reconnect w/ backoff. |
//!
//! Reconnect uses exponential backoff: 1 s → 2 s → 4 s → … capped at 30 s.
//!
//! # Auth note
//!
//! `EventSource` cannot send custom request headers, so the `/api/stream`
//! endpoint intentionally carries no authentication middleware.  No changes
//! are needed here.

use dioxus::prelude::*;
use gloo_events::EventListener;
use gloo_timers::callback::Timeout;
use serde::Deserialize;
use wasm_bindgen::JsCast;

use crate::types::TicketSummary;

// ── SSE payload types (frontend deserialisation) ───────────────────────────

/// Minimal ticket snapshot embedded in `ticket.upsert` events.
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct SseTicket {
    id: String,
    state: Option<String>,
    title: Option<String>,
}

/// Data payload for `ticket.upsert` SSE events.
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct UpsertPayload {
    ticket: SseTicket,
}

/// Data payload for `ticket.delete` SSE events.
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct DeletePayload {
    /// UUID of the deleted ticket, serialised as a string by the server.
    id: String,
}

// ── SseHandle ─────────────────────────────────────────────────────────────

/// RAII owner of a live SSE session.
///
/// Dropping this value:
/// 1. Calls [`web_sys::EventSource::close()`], preventing the browser from
///    attempting its own auto-reconnect.
/// 2. Drops each [`EventListener`], which removes the JS event listeners from
///    the `EventTarget` (no `Closure::forget()` calls anywhere in this file).
struct SseHandle {
    es: web_sys::EventSource,
    /// Must be held alive — gloo removes listeners from the DOM on drop.
    _listeners: Vec<EventListener>,
}

impl Drop for SseHandle {
    fn drop(&mut self) {
        self.es.close();
    }
}

// ── Backoff parameters ─────────────────────────────────────────────────────

const BACKOFF_INITIAL_MS: u32 = 1_000;
const BACKOFF_MAX_MS: u32 = 30_000;

// ── Hook ───────────────────────────────────────────────────────────────────

/// Dioxus hook: open an SSE connection to `/api/stream?workspace={workspace}`
/// and live-update `tickets` from incoming events.
///
/// Call this once at the top of the component that owns the ticket list, e.g.
/// `TicketListPage`.  The connection is torn down automatically when the
/// component unmounts.
///
/// # Arguments
///
/// * `workspace` — name of the workspace to subscribe to (used as a query-
///   string parameter).
/// * `tickets` — signal that holds the displayed ticket list.
/// * `refresh_counter` — monotonic signal incremented when the backend emits a
///   `snapshot.ready` heartbeat, allowing the list page to refetch from the
///   current filter state.
/// * `silent_refresh` — marks the next refetch as background-only so the
///   current list stays visible during reconcile heartbeats.
pub fn use_sse(
    workspace: String,
    _tickets: Signal<Vec<TicketSummary>>,
    refresh_counter: Signal<u32>,
    silent_refresh: Signal<bool>,
) {
    // Monotonic counter — incrementing this value re-runs the effect, which
    // drops the old `SseHandle` (closing the EventSource) and opens a fresh one.
    #[cfg_attr(not(target_arch = "wasm32"), allow(unused_mut))]
    let mut reconnect_gen: Signal<u32> = use_signal(|| 0_u32);

    // Current reconnect delay in milliseconds; doubled after each error.
    #[cfg_attr(not(target_arch = "wasm32"), allow(unused_mut))]
    let mut backoff_ms: Signal<u32> = use_signal(|| BACKOFF_INITIAL_MS);

    // Keeps the live `SseHandle` alive for the component's lifetime.
    // The Option allows us to drop it explicitly before creating the next one.
    #[cfg_attr(not(target_arch = "wasm32"), allow(unused_mut))]
    let mut handle: Signal<Option<SseHandle>> = use_signal(|| None);

    // Owns the pending reconnect timer so it can be cancelled on rerender or
    // unmount instead of firing after the hook has been torn down.
    #[cfg_attr(not(target_arch = "wasm32"), allow(unused_mut))]
    let mut retry_timer: Signal<Option<Timeout>> = use_signal(|| None);

    use_effect(move || {
        // Reading `reconnect_gen` subscribes this effect to its changes, so
        // the effect body re-runs each time we increment it for a reconnect.
        let _gen = reconnect_gen();

        // Drop the previous SseHandle before opening the next connection.
        // `SseHandle::drop` calls `es.close()`, preventing the browser's own
        // auto-reconnect from racing with ours.
        handle.with_mut(|h| *h = None);
        retry_timer.with_mut(|timer| *timer = None);

        let url = format!("/api/stream?workspace={workspace}");
        let es = match web_sys::EventSource::new(&url) {
            Ok(es) => es,
            Err(e) => {
                tracing::error!(
                    "EventSource::new failed for {workspace}: {:?}",
                    e
                );
                return;
            },
        };

        // ── ticket.upsert ──────────────────────────────────────────────────
        // Refetch through the normal list request path so current query/state
        // filters remain authoritative.
        let mut refresh_upsert = refresh_counter;
        let mut silent_upsert = silent_refresh;
        // The graph-layout fetch service only exists on wasm32 (see `main.rs`),
        // so gate its acquisition and use consistently with the rest of the
        // crate. On native builds the SSE path still updates the list counters.
        #[cfg(target_arch = "wasm32")]
        let service_upsert =
            use_context::<crate::graph_fetch::GraphFetchService>();
        #[cfg(target_arch = "wasm32")]
        let workspace_upsert = workspace.clone();
        let l_upsert = EventListener::new(&es, "ticket.upsert", move |event| {
            let data = msg_data(event);
            match serde_json::from_str::<UpsertPayload>(&data) {
                Ok(_payload) => {
                    #[cfg(target_arch = "wasm32")]
                    service_upsert.update_node_or_invalidate(
                        &workspace_upsert,
                        &_payload.ticket.id,
                        _payload.ticket.title.as_deref(),
                        _payload.ticket.state.as_deref(),
                    );
                    silent_upsert.set(true);
                    refresh_upsert.with_mut(|value| *value += 1);
                },
                Err(e) => tracing::warn!("ticket.upsert parse error: {e}"),
            }
        });

        // ── ticket.delete ──────────────────────────────────────────────────
        let mut refresh_delete = refresh_counter;
        let mut silent_delete = silent_refresh;
        let l_delete = EventListener::new(&es, "ticket.delete", move |event| {
            let data = msg_data(event);
            match serde_json::from_str::<DeletePayload>(&data) {
                Ok(_payload) => {
                    silent_delete.set(true);
                    refresh_delete.with_mut(|value| *value += 1);
                },
                Err(e) => tracing::warn!("ticket.delete parse error: {e}"),
            }
        });

        // ── snapshot.ready ────────────────────────────────────────────────
        // The HTTP reconcile loop emits this heartbeat so clients can rebuild
        // local state after mutations made outside this process (for example
        // via the CLI or MCP server).
        let mut refresh = refresh_counter;
        let mut silent = silent_refresh;
        let l_snapshot = EventListener::new(&es, "snapshot.ready", move |_| {
            silent.set(true);
            refresh.with_mut(|value| *value += 1);
        });

        // ── edge events ────────────────────────────────────────────────────
        // Acknowledged without mutating the ticket list.  The dep-graph
        // component has its own SSE subscription that handles edge events.
        let l_edge_up = EventListener::new(&es, "edge.upsert", move |_| {});
        let l_edge_del = EventListener::new(&es, "edge.delete", move |_| {});

        // ── open — reset backoff ───────────────────────────────────────────
        let mut boff_open = backoff_ms;
        let l_open = EventListener::new(&es, "open", move |_| {
            boff_open.set(BACKOFF_INITIAL_MS);
            tracing::debug!("SSE connected; backoff reset");
        });

        // ── error — close and schedule reconnect with exponential backoff ──
        //
        // Clone the JsValue-backed EventSource so the closure can call
        // `close()` immediately, preventing the browser's own automatic
        // reconnect from competing with our backoff schedule.
        let es_close = es.clone();
        let mut rgen = reconnect_gen;
        let mut boff_err = backoff_ms;
        let mut retry_timer_err = retry_timer;
        let l_error = EventListener::new(&es, "error", move |_| {
            // Prevent the browser's built-in SSE reconnect.
            es_close.close();

            let delay = *boff_err.peek();
            let next_delay = (delay * 2).min(BACKOFF_MAX_MS);
            boff_err.set(next_delay);
            tracing::debug!("SSE error; reconnecting in {delay} ms (next backoff {next_delay} ms)");

            retry_timer_err.with_mut(|timer| {
                *timer = Some(Timeout::new(delay, move || {
                    // Incrementing `reconnect_gen` re-triggers the enclosing
                    // `use_effect`, which drops the old SseHandle and opens a
                    // fresh EventSource.
                    rgen.with_mut(|v| *v += 1);
                }));
            });
        });

        handle.with_mut(|h| {
            *h = Some(SseHandle {
                es,
                _listeners: vec![
                    l_upsert, l_delete, l_snapshot, l_edge_up, l_edge_del,
                    l_open, l_error,
                ],
            });
        });
    });
}

// ── Helpers ────────────────────────────────────────────────────────────────

/// Extract the `data` string from a DOM `MessageEvent`.
///
/// Returns an empty string if the event is not a `MessageEvent` or if the
/// data field is not a plain string (we never expect binary payloads here).
fn msg_data(event: &web_sys::Event) -> String {
    event
        .dyn_ref::<web_sys::MessageEvent>()
        .and_then(|msg| msg.data().as_string())
        .unwrap_or_default()
}
