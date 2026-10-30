#[allow(unused_imports)]
use super::*;

pub(super) fn render_kanban_overlay(overlay: &KanbanOverlay) -> Element {
    rsx! {
        div {
            id: "graph3d-kanban-guides",
            style: "position: absolute; inset: 0; pointer-events: none;",
            for (index, separator) in overlay.separators.iter().enumerate() {
                div {
                    key: "kanban-separator-{index}",
                    "data-kanban-column-separator": "{index}",
                    "data-layout-line-x1": "{layout_anchor_x(separator.x)}",
                    "data-layout-line-y1": "{layout_anchor_y(separator.top_y)}",
                    "data-layout-line-z1": "0",
                    "data-layout-line-x2": "{layout_anchor_x(separator.x)}",
                    "data-layout-line-y2": "{layout_anchor_y(separator.bottom_y)}",
                    "data-layout-line-z2": "0",
                    "data-layout-z-index": "12",
                    style: "position: absolute; display: none; pointer-events: none; height: 2px; background: linear-gradient(180deg, rgba(122, 162, 255, 0.18) 0%, rgba(122, 162, 255, 0.42) 45%, rgba(122, 162, 255, 0.18) 100%); box-shadow: 0 0 0 1px rgba(122, 162, 255, 0.08); transform-origin: 0 50%;",
                }
            }
            for column in overlay.columns.iter() {
                div {
                    key: "kanban-column-{column.state}",
                    "data-kanban-column-header": "{column.state}",
                    "data-layout-anchor-x": "{layout_anchor_x(column.x)}",
                    "data-layout-anchor-y": "{layout_anchor_y(column.y)}",
                    "data-layout-anchor-z": "0",
                    "data-layout-anchor-origin": "center-bottom",
                    "data-layout-z-index": "18",
                    style: "position: absolute; display: none; min-width: 128px; max-width: 188px; padding: 7px 12px; border-radius: 999px; border: 1px solid rgba(122, 162, 255, 0.24); background: linear-gradient(180deg, rgba(20, 26, 38, 0.92) 0%, rgba(13, 17, 28, 0.84) 100%); box-shadow: 0 10px 22px rgba(0, 0, 0, 0.24); color: rgba(232, 240, 255, 0.92); font-family: sans-serif; font-size: 11px; font-weight: 700; letter-spacing: 0.08em; line-height: 1; text-align: center; text-transform: uppercase; white-space: nowrap; transform-origin: center bottom;",
                    "{state_label(&column.state)}"
                }
            }
            for row in overlay.row_labels.iter() {
                div {
                    key: "kanban-row-{row.label}",
                    "data-kanban-row-label": "{row.label}",
                    "data-layout-anchor-x": "{layout_anchor_x(row.x)}",
                    "data-layout-anchor-y": "{layout_anchor_y(row.y)}",
                    "data-layout-anchor-z": "0",
                    "data-layout-anchor-origin": "right-center",
                    "data-layout-z-index": "16",
                    style: "position: absolute; display: none; max-width: 196px; padding: 5px 8px; border-radius: 12px; border: 1px solid rgba(255, 255, 255, 0.10); background: rgba(14, 18, 28, 0.80); box-shadow: 0 8px 20px rgba(0, 0, 0, 0.20); color: rgba(218, 224, 236, 0.86); font-family: sans-serif; font-size: 9px; font-weight: 600; line-height: 1.02; letter-spacing: 0.02em; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; transform-origin: right center;",
                    "{row.label}"
                }
            }
        }
    }
}
