#[allow(unused_imports)]
use super::*;

pub(super) fn render_recent_searches(
    recents: &[String],
    mut query: Signal<String>,
    mut state_filter: Signal<Option<String>>,
    mut type_filter: Signal<Option<String>>,
    mut hovered_recent: Signal<Option<usize>>,
) -> Element {
    rsx! {
        div {
            style: "padding: 0.5rem 0;",
            div {
                style: "
                    padding: 0.25rem 1rem;
                    font-size: 11px;
                    color: #6666aa;
                    text-transform: uppercase;
                    letter-spacing: 0.08em;
                ",
                "Recent searches"
            }
            for (index, recent) in recents.iter().enumerate() {
                {
                    let recent = recent.clone();
                    let is_hovered = *hovered_recent.read() == Some(index);
                    let row_bg = if is_hovered {
                        "rgba(80,80,140,0.25)"
                    } else {
                        "transparent"
                    };
                    rsx! {
                        button {
                            "data-testid": "search-recent-{index}",
                            aria_selected: if is_hovered { "true" } else { "false" },
                            style: "
                                display: flex;
                                align-items: center;
                                gap: 0.5rem;
                                width: 100%;
                                background: {row_bg};
                                border: none;
                                padding: 0.5rem 1rem;
                                color: #c0c0e8;
                                font-size: 0.9rem;
                                cursor: pointer;
                                text-align: left;
                            ",
                            onmouseenter: move |_| hovered_recent.set(Some(index)),
                            onmouseleave: move |_| hovered_recent.set(None),
                            onclick: move |_| {
                                state_filter.set(None);
                                type_filter.set(None);
                                query.set(recent.clone());
                            },
                            span { style: "color: #6666aa; font-size: 0.85rem;", "↩" }
                            span { "{recent}" }
                        }
                    }
                }
            }
        }
    }
}
