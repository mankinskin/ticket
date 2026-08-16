//! Centralised graph-layout fetch service.
//!
//! [`GraphFetchService`] is provided as a Dioxus context (see `main.rs`) and
//! is the **sole** place where backend subgraph requests are issued.  Every
//! call to [`GraphFetchService::ensure_fetched`] is guaranteed to:
//!
//! 1. Return immediately if the layout is already in the LRU cache.
//! 2. Return immediately if a fetch for the same key is already in-flight,
//!    avoiding duplicate concurrent requests.
//! 3. Otherwise spawn exactly one `gloo_net` HTTP request that, when
//!    complete, writes the result into the shared cache and bumps a reactive
//!    version counter — causing every `Graph3D` component that reads that
//!    counter to re-render and pick up the new cache entry.
//!
//! This architecture means `Graph3D` never issues its own network requests.
//! It only reads the cache; therefore rapidly mounting and unmounting
//! `Graph3D` (due to fast ticket switching) cannot saturate the browser
//! connection pool.

use std::{
    cell::RefCell,
    collections::{
        HashMap,
        HashSet,
        VecDeque,
    },
    rc::Rc,
};

use dioxus::prelude::*;
use futures_util::pin_mut;
use gloo_timers::future::TimeoutFuture;

use crate::{
    api::{
        HttpTicketBackend,
        TicketBackend,
    },
    layout::GraphLayout,
    GraphCache,
};

const T: &str = "ticket_viewer::graph_fetch";
const REQUEST_TIMEOUT_MS: u32 = 20_000;
const MAX_CONCURRENT_FETCHES: usize = 3;
const MAX_RETRIES: u8 = 1;

pub(crate) fn workspace_cache_key(workspace: &str) -> String {
    format!("workspace:{workspace}")
}

// ── Inner state ───────────────────────────────────────────────────────────────

#[derive(Clone)]
struct FetchJob {
    cache_key: String,
    workspace: String,
}

#[derive(Clone)]
struct FetchFailure {
    message: String,
    attempts: u8,
}

struct Inner {
    /// Keys currently queued or being fetched.
    in_flight: HashSet<String>,
    /// FIFO fetch queue; bounded concurrency is enforced by `active_fetches`.
    queue: VecDeque<FetchJob>,
    active_fetches: usize,
    /// Last fetch error by cache key, surfaced to the UI.
    errors: HashMap<String, FetchFailure>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GraphFetchState {
    Idle,
    Loading,
    Ready,
    Error { message: String, attempts: u8 },
}

// ── Public API ────────────────────────────────────────────────────────────────

/// A cheap-to-clone handle to the global fetch service.
///
/// Cloning shares the same `Rc<RefCell<Inner>>` and the same Dioxus signals.
#[derive(Clone)]
pub struct GraphFetchService {
    inner: Rc<RefCell<Inner>>,
    cache: GraphCache,
    /// Bumped (incremented) every time a fetch completes and a new layout is
    /// inserted into the cache.  Components that call `version()` subscribe to
    /// this signal and are automatically scheduled for re-render.
    version: Signal<u64>,
}

impl GraphFetchService {
    /// Create a new service.  Call once from `use_context_provider` in `App`.
    pub fn new(cache: GraphCache) -> Self {
        Self {
            inner: Rc::new(RefCell::new(Inner {
                in_flight: HashSet::new(),
                queue: VecDeque::new(),
                active_fetches: 0,
                errors: HashMap::new(),
            })),
            cache,
            version: Signal::new(0),
        }
    }

    /// Read the reactive version counter.
    ///
    /// Calling this inside a Dioxus component body (or `use_memo` / `use_effect`)
    /// subscribes the current scope to updates: the scope will be re-scheduled
    /// whenever a fetch completes.
    pub fn version(&self) -> u64 {
        *self.version.read()
    }

    pub fn state_for(
        &self,
        workspace: &str,
        _root_id: &str,
    ) -> GraphFetchState {
        let cache_key = workspace_cache_key(workspace);
        if self.cache.get(&cache_key).is_some() {
            return GraphFetchState::Ready;
        }
        let inner = self.inner.borrow();
        if inner.in_flight.contains(&cache_key) {
            return GraphFetchState::Loading;
        }
        if let Some(err) = inner.errors.get(&cache_key) {
            return GraphFetchState::Error {
                message: err.message.clone(),
                attempts: err.attempts,
            };
        }
        GraphFetchState::Idle
    }

