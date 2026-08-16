//! TicketCard — DOM element rendered as a graph node.
//!
//! Each card is positioned absolutely by the DepGraph layout engine.
//! The card's border-left colour signals the ticket state.  Clicking opens
//! the TicketDetailPage; pressing mouse-down starts a drag.

use dioxus::prelude::*;

/// Map a ticket state string to a CSS hex colour for the card's left border.
pub fn state_color(state: Option<&str>) -> &'static str {
    crate::types::state_accent(state)
}

/// Map a priority string to a CSS hex colour for the priority badge.
fn priority_color(p: Option<&str>) -> &'static str {
    match p {
        Some("critical") => "#ef4444",
        Some("high") => "#f97316",
        Some("medium") => "#eab308",
        Some("low") => "#3b82f6",
        Some("none") | None | Some("") => "#6b7280",
        _ => "#6b7280",
    }
}

/// Abbreviate a ticket type string so it fits in a compact badge.
fn short_type(t: Option<&str>) -> &'static str {
    match t {
        Some("tracker-improvement") => "task",
        Some("bug") => "bug",
        Some("feature") => "feat",
        Some("epic") => "epic",
        Some("chore") => "chore",
        _ => "tkt",
    }
}

// ── Props ──────────────────────────────────────────────────────────────────

#[derive(Props, Clone, PartialEq)]
pub struct TicketCardProps {
    pub id: String,
    pub title: Option<String>,
    pub state: Option<String>,
    pub depth: usize,
    pub priority: Option<String>,
    pub ticket_type: Option<String>,
    /// Layout-space pixel position of the card centre.
    pub layout_x: f64,
    pub layout_y: f64,
    pub pan_x: f64,
    pub pan_y: f64,
    pub zoom: f64,
    pub canvas_w: f64,
    pub canvas_h: f64,
    pub on_click: EventHandler<String>,
    pub on_drag_start: EventHandler<(String, f64, f64)>,
}

// ── Component ──────────────────────────────────────────────────────────────

/// A single ticket node rendered as an absolutely-positioned DOM element.
#[component]
pub fn TicketCard(props: TicketCardProps) -> Element {
    let color = state_color(props.state.as_deref());
    let title = props.title.as_deref().unwrap_or("Untitled");
    let state_label = props.state.as_deref().unwrap_or("—");
    let prio_color = priority_color(props.priority.as_deref());
    let prio_label = props.priority.as_deref().unwrap_or("—");
    let type_label = short_type(props.ticket_type.as_deref());

    // Convert layout-space position to screen pixels relative to the
    // dep-graph container (which has its origin at the container centre).
    let screen_x =
        props.canvas_w / 2.0 + (props.layout_x + props.pan_x) * props.zoom;
    let screen_y =
        props.canvas_h / 2.0 + (props.layout_y + props.pan_y) * props.zoom;

    const CARD_W: f64 = 220.0;
    const CARD_H: f64 = 80.0;
    let card_w_px = CARD_W * props.zoom.clamp(0.4, 2.5);
    let card_h_px = CARD_H * props.zoom.clamp(0.4, 2.5);

    // Position the card so its centre aligns with screen_x / screen_y.
    let left = screen_x - card_w_px / 2.0;
    let top = screen_y - card_h_px / 2.0;

    let font_scale = props.zoom.clamp(0.55, 1.5);

    // Short ID suffix (last 8 chars) shown in the card footer.
    let short_id = if props.id.len() > 8 {
        &props.id[props.id.len() - 8..]
    } else {
        &props.id
    };

    let id = props.id.clone();
    let id_drag = props.id.clone();

    rsx! {
        div {
            key: "{props.id}",
            style: "
                position: absolute;
                left: {left}px;
                top: {top}px;
                width: {card_w_px}px;
                height: {card_h_px}px;
                box-sizing: border-box;
                border: 1px solid rgba(200,200,200,0.25);
                border-left: 3px solid {color};
                border-radius: 6px;
                background: rgba(28,28,38,0.92);
                backdrop-filter: blur(2px);
                padding: 6px 8px;
                cursor: pointer;
                user-select: none;
                overflow: hidden;
                font-family: var(--font-mono, monospace);
                box-shadow: 0 2px 10px rgba(0,0,0,0.55);
                transition: box-shadow 0.1s;
                z-index: 10;
                display: flex;
                flex-direction: column;
                gap: 3px;
            ",
            onmousedown: move |evt: Event<MouseData>| {
                evt.stop_propagation();
                let client = evt.client_coordinates();
                props.on_drag_start.call((id_drag.clone(), client.x, client.y));
            },
            onclick: move |evt: Event<MouseData>| {
                evt.stop_propagation();
                props.on_click.call(id.clone());
            },

            // ── Row 1: title ──────────────────────────────────────────
            div {
                style: "
                    font-size: {13.0 * font_scale}px;
                    font-weight: 600;
                    color: #e8e8ee;
                    overflow: hidden;
                    text-overflow: ellipsis;
                    white-space: nowrap;
                    line-height: 1.3;
                ",
                "{title}"
            }

            // ── Row 2: type badge · state badge ───────────────────────
            div {
                style: "
                    display: flex;
                    align-items: center;
                    gap: 5px;
                    overflow: hidden;
                    white-space: nowrap;
                ",
                // Type badge
                span {
                    style: "
                        font-size: {9.0 * font_scale}px;
                        font-weight: 600;
                        padding: 1px 5px;
                        border-radius: 3px;
                        background: rgba(120,120,160,0.25);
                        color: rgba(180,180,210,0.9);
                        text-transform: uppercase;
                        letter-spacing: 0.04em;
                        flex-shrink: 0;
                    ",
                    "{type_label}"
                }
                // State badge (coloured)
                span {
                    style: "
                        font-size: {9.0 * font_scale}px;
                        font-weight: 600;
                        color: {color};
                        text-transform: uppercase;
                        letter-spacing: 0.05em;
                        overflow: hidden;
                        text-overflow: ellipsis;
                        white-space: nowrap;
                    ",
                    "{state_label}"
                }
            }

            // ── Row 3: priority · short id ────────────────────────────
            div {
                style: "
                    display: flex;
                    align-items: center;
                    justify-content: space-between;
                    overflow: hidden;
                    white-space: nowrap;
                    margin-top: auto;
                ",
                span {
                    style: "
                        font-size: {9.0 * font_scale}px;
                        font-weight: 600;
                        color: {prio_color};
                        text-transform: uppercase;
                        letter-spacing: 0.04em;
                        flex-shrink: 0;
                    ",
                    "▲ {prio_label}"
                }
                span {
                    style: "
                        font-size: {8.5 * font_scale}px;
                        color: rgba(140,140,160,0.6);
                        font-family: monospace;
                        overflow: hidden;
                        text-overflow: ellipsis;
                        white-space: nowrap;
                        margin-left: 6px;
                    ",
                    "…{short_id}"
                }
            }
        }
    }
}
