#[allow(unused_imports)]
use super::*;

// ── fmt (canonical field ordering) ────────────────────────────────────────────

pub(crate) fn cmd_fmt(args: FmtArgs, store: &TicketStore) -> Result<Value, CliRunError> {
    use ticket_api::model::{filesystem::TICKET_MANIFEST_FILE, manifest_format};

    // Use the same ticket enumeration as `health --all`: iterate via the index
    // so we pick up every non-deleted ticket regardless of scan-root registration.
    let tickets = store.list(None, None, None)?;

    let mut checked = 0u64;
    let mut reformatted = 0u64;
    let mut already_ok = 0u64;
    let mut errors: Vec<Value> = Vec::new();

    for t in &tickets {
        checked += 1;
        let manifest_path = t.path.join(TICKET_MANIFEST_FILE);

        // Read raw TOML to determine whether reformatting is needed.
        let raw = match std::fs::read_to_string(&manifest_path) {
            Ok(r) => r,
            Err(e) => {
                errors.push(json!({
                    "id": t.id,
                    "path": manifest_path,
                    "error": e.to_string(),
                }));
                continue;
            }
        };

        if manifest_format::is_canonically_ordered(&raw) {
            already_ok += 1;
            continue;
        }

        if args.check {
            // Check-only mode: count but don't write.
            reformatted += 1;
        } else {
            match TicketFs::reformat(&t.path) {
                Ok(()) => reformatted += 1,
                Err(e) => {
                    errors.push(json!({
                        "id": t.id,
                        "path": manifest_path,
                        "error": e.to_string(),
                    }));
                }
            }
        }
    }

    let status = if args.check && reformatted > 0 {
        "needs_formatting"
    } else {
        "ok"
    };

    Ok(json!({
        "command": "fmt",
        "status": status,
        "check_only": args.check,
        "checked": checked,
        "reformatted": reformatted,
        "already_ok": already_ok,
        "errors": errors,
    }))
}
