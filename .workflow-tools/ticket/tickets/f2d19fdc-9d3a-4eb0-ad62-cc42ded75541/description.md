Commit 4bb0961 ("chore(warnings): apply mechanical unused cleanup in dioxus viewers") was applied under native-target checking only and broke the wasm32 build of ticket-viewer-dioxus. It:

- Removed three imports used only in `#[cfg(target_arch = "wasm32")]` code in `components/dep_graph/page.rs`: `dioxus_router::Navigator`, `crate::types::TicketRef`, and `super::interactions::select_node_or_navigate`.
- Renamed the DepGraph webgpu bindings (`nav`, `on_hover`, `on_deselect`, `selected_node_id`, `hovered_node_id`) to `_`-prefixed while the wasm-only `render_webgpu_graph` call site still referenced the non-underscore names → E0425.
- Dropped `mut` from `open_ticket` in `store.rs`, which is mutated inside the wasm-only hashchange listener → E0596.

This broke the wasm build for the entire ticket-viewer, blocking browser verification for all frontend workstreams under the roadmap tracker (W1, W3, W4, etc.).

Fix (target-correct, native-clean):
- Restored the three imports gated behind `#[cfg(target_arch = "wasm32")]`.
- Changed the wasm branch to pass the `_`-prefixed bindings.
- Restored `let mut open_ticket` with `#[cfg_attr(not(target_arch = "wasm32"), allow(unused_mut))]`.

Validation: `cargo check --target wasm32-unknown-unknown --features profile-browser` now finishes (EXIT 0, warnings only). Native `cargo check` remains clean.

Note: A separate ENVIRONMENT blocker remains for the full `trunk build`/browser step — `memory-viewers/viewer-api` is an empty, non-gitlinked directory that the shared trunk CSS asset links target (real CSS lives at root `viewer-api/viewer-api/frontend/dioxus/public/css/`). `./install-tools.sh` last exited 1. This blocks `viewer-ctl prepare ticket-viewer` and browser verification until the environment/link setup is repaired; it is not a code defect in this ticket.