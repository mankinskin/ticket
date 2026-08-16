pub(super) fn fields_to_toml(
    id: &str,
    fields: &serde_json::Value,
) -> String {
    let mut lines: Vec<String> = vec![format!("# {id}"), String::new()];

    if let Some(map) = fields.as_object() {
        for (key, value) in map {
            let formatted = match value {
                serde_json::Value::String(text) => {
                    let escaped = text.replace('"', "\\\"");
                    format!("{key} = \"{escaped}\"")
                },
                serde_json::Value::Null => format!("{key} = \"\""),
                serde_json::Value::Bool(flag) => format!("{key} = {flag}"),
                serde_json::Value::Number(number) =>
                    format!("{key} = {number}"),
                other => format!("{key} = {other}"),
            };
            lines.push(formatted);
        }
    }

    lines.join("\n")
}

pub(super) fn content_tab_label(
    asset_path: Option<&str>,
    is_description: bool,
) -> String {
    if is_description {
        return "Document".to_string();
    }

    asset_path
        .and_then(|path| path.rsplit('/').next())
        .unwrap_or("Content")
        .to_string()
}
