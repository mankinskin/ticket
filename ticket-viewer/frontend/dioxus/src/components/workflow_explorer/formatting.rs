#[allow(unused_imports)]
use super::*;

pub(super) fn workflow_evidence(
    became_actionable_at: Option<&str>,
    last_blocker_progress_at: Option<&str>,
) -> Option<String> {
    match (
        format_workflow_timestamp(became_actionable_at),
        format_workflow_timestamp(last_blocker_progress_at),
    ) {
        (Some(actionable), Some(progress)) => Some(format!(
            "Actionable since {actionable}; last blocker progress {progress}"
        )),
        (Some(actionable), None) => Some(format!("Actionable since {actionable}")),
        (None, Some(progress)) => Some(format!("Last blocker progress {progress}")),
        (None, None) => None,
    }
}

pub(super) fn format_workflow_timestamp(value: Option<&str>) -> Option<String> {
    let value = value?;
    let (date, time_with_zone) = value.split_once('T')?;
    let mut cutoff = time_with_zone.len();
    for marker in ['.', '+', 'Z'] {
        if let Some(index) = time_with_zone.find(marker) {
            cutoff = cutoff.min(index);
        }
    }
    let time = &time_with_zone[..cutoff];
    let suffix = if value.ends_with("+00:00") || value.ends_with('Z') {
        " UTC"
    } else {
        ""
    };
    Some(format!("{date} {time}{suffix}"))
}

pub(super) fn ticket_ref_label(ticket_ref: &TicketRef) -> String {
    format!("{} ({})", ticket_ref.id, ticket_ref.workspace)
}

pub(super) fn title_or_untitled(title: Option<&str>) -> &str {
    title.unwrap_or("Untitled")
}

pub(super) fn short_id(id: &str) -> String {
    id.chars().take(8).collect()
}

pub(super) fn render_short_id_list(ids: &[String]) -> String {
    ids.iter()
        .map(|id| short_id(id))
        .collect::<Vec<_>>()
        .join(", ")
}

pub(super) fn panel_style(highlight: bool) -> String {
    format!(
        "
            display: flex;
            flex-direction: column;
            gap: 6px;
            width: 100%;
            padding: 10px 12px;
            border: 1px solid {};
            border-radius: 10px;
            background: {};
            color: var(--text-primary);
            cursor: pointer;
            text-align: left;
        ",
        if highlight {
            "color-mix(in srgb, var(--accent-blue) 50%, var(--border-subtle))"
        } else {
            "var(--border-subtle)"
        },
        if highlight {
            "color-mix(in srgb, var(--accent-blue) 10%, var(--bg-secondary))"
        } else {
            "var(--bg-secondary)"
        },
    )
}

pub(super) fn frontier_pill_style() -> &'static str {
    "
        padding: 2px 6px;
        border-radius: 999px;
        background: color-mix(in srgb, var(--accent-blue) 18%, transparent);
        color: var(--accent-blue);
        font-size: 10px;
        font-weight: 700;
        letter-spacing: 0.02em;
        text-transform: uppercase;
    "
}

pub(super) fn rank_pill_style(frontier: bool) -> &'static str {
    if frontier {
        "
            min-width: 28px;
            padding: 2px 6px;
            border-radius: 999px;
            background: color-mix(in srgb, var(--accent-blue) 20%, transparent);
            color: var(--accent-blue);
            font-size: 10px;
            font-weight: 700;
            text-align: center;
        "
    } else {
        "
            min-width: 28px;
            padding: 2px 6px;
            border-radius: 999px;
            background: color-mix(in srgb, var(--text-muted) 20%, transparent);
            color: var(--text-secondary);
            font-size: 10px;
            font-weight: 700;
            text-align: center;
        "
    }
}

pub(super) fn evidence_style() -> &'static str {
    "
        font-size: 11px;
        color: var(--text-muted);
        width: 100%;
        text-align: left;
        line-height: 1.4;
    "
}

pub(super) fn empty_panel_style() -> &'static str {
    "
        padding: 12px;
        border: 1px dashed var(--border-subtle);
        border-radius: 8px;
        color: var(--text-muted);
        font-size: 12px;
        line-height: 1.4;
    "
}
