use dioxus::prelude::*;

use super::model::CommandResult;

pub(crate) fn render_results_panel(
    results: &[CommandResult],
    on_done: EventHandler<()>,
) -> Element {
    rsx! {
        div {
            style: "
                pointer-events: all;
                background: var(--bg-primary, #1a1a2e);
                border-top: 1px solid var(--border-subtle, #333);
                max-height: 200px;
                overflow-y: auto;
                padding: 8px 16px;
            ",
            div {
                style: "display: flex; justify-content: space-between; align-items: center; margin-bottom: 6px;",
                span {
                    style: "font-size: 12px; font-weight: 600; color: var(--text-primary);",
                    "Batch results"
                }
                button {
                    style: "border: none; background: transparent; color: var(--text-muted); cursor: pointer; font-size: 11px;",
                    onclick: move |_| on_done.call(()),
                    "✕ Close & refresh"
                }
            }
            for result in results.iter() {
                {
                    let row_color = if result.ok {
                        "var(--success, #4ade80)"
                    } else {
                        "var(--error, #f87171)"
                    };
                    let short_id = if result.id.len() >= 8 {
                        result.id[..8].to_string()
                    } else {
                        result.id.clone()
                    };
                    let message = result.message.clone();
                    rsx! {
                        div {
                            key: "{result.id}",
                            style: "
                                display: flex; gap: 8px; align-items: baseline;
                                font-size: 11px; padding: 2px 0;
                                color: {row_color};
                            ",
                            span {
                                style: "font-weight: 600; min-width: 100px; overflow: hidden; text-overflow: ellipsis;",
                                "{short_id}"
                            }
                            span { "{message}" }
                        }
                    }
                }
            }
        }
    }
}
