#[allow(unused_imports)]
use super::*;

// ── health report ──────────────────────────────────────────────────────────────

pub(super) fn render_health_report(obj: &serde_json::Map<String, Value>) -> String {
    let mut out = String::new();

    let checked = obj
        .get("tickets_checked")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let finding_count = obj
        .get("finding_count")
        .and_then(Value::as_u64)
        .unwrap_or(0);

    let _ = writeln!(
        out,
        "Health check: {checked} tickets checked, {finding_count} finding(s)\n"
    );

    if let Some(findings) = obj.get("findings").and_then(Value::as_array) {
        let severity_order = |s: &str| -> u8 {
            match s {
                "error" => 0,
                "warning" => 1,
                _ => 2,
            }
        };

        let mut sorted: Vec<&Value> = findings.iter().collect();
        sorted.sort_by(|a, b| {
            let sa = a.get("severity").and_then(Value::as_str).unwrap_or("info");
            let sb = b.get("severity").and_then(Value::as_str).unwrap_or("info");
            severity_order(sa).cmp(&severity_order(sb))
        });

        for f in &sorted {
            let severity = f.get("severity").and_then(Value::as_str).unwrap_or("info");
            let check = f.get("check").and_then(Value::as_str).unwrap_or("?");
            let short_id = f.get("short_id").and_then(Value::as_str).unwrap_or("?");
            let title = f.get("title").and_then(Value::as_str).unwrap_or("?");
            let message = f.get("message").and_then(Value::as_str).unwrap_or("");

            let icon = match severity {
                "error" => "ERR ",
                "warning" => "WARN",
                _ => "INFO",
            };
            let _ = writeln!(out, "[{icon}] {short_id} {title}");
            let _ = writeln!(out, "       {check}: {message}");
        }
    }

    if finding_count == 0 {
        let _ = writeln!(out, "All checks passed.");
    } else {
        let _ = writeln!(out);
        // Summary line
        if let Some(summary) = obj.get("summary").and_then(Value::as_object) {
            let parts: Vec<String> = summary
                .iter()
                .map(|(k, v)| format!("{k}: {}", v.as_u64().unwrap_or(0)))
                .collect();
            let _ = writeln!(out, "Summary: {}", parts.join(", "));
        }
    }

    out
}
