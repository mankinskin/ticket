//! Graph settings panel — floating gear button + popover with layout / projection options.

use dioxus::prelude::*;
use viewer_api_dioxus::Projection;

use crate::layout::LayoutMode;

const PANEL_STYLE: &str = "
    position: absolute;
    top: calc(100% + 4px);
    right: 0;
    min-width: 220px;
    background: rgba(22, 25, 34, 0.96);
    border: 1px solid rgba(255,255,255,0.12);
    border-radius: 10px;
    padding: 14px 16px;
    box-shadow: 0 8px 28px rgba(0,0,0,0.55);
    font-family: sans-serif;
    font-size: 13px;
    color: #d8d8e8;
    z-index: 1000;
";

const SECTION_LABEL_STYLE: &str = "
    font-size: 11px;
    font-weight: 700;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: #888;
    margin-bottom: 8px;
";

const OPTION_ROW_STYLE: &str = "
    display: flex;
    gap: 8px;
    margin-bottom: 6px;
";

fn option_btn_style(active: bool) -> String {
    let bg = if active {
        "rgba(79, 140, 255, 0.22)"
    } else {
        "rgba(255,255,255,0.06)"
    };
    let border = if active {
        "1px solid rgba(79,140,255,0.55)"
    } else {
        "1px solid rgba(255,255,255,0.1)"
    };
    let color = if active { "#93bbff" } else { "#bbb" };
    format!(
        "flex: 1; padding: 6px 0; border-radius: 6px; border: {border}; \
         background: {bg}; color: {color}; font-size: 12px; font-weight: 500; \
         cursor: pointer; text-align: center; white-space: nowrap;"
    )
}

#[derive(Props, Clone, PartialEq)]
pub struct GraphSettingsPanelProps {
    pub layout_mode: Signal<LayoutMode>,
    pub projection:  Signal<Projection>,
}

#[component]
pub fn GraphSettingsPanel(props: GraphSettingsPanelProps) -> Element {
    let mut open: Signal<bool> = use_hook(|| Signal::new(false));
    let mut layout_mode = props.layout_mode;
    let mut projection  = props.projection;

    let cur_layout = *layout_mode.read();
    let cur_proj   = *projection.read();

    rsx! {
        div {
            style: "position: relative; display: inline-block;",

            // Gear button
            button {
                style: "
                    width: 34px; height: 34px;
                    border-radius: 7px;
                    border: 1px solid rgba(255,255,255,0.12);
                    background: rgba(255,255,255,0.07);
                    color: #ccc;
                    font-size: 17px;
                    cursor: pointer;
                    display: flex; align-items: center; justify-content: center;
                    transition: background 0.15s;
                ",
                title: "Graph settings",
                onclick: move |evt| {
                    evt.stop_propagation();
                    let cur = *open.read();
                    *open.write() = !cur;
                },
                "\u{2699}\u{FE0F}"
            }

            // Floating panel (visible only when open)
            if *open.read() {
                div {
                    style: "{PANEL_STYLE}",

                    // ── Layout section ──────────────────────────────────
                    div { style: "{SECTION_LABEL_STYLE}", "Layout" }
                    div {
                        style: "{OPTION_ROW_STYLE}",
                        button {
                            style: "{option_btn_style(cur_layout == LayoutMode::Hierarchical3D)}",
                            onclick: move |_| { *layout_mode.write() = LayoutMode::Hierarchical3D; },
                            "Hierarchical 3D"
                        }
                        button {
                            style: "{option_btn_style(cur_layout == LayoutMode::Flat2D)}",
                            onclick: move |_| { *layout_mode.write() = LayoutMode::Flat2D; },
                            "Flat 2D"
                        }
                    }

                    div { style: "height: 10px;" }

                    // ── Projection section ──────────────────────────────
                    div { style: "{SECTION_LABEL_STYLE}", "Projection" }
                    div {
                        style: "{OPTION_ROW_STYLE}",
                        button {
                            style: "{option_btn_style(cur_proj == Projection::Perspective)}",
                            onclick: move |_| { *projection.write() = Projection::Perspective; },
                            "Perspective"
                        }
                        button {
                            style: "{option_btn_style(cur_proj == Projection::Orthographic)}",
                            onclick: move |_| { *projection.write() = Projection::Orthographic; },
                            "Orthographic"
                        }
                    }
                }
            }
        }
    }
}
