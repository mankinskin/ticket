use dioxus::prelude::*;

use super::{
    actions::{
        apply_batch_operation,
        toggle_pending_op,
    },
    model::{
        BulkOp,
        CommandResult,
        PRIORITY_OPS,
        STATE_OPS,
    },
    results::render_results_panel,
};

#[derive(Props, Clone, PartialEq)]
pub struct BatchPanelProps {
    pub workspace: String,
    pub selected_ids: Vec<String>,
    pub on_done: EventHandler<()>,
}

#[component]
pub fn BatchPanel(props: BatchPanelProps) -> Element {
    let pending_op: Signal<Option<BulkOp>> = use_signal(|| None);
    let running: Signal<bool> = use_signal(|| false);
    let results: Signal<Vec<CommandResult>> = use_signal(Vec::new);
    let global_error: Signal<Option<String>> = use_signal(|| None);

    let count = props.selected_ids.len();
    let result_rows = results.read().clone();
    let has_results = !result_rows.is_empty();

    rsx! {
        div {
            style: "
                position: fixed;
                left: 0; right: 0; bottom: 0;
                z-index: 200;
                display: flex;
                flex-direction: column;
                align-items: stretch;
                pointer-events: none;
            ",
            if has_results {
                {render_results_panel(&result_rows, props.on_done.clone())}
            }
            if !has_results {
                {render_action_bar(
                    props.workspace.clone(),
                    props.selected_ids.clone(),
                    props.on_done.clone(),
                    count,
                    pending_op,
                    running,
                    results,
                    global_error,
                )}
            }
        }
    }
}

fn render_action_bar(
    workspace: String,
    selected_ids: Vec<String>,
    on_done: EventHandler<()>,
    count: usize,
    pending_op: Signal<Option<BulkOp>>,
    running: Signal<bool>,
    results: Signal<Vec<CommandResult>>,
    global_error: Signal<Option<String>>,
) -> Element {
    let is_running = *running.read();
    let op_ready = pending_op.read().is_some();
    let btn_opacity = if is_running { "0.5" } else { "1" };
    let apply_bg = if op_ready && !is_running {
        "var(--accent-blue)"
    } else {
        "var(--bg-tertiary, #2a2a3a)"
    };
    let apply_cursor = if op_ready && !is_running {
        "pointer"
    } else {
        "default"
    };
    let apply_opacity = if op_ready && !is_running { "1" } else { "0.5" };
    let apply_label = if is_running { "Applying…" } else { "Apply" };

    rsx! {
        div {
            style: "
                pointer-events: all;
                background: var(--bg-secondary, #13131f);
                border-top: 2px solid var(--accent-blue, #3b82f6);
                padding: 10px 16px;
                display: flex;
                flex-wrap: wrap;
                gap: 8px;
                align-items: center;
            ",
            span {
                style: "font-size: 13px; font-weight: 600; color: var(--text-primary); margin-right: 4px;",
                "{count} selected"
            }
            if let Some(error) = global_error.read().as_deref() {
                span {
                    style: "font-size: 11px; color: var(--error, #f87171);",
                    "{error}"
                }
            }
            div { style: "flex: 1;" }
            for &(label, state_value) in STATE_OPS.iter() {
                {
                    render_operation_button(
                        state_value.to_string(),
                        label,
                        BulkOp::SetState(state_value.to_string()),
                        "var(--accent-blue)",
                        pending_op,
                        global_error,
                        is_running,
                        btn_opacity,
                    )
                }
            }
            {render_separator()}
            for &(label, priority) in PRIORITY_OPS.iter() {
                {
                    render_operation_button(
                        format!("prio-{priority}"),
                        label,
                        BulkOp::SetPriority(priority.to_string()),
                        "var(--accent-purple, #a855f7)",
                        pending_op,
                        global_error,
                        is_running,
                        btn_opacity,
                    )
                }
            }
            {render_separator()}
            {render_operation_button(
                "close".to_string(),
                "✓ Close",
                BulkOp::Close,
                "var(--success, #22c55e)",
                pending_op,
                global_error,
                is_running,
                btn_opacity,
            )}
            {render_operation_button(
                "cancel".to_string(),
                "✗ Cancel tickets",
                BulkOp::Cancel,
                "var(--error, #ef4444)",
                pending_op,
                global_error,
                is_running,
                btn_opacity,
            )}
            {render_separator()}
            button {
                disabled: !op_ready || is_running,
                style: "
                    padding: 6px 16px; border-radius: 6px; border: none;
                    background: {apply_bg};
                    color: white; cursor: {apply_cursor};
                    font-size: 12px; font-weight: 700;
                    opacity: {apply_opacity};
                ",
                onclick: move |_| {
                    let Some(op) = pending_op.read().clone() else {
                        return;
                    };
                    apply_batch_operation(
                        workspace.clone(),
                        selected_ids.clone(),
                        op,
                        running,
                        results,
                        pending_op,
                        global_error,
                    );
                },
                "{apply_label}"
            }
            button {
                disabled: is_running,
                style: "
                    padding: 6px 12px; border-radius: 6px;
                    border: 1px solid var(--border-subtle, #333);
                    background: transparent;
                    color: var(--text-muted); cursor: pointer;
                    font-size: 12px; font-weight: 600;
                    opacity: {btn_opacity};
                ",
                onclick: move |_| on_done.call(()),
                "✕ Deselect"
            }
        }
    }
}

fn render_operation_button(
    key: String,
    label: &str,
    op: BulkOp,
    active_bg: &str,
    pending_op: Signal<Option<BulkOp>>,
    global_error: Signal<Option<String>>,
    is_running: bool,
    btn_opacity: &str,
) -> Element {
    let is_pending = *pending_op.read() == Some(op.clone());
    let bg = if is_pending {
        active_bg
    } else {
        "var(--bg-primary, #1a1a2e)"
    };

    rsx! {
        button {
            key: "{key}",
            disabled: is_running,
            style: "
                padding: 5px 10px; border-radius: 5px;
                border: 1px solid var(--border-subtle, #333);
                background: {bg};
                color: var(--text-primary); cursor: pointer;
                font-size: 11px; font-weight: 600;
                opacity: {btn_opacity};
            ",
            onclick: move |_| toggle_pending_op(pending_op, global_error, op.clone()),
            "{label}"
        }
    }
}

fn render_separator() -> Element {
    rsx! {
        span { style: "color: var(--border-subtle, #333); font-size: 14px;", "|" }
    }
}
