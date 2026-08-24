use std::{
    collections::{BTreeMap, BTreeSet},
    env, fs,
    process::ExitCode,
};

use serde_json::Value as JsonValue;

fn main() -> ExitCode {
    let args = env::args().skip(1).collect::<Vec<_>>();
    if args.len() != 4 {
        eprintln!("ticket-record-merge: expected %O %A %B %P");
        return ExitCode::FAILURE;
    }
    let [base, ours, theirs, path]: [String; 4] = args.try_into().unwrap();
    let result = if path.ends_with("ticket.toml") {
        merge_manifest(&base, &ours, &theirs)
    } else if path.ends_with("history.ndjson") {
        merge_history(&base, &ours, &theirs)
    } else {
        Err(format!("unsupported ticket artifact {path}"))
    };
    match result {
        Ok(content) => match fs::write(&ours, content) {
            Ok(()) => ExitCode::SUCCESS,
            Err(error) => {
                eprintln!("ticket-record-merge: cannot write {ours}: {error}");
                ExitCode::FAILURE
            },
        },
        Err(error) => {
            eprintln!("ticket-record-merge: {error}");
            ExitCode::FAILURE
        },
    }
}

fn read(path: &str) -> Result<String, String> {
    fs::read_to_string(path).map_err(|error| format!("cannot read {path}: {error}"))
}

fn merge_manifest(base: &str, ours: &str, theirs: &str) -> Result<String, String> {
    let base = read(base)?.parse::<toml::Value>().map_err(|error| error.to_string())?;
    let ours = read(ours)?.parse::<toml::Value>().map_err(|error| error.to_string())?;
    let theirs = read(theirs)?.parse::<toml::Value>().map_err(|error| error.to_string())?;
    let merged = merge_toml(&base, &ours, &theirs, "ticket")?;
    toml::to_string_pretty(&merged).map_err(|error| error.to_string())
}

fn merge_toml(base: &toml::Value, ours: &toml::Value, theirs: &toml::Value, path: &str) -> Result<toml::Value, String> {
    if ours == theirs { return Ok(ours.clone()); }
    if ours == base { return Ok(theirs.clone()); }
    if theirs == base { return Ok(ours.clone()); }
    match (base, ours, theirs) {
        (toml::Value::Table(base), toml::Value::Table(ours), toml::Value::Table(theirs)) => {
            let keys = base.keys().chain(ours.keys()).chain(theirs.keys()).collect::<BTreeSet<_>>();
            let mut merged = toml::map::Map::new();
            for key in keys {
                match (base.get(key), ours.get(key), theirs.get(key)) {
                    (Some(base), Some(ours), Some(theirs)) => { merged.insert(key.clone(), merge_toml(base, ours, theirs, &format!("{path}.{key}"))?); },
                    (None, Some(value), None) | (None, None, Some(value)) => { merged.insert(key.clone(), value.clone()); },
                    (Some(_), None, None) => {},
                    _ => return Err(format!("incompatible manifest edit at {path}.{key}")),
                }
            }
            Ok(toml::Value::Table(merged))
        },
        (_, toml::Value::Array(ours), toml::Value::Array(theirs)) if path.ends_with("parts") => merge_parts(ours, theirs),
        _ => Err(format!("incompatible manifest edit at {path}")),
    }
}

fn merge_parts(ours: &[toml::Value], theirs: &[toml::Value]) -> Result<toml::Value, String> {
    let mut by_id = BTreeMap::new();
    for part in ours.iter().chain(theirs) {
        let id = part.get("id").and_then(toml::Value::as_str).ok_or("part is missing id")?;
        if let Some(existing) = by_id.insert(id.to_owned(), part.clone()) {
            if existing != *part { return Err(format!("incompatible part {id}")); }
        }
    }
    Ok(toml::Value::Array(by_id.into_values().collect()))
}

fn merge_history(base: &str, ours: &str, theirs: &str) -> Result<String, String> {
    let base = history(&read(base)?)?;
    let mut entries = BTreeMap::new();
    for entry in history(&read(ours)?)?.into_iter().chain(history(&read(theirs)?)?) {
        let key = serde_json::to_string(&entry).map_err(|error| error.to_string())?;
        if !base.iter().any(|base| base == &entry) { entries.insert(key, entry); }
    }
    let mut merged = base;
    merged.extend(entries.into_values());
    merged.sort_by(|left, right| {
        left.get("ts")
            .and_then(JsonValue::as_str)
            .cmp(&right.get("ts").and_then(JsonValue::as_str))
            .then_with(|| left.to_string().cmp(&right.to_string()))
    });
    for (index, entry) in merged.iter_mut().enumerate() { entry["rev"] = JsonValue::from((index + 1) as u64); }
    merged.into_iter().map(|entry| serde_json::to_string(&entry).map_err(|error| error.to_string())).collect::<Result<Vec<_>, _>>().map(|lines| format!("{}\n", lines.join("\n")))
}

fn history(content: &str) -> Result<Vec<JsonValue>, String> {
    content.lines().filter(|line| !line.trim().is_empty()).map(|line| serde_json::from_str(line).map_err(|error| error.to_string())).collect()
}

#[cfg(test)]
mod tests {
    use super::{
        merge_history,
        merge_toml,
    };

    #[test]
    fn preserves_distinct_parts() {
        let base: toml::Value = "parts = []".parse().unwrap();
        let ours: toml::Value = "parts = [{ id = \"one\", kind = \"note\" }]".parse().unwrap();
        let theirs: toml::Value = "parts = [{ id = \"two\", kind = \"note\" }]".parse().unwrap();
        let merged = merge_toml(&base, &ours, &theirs, "ticket").unwrap();
        assert_eq!(merged["parts"].as_array().unwrap().len(), 2);
    }

    #[test]
    fn rejects_incompatible_scalar_edit() {
        let base: toml::Value = "state = \"open\"".parse().unwrap();
        let ours: toml::Value = "state = \"in-review\"".parse().unwrap();
        let theirs: toml::Value = "state = \"done\"".parse().unwrap();
        assert!(merge_toml(&base, &ours, &theirs, "ticket").is_err());
    }

    #[test]
    fn merges_and_renumbers_independent_history() {
        let dir = tempfile::tempdir().unwrap();
        let base = dir.path().join("base");
        let ours = dir.path().join("ours");
        let theirs = dir.path().join("theirs");
        std::fs::write(&base, "{\"rev\":1,\"ts\":\"2026-01-01T00:00:00Z\",\"fields\":{}}\n").unwrap();
        std::fs::write(&ours, "{\"rev\":1,\"ts\":\"2026-01-01T00:00:00Z\",\"fields\":{}}\n{\"rev\":2,\"ts\":\"2026-01-02T00:00:00Z\",\"fields\":{\"a\":1}}\n").unwrap();
        std::fs::write(&theirs, "{\"rev\":1,\"ts\":\"2026-01-01T00:00:00Z\",\"fields\":{}}\n{\"rev\":2,\"ts\":\"2026-01-03T00:00:00Z\",\"fields\":{\"b\":1}}\n").unwrap();
        let merged = merge_history(base.to_str().unwrap(), ours.to_str().unwrap(), theirs.to_str().unwrap()).unwrap();
        assert_eq!(merged.lines().count(), 3);
        assert!(merged.contains("\"rev\":3"));
    }
}