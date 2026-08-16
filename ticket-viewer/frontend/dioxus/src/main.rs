mod api;
mod components;
#[cfg(target_arch = "wasm32")]
mod graph3d;
#[cfg(target_arch = "wasm32")]
mod graph_fetch;
mod layout;
mod routes;
mod search_syntax;
mod sse;
mod store;
mod types;

use dioxus::prelude::*;
use viewer_api_dioxus::{
    Prefetcher,
    ThemeProvider,
    ViewerShell,
    WgpuOverlay,
};

use crate::layout::GraphLayout;
use routes::Route;

/// LRU cache shared across all graph views in the ticket viewer.
///
/// Keyed by `"{workspace}:{root_id}"`, holds a precomputed [`GraphLayout`] so
/// that switching back to a previously-opened ticket renders the graph
/// instantly without a round-trip to the backend.  The [`crate::graph3d::lift_2d`]
/// step is deferred to render time so the current [`crate::layout::LayoutMode`]
/// can be applied without re-fetching.
pub type GraphCache = Prefetcher<String, GraphLayout>;

fn main() {
    #[cfg(target_arch = "wasm32")]
    viewer_api_dioxus::tracing_setup::install();
    dioxus_web::launch::launch_cfg(App, dioxus_web::Config::default());
}

/// Root application component for the ticket viewer.
///
/// Mounts the shared `ViewerShell` (WebGPU canvas + UI overlay) from
/// `viewer-api-dioxus` and nests the ticket-viewer SPA router inside the
/// overlay so all route components render on top of the GPU canvas layer.
///
/// [`WgpuOverlay`] provides background effects (smoke, CRT, particles) when
/// the 3-D dependency graph is not visible.  When [`Graph3D`] mounts it calls
/// [`viewer_api_dioxus::set_gpu_canvas_owner`]`(true)`, causing `WgpuOverlay`
/// to idle its render loop and cede the canvas.  When `Graph3D` unmounts the
/// flag is cleared and `WgpuOverlay` resumes automatically.
#[component]
fn App() -> Element {
    // Provide a single shared LRU graph-layout cache (capacity 20) for the
    // entire app.  Graph3D consults it before falling back to the network,
    // eliminating the "Loading graph…" flash when revisiting a ticket.
    #[cfg_attr(not(target_arch = "wasm32"), allow(unused_variables))]
    let cache =
        use_context_provider::<GraphCache>(|| Prefetcher::with_capacity(20));

    // Provide the centralised fetch service.  All subgraph HTTP requests go
    // through this service; Graph3D only reads the cache, never fetches.
    #[cfg(target_arch = "wasm32")]
    use_context_provider::<crate::graph_fetch::GraphFetchService>(|| {
        crate::graph_fetch::GraphFetchService::new(cache)
    });

    rsx! {
        style { "html, body, #main {{ overflow: hidden; margin: 0; padding: 0; width: 100%; height: 100%; }}" }
        ThemeProvider {
            ViewerShell {
                WgpuOverlay {}
                Router::<Route> {}
            }
        }
    }
}
