use dioxus::prelude::*;

use crate::types::{
    FieldDef,
    SchemaListResponse,
    TypeSchema,
};

use super::draft::{
    save_draft,
    DraftState,
    PRIORITY_OPTIONS,
};

pub(crate) fn render_type_selector(
    schema_list: Signal<Option<SchemaListResponse>>,
    current_type_id: &str,
    mut draft: Signal<DraftState>,
) -> Element {
    rsx! {
        div {
            {render_required_label("Type")}
            if let Some(schema_list) = schema_list.read().as_ref() {
                select {
                    style: select_style(),
                    onchange: move |evt: Event<FormData>| {
                        draft.with_mut(|draft| {
                            draft.type_id = evt.value().to_string();
                            draft.extra.clear();
                            save_draft(draft);
                        });
                    },
                    option { value: "", disabled: true, selected: current_type_id.is_empty(), "— select type —" }
                    for schema in schema_list.types.iter() {
                        option {
                            key: "{schema.type_id}",
                            value: "{schema.type_id}",
                            selected: schema.type_id == current_type_id,
                            "{schema.type_id}"
                        }
                    }
                }
            } else {
                div {
                    style: "font-size: 13px; color: #6b7280; padding: 6px 0;",
                    "Loading types…"
                }
            }
        }
    }
}

pub(crate) fn render_static_fields(mut draft: Signal<DraftState>) -> Element {
    let title = draft.read().title.clone();
    let description = draft.read().description.clone();
    let priority = draft.read().priority.clone();
    let component = draft.read().component.clone();

    rsx! {
        {render_text_field(
            "Title",
            Some("*"),
            title,
            Some("Short summary of the ticket"),
            EventHandler::new(move |value: String| {
                draft.with_mut(|draft| {
                    draft.title = value;
                    save_draft(draft);
                });
            }),
        )}
        {render_textarea_field(description, EventHandler::new(move |value: String| {
            draft.with_mut(|draft| {
                draft.description = value;
                save_draft(draft);
            });
        }))}
        {render_priority_field(priority, EventHandler::new(move |value: String| {
            draft.with_mut(|draft| {
                draft.priority = value;
                save_draft(draft);
            });
        }))}
        {render_text_field(
            "Component",
            None,
            component,
            Some("e.g. ticket-viewer"),
            EventHandler::new(move |value: String| {
                draft.with_mut(|draft| {
                    draft.component = value;
                    save_draft(draft);
                });
            }),
        )}
    }
}

pub(crate) fn render_schema_fields(
    type_schema: &TypeSchema,
    draft: Signal<DraftState>,
    mut show_optional: Signal<bool>,
) -> Element {
    let required_fields = collect_fields(type_schema, true);
    let optional_fields = collect_fields(type_schema, false);

    rsx! {
        for (field_key, _) in required_fields.iter() {
            {
                render_extra_field(field_key.clone(), true, draft, true)
            }
        }
        if !optional_fields.is_empty() {
            div {
                button {
                    r#type: "button",
                    style: "
                        background: none; border: none; color: #6b7280;
                        cursor: pointer; font-size: 12px; padding: 0;
                        display: flex; align-items: center; gap: 4px;
                    ",
                    onclick: move |_| show_optional.set(!show_optional()),
                    if show_optional() { "▾" } else { "▸" }
                    " Optional fields ({optional_fields.len()})"
                }
                if show_optional() {
                    div {
                        style: "
                            margin-top: 0.75rem;
                            display: flex; flex-direction: column; gap: 0.75rem;
                        ",
                        for (field_key, _) in optional_fields.iter() {
                            {
                                render_extra_field(field_key.clone(), false, draft, false)
                            }
                        }
                    }
                }
            }
        }
    }
}