    pub fn retry(
        &self,
        workspace: &str,
        _root_id: &str,
    ) {
        let cache_key = workspace_cache_key(workspace);
        self.inner.borrow_mut().errors.remove(&cache_key);
        self.ensure_fetched(workspace, "");
    }

    /// Invalidate the cache entry for a workspace, forcing a fresh fetch.
    ///
    /// Removes the layout from the cache, clears any stored error, and removes
    /// the key from the in-flight set.  The next call to `ensure_fetched` will
    /// trigger a new fetch.
    pub fn invalidate_workspace(
        &self,
        workspace: &str,
    ) {
        let cache_key = workspace_cache_key(workspace);
        self.cache.remove(&cache_key);
        let mut inner = self.inner.borrow_mut();
        inner.errors.remove(&cache_key);
        inner.in_flight.remove(&cache_key);
        // Bump version to trigger UI updates
        let mut version = self.version;
        *version.write() += 1;
    }

    /// Try to update a single node's visual properties (title/state) in the cache.
    /// Falls back to full invalidate if the node doesn't exist or isn't cached.
    pub fn update_node_or_invalidate(
        &self,
        workspace: &str,
        ticket_id: &str,
        title: Option<&str>,
        state: Option<&str>,
    ) {
        let cache_key = workspace_cache_key(workspace);
        if let Some(mut layout) = self.cache.get(&cache_key) {
            let mut found = false;
            for node in &mut layout.nodes {
                if node.id == ticket_id {
                    if let Some(t) = title {
                        node.title = Some(t.to_string());
                    }
                    if let Some(s) = state {
                        node.state = Some(s.to_string());
                    }
                    found = true;
                    break;
                }
            }
            if found {
                self.cache.insert(cache_key, layout);
                // Bump version to trigger UI updates
                let mut version = self.version;
                *version.write() += 1;
                return;
            }
        }
        // Fallback: full invalidation
        self.invalidate_workspace(workspace);
    }

    /// Ensure a workspace-scoped graph layout will be available in the shared
    /// cache.
    ///
    /// * Cache hit  → no-op (returns immediately).
    /// * In-flight  → no-op (another spawn is already working on it).
    /// * Cold       → spawns a fetch task that writes to the cache on success.
    ///
    /// Safe to call from any component render or effect body.
    pub fn ensure_fetched(
        &self,
        workspace: &str,
        _root_id: &str,
    ) {
        let cache_key = workspace_cache_key(workspace);

        // Fast paths — no spawn needed.
        if self.cache.get(&cache_key).is_some() {
            tracing::debug!(target: T, key = %cache_key, "ensure_fetched: cache hit");
            return;
        }
        {
            let inner = self.inner.borrow();
            if inner.in_flight.contains(&cache_key) {
                tracing::debug!(target: T, key = %cache_key, "ensure_fetched: already in-flight");
                return;
            }
        }

        // Mark in-flight before queueing so a second synchronous call while
        // the microtask queue hasn't flushed yet doesn't enqueue a duplicate.
        {
            let mut inner = self.inner.borrow_mut();
            inner.errors.remove(&cache_key);
            inner.in_flight.insert(cache_key.clone());
            inner.queue.push_back(FetchJob {
                cache_key: cache_key.clone(),
                workspace: workspace.to_string(),
            });
        }

        #[cfg(target_arch = "wasm32")]
        web_sys::console::log_1(
            &format!("[graph_fetch] enqueue key={cache_key}").into(),
        );

        tracing::debug!(target: T, key = %cache_key, "ensure_fetched: queued fetch");
        self.pump_queue();
    }