fn render_extra_field(
    field_key: String,
    required: bool,
    mut draft: Signal<DraftState>,
    highlight_error: bool,
) -> Element {
    let value = draft
        .read()
        .extra
        .get(&field_key)
        .cloned()
        .unwrap_or_default();
    let key = if required {
        format!("req-{field_key}")
    } else {
        format!("opt-{field_key}")
    };
    let border = if highlight_error {
        "1px solid #ef4444"
    } else {
        "1px solid #3b3b5a"
    };

    rsx! {
        div {
            key: "{key}",
            if required {
                {render_required_label(&field_key)}
            } else {
                {render_field_label(&field_key, None)}
            }
            input {
                r#type: "text",
                value: "{value}",
                style: "
                    width: 100%; background: #13131f; color: #e0e0e8;
                    border: {border}; border-radius: 6px;
                    padding: 6px 8px; font-size: 14px;
                    box-sizing: border-box;
                ",
                oninput: move |evt| {
                    let field_key = field_key.clone();
                    draft.with_mut(|draft| {
                        draft.extra.insert(field_key, evt.value().to_string());
                        save_draft(draft);
                    });
                },
            }
        }
    }
}

fn render_text_field(
    label: &str,
    suffix: Option<&str>,
    value: String,
    placeholder: Option<&str>,
    on_change: EventHandler<String>,
) -> Element {
    rsx! {
        div {
            {render_field_label(label, suffix)}
            input {
                r#type: "text",
                value: "{value}",
                placeholder: placeholder.unwrap_or_default(),
                style: text_input_style(),
                oninput: move |evt| on_change.call(evt.value().to_string()),
            }
        }
    }
}

fn render_textarea_field(
    value: String,
    on_change: EventHandler<String>,
) -> Element {
    rsx! {
        div {
            label {
                style: label_style(),
                "Description "
                span {
                    style: "color: #4b5563; font-size: 10px; text-transform: none; letter-spacing: 0;",
                    "(markdown)"
                }
            }
            textarea {
                placeholder: "Detailed description (markdown supported)…",
                rows: 5,
                style: "
                    width: 100%; background: #13131f; color: #e0e0e8;
                    border: 1px solid #3b3b5a; border-radius: 6px;
                    padding: 6px 8px; font-size: 14px; font-family: inherit;
                    resize: vertical; box-sizing: border-box;
                ",
                value: "{value}",
                oninput: move |evt| on_change.call(evt.value().to_string()),
            }
        }
    }
}

fn render_priority_field(
    value: String,
    on_change: EventHandler<String>,
) -> Element {
    rsx! {
        div {
            {render_field_label("Priority", None)}
            select {
                style: select_style(),
                onchange: move |evt: Event<FormData>| on_change.call(evt.value().to_string()),
                for option in PRIORITY_OPTIONS {
                    option {
                        key: "{option}",
                        value: "{option}",
                        selected: *option == value.as_str(),
                        if option.is_empty() { "— none —" } else { "{option}" }
                    }
                }
            }
        }
    }
}

fn render_required_label(label: &str) -> Element {
    render_field_label(label, Some("*"))
}

fn render_field_label(
    label: &str,
    suffix: Option<&str>,
) -> Element {
    rsx! {
        label {
            style: label_style(),
            "{label}"
            if let Some(suffix) = suffix {
                " "
                span { style: "color: #ef4444;", "{suffix}" }
            }
        }
    }
}

fn collect_fields(
    type_schema: &TypeSchema,
    required: bool,
) -> Vec<(String, FieldDef)> {
    type_schema
        .fields
        .iter()
        .filter(|(key, field_def)| {
            field_def.required == required && !is_reserved_field(key)
        })
        .map(|(key, field_def)| (key.clone(), field_def.clone()))
        .collect()
}

fn is_reserved_field(key: &str) -> bool {
    matches!(
        key,
        "title" | "description" | "priority" | "component" | "type" | "state"
    )
}

fn label_style() -> &'static str {
    "
        display: block; font-size: 11px; color: #6b7280;
        text-transform: uppercase; letter-spacing: 0.05em;
        margin-bottom: 4px;
    "
}

fn text_input_style() -> &'static str {
    "
        width: 100%; background: #13131f; color: #e0e0e8;
        border: 1px solid #3b3b5a; border-radius: 6px;
        padding: 6px 8px; font-size: 14px;
        box-sizing: border-box;
    "
}

fn select_style() -> &'static str {
    "
        width: 100%; background: #13131f; color: #e0e0e8;
        border: 1px solid #3b3b5a; border-radius: 6px;
        padding: 6px 8px; font-size: 14px;
    "
}