    fn pump_queue(&self) {
        loop {
            let Some(job) = self.dequeue_next_job() else {
                break;
            };

            #[cfg(target_arch = "wasm32")]
            web_sys::console::log_1(
                &format!("[graph_fetch] spawn key={}", job.cache_key).into(),
            );
            tracing::debug!(target: T, key = %job.cache_key, "spawn_fetch");

            let mut svc = self.clone();
            spawn(async move {
                let key = job.cache_key.clone();
                match svc.fetch_with_retry(&job.workspace).await {
                    Ok(layout) => {
                        let n = layout.nodes.len();
                        svc.cache.insert(key.clone(), layout);
                        {
                            let mut inner = svc.inner.borrow_mut();
                            inner.in_flight.remove(&key);
                            inner.active_fetches =
                                inner.active_fetches.saturating_sub(1);
                            inner.errors.remove(&key);
                        }
                        #[cfg(target_arch = "wasm32")]
                        web_sys::console::log_1(
                            &format!(
                                "[graph_fetch] fetch_ok key={key} nodes={n}"
                            )
                            .into(),
                        );
                        tracing::debug!(target: T, key = %key, nodes = n, "fetch_ok");
                        // Bump version — wakes every Graph3D that subscribed.
                        svc.version += 1;
                    },
                    Err((message, attempts)) => {
                        {
                            let mut inner = svc.inner.borrow_mut();
                            inner.in_flight.remove(&key);
                            inner.active_fetches =
                                inner.active_fetches.saturating_sub(1);
                            inner.errors.insert(
                                key.clone(),
                                FetchFailure {
                                    message: message.clone(),
                                    attempts,
                                },
                            );
                        }
                        #[cfg(target_arch = "wasm32")]
                        web_sys::console::log_1(
                            &format!(
                                "[graph_fetch] fetch_err key={key} attempts={attempts} err={message}"
                            )
                            .into(),
                        );
                        tracing::warn!(
                            target: T,
                            key = %key,
                            attempts,
                            error = %message,
                            "fetch_err"
                        );
                        // Bump version so UI can transition from spinner to an error message.
                        svc.version += 1;
                    },
                }

                // Continue draining queued requests after each completion.
                svc.pump_queue();
            });
        }
    }

    fn dequeue_next_job(&self) -> Option<FetchJob> {
        let mut inner = self.inner.borrow_mut();
        if inner.active_fetches >= MAX_CONCURRENT_FETCHES {
            return None;
        }
        let job = inner.queue.pop_front()?;
        inner.active_fetches += 1;
        Some(job)
    }

    async fn fetch_with_retry(
        &self,
        workspace: &str,
    ) -> Result<GraphLayout, (String, u8)> {
        let mut last_error = String::from("unknown error");

        for attempt in 0..=MAX_RETRIES {
            match Self::fetch_once_with_timeout(workspace).await {
                Ok(layout) => return Ok(layout),
                Err(err) => {
                    last_error = err;
                    if attempt < MAX_RETRIES {
                        tracing::warn!(
                            target: T,
                            workspace,
                            attempt = attempt + 1,
                            "retrying fetch after failure"
                        );
                    }
                },
            }
        }

        Err((last_error, MAX_RETRIES + 1))
    }

    async fn fetch_once_with_timeout(
        workspace: &str
    ) -> Result<GraphLayout, String> {
        let backend = HttpTicketBackend::new(None);
        let fetch_fut =
            async move { backend.get_workspace_graph(workspace).await };
        let timeout_fut = TimeoutFuture::new(REQUEST_TIMEOUT_MS);
        pin_mut!(fetch_fut);
        pin_mut!(timeout_fut);

        match futures_util::future::select(fetch_fut, timeout_fut).await {
            futures_util::future::Either::Left((fetch_result, _)) =>
                match fetch_result {
                    Ok(resp) => {
                        let active_workspace =
                            if resp.active_workspace.is_empty() {
                                workspace.to_string()
                            } else {
                                resp.active_workspace.clone()
                            };
                        Ok(GraphLayout::build(
                            &active_workspace,
                            resp.nodes,
                            resp.edges,
                        ))
                    },
                    Err(e) => Err(format!("request failed: {e}")),
                },
            futures_util::future::Either::Right((_elapsed, _)) =>
                Err(format!("request timed out after {}ms", REQUEST_TIMEOUT_MS)),
        }
    }
}
