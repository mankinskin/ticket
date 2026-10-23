use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
};

use serde::{Serialize, de::DeserializeOwned};
use serde_json::{Value, json};
use ticket_api::storage::{PartLayoutApplyReport, PartLayoutDryRunReport, TicketStore};

use crate::cli::{CliRunError, MigratePartLayoutArgs};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Operation {
    Plan,
    Apply,
    Verify,
    Undo,
}

pub(crate) fn cmd_migrate_part_layout(
    args: MigratePartLayoutArgs,
    store: &TicketStore,
    global_dry_run: bool,
) -> Result<Value, CliRunError> {
    let operation = resolve_operation(&args, global_dry_run)?;
    let input_path = match operation {
        Operation::Plan => None,
        Operation::Apply => args.plan_file.as_deref(),
        Operation::Verify | Operation::Undo => args.apply_report.as_deref(),
    };
    ensure_report_destination(&args.report_file, input_path)?;
    let report_file = super::normalize_display_path(&args.report_file);

    match operation {
        Operation::Plan => {
            let report = store.part_layout_dry_run()?;
            let status = if report.conflict_tickets == 0 {
                "planned"
            } else {
                "conflicts"
            };
            write_report(&args.report_file, &report)?;
            Ok(json!({
                "command": "migrate_part_layout",
                "status": status,
                "report_file": report_file,
                "report": report,
            }))
        }
        Operation::Apply => {
            let plan: PartLayoutDryRunReport = read_report(
                args.plan_file
                    .as_deref()
                    .ok_or_else(|| CliRunError::BadRequest("--plan-file is required".into()))?,
            )?;
            let report = store.part_layout_apply(&plan)?;
            write_report(&args.report_file, &report)?;
            if let Some(failure) = report.failure.as_deref() {
                return Err(CliRunError::BadRequest(format!(
                    "part-layout apply stopped: {failure}; partial result saved to {report_file}"
                )));
            }
            Ok(json!({
                "command": "migrate_part_layout",
                "status": "applied",
                "report_file": report_file,
                "report": report,
            }))
        }
        Operation::Verify => {
            let apply: PartLayoutApplyReport =
                read_report(args.apply_report.as_deref().ok_or_else(|| {
                    CliRunError::BadRequest("--apply-report is required".into())
                })?)?;
            let report = store.part_layout_verify(&apply)?;
            write_report(&args.report_file, &report)?;
            if !report.passed {
                return Err(CliRunError::BadRequest(format!(
                    "part-layout verification failed; report saved to {report_file}"
                )));
            }
            Ok(json!({
                "command": "migrate_part_layout",
                "status": "verified",
                "report_file": report_file,
                "report": report,
            }))
        }
        Operation::Undo => {
            let apply: PartLayoutApplyReport =
                read_report(args.apply_report.as_deref().ok_or_else(|| {
                    CliRunError::BadRequest("--apply-report is required".into())
                })?)?;
            let report = store.part_layout_undo(&apply)?;
            write_report(&args.report_file, &report)?;
            if report.failure.is_some() {
                return Err(CliRunError::BadRequest(format!(
                    "part-layout undo did not fully complete; report saved to {report_file}"
                )));
            }
            Ok(json!({
                "command": "migrate_part_layout",
                "status": "undone",
                "report_file": report_file,
                "report": report,
            }))
        }
    }
}

pub(crate) fn cmd_migrate_part_layout_dry_run(
    args: MigratePartLayoutArgs,
    index_root: &Path,
) -> Result<Value, CliRunError> {
    let _ = resolve_operation(&args, true)?;
    ensure_report_destination(&args.report_file, None)?;
    let report_file = super::normalize_display_path(&args.report_file);
    let report = TicketStore::part_layout_dry_run_from_filesystem(index_root)?;
    let status = if report.conflict_tickets == 0 {
        "planned"
    } else {
        "conflicts"
    };
    write_report(&args.report_file, &report)?;
    Ok(json!({
        "command": "migrate_part_layout",
        "status": status,
        "report_file": report_file,
        "report": report,
    }))
}

fn resolve_operation(
    args: &MigratePartLayoutArgs,
    global_dry_run: bool,
) -> Result<Operation, CliRunError> {
    let modes = [args.apply, args.verify, args.undo]
        .into_iter()
        .filter(|enabled| *enabled)
        .count();
    if global_dry_run {
        if modes != 0 || args.plan_file.is_some() || args.apply_report.is_some() {
            return Err(CliRunError::BadRequest(
                "--dry-run selects plan mode and cannot be combined with --apply, --verify, --undo, --plan-file, or --apply-report".into(),
            ));
        }
        return Ok(Operation::Plan);
    }
    if modes != 1 {
        return Err(CliRunError::BadRequest(
            "select exactly one of --apply, --verify, or --undo; use --dry-run for plan mode"
                .into(),
        ));
    }

    match (args.apply, args.verify, args.undo) {
        (true, false, false) if args.plan_file.is_some() && args.apply_report.is_none() => {
            Ok(Operation::Apply)
        }
        (false, true, false) if args.apply_report.is_some() && args.plan_file.is_none() => {
            Ok(Operation::Verify)
        }
        (false, false, true) if args.apply_report.is_some() && args.plan_file.is_none() => {
            Ok(Operation::Undo)
        }
        (true, false, false) => Err(CliRunError::BadRequest(
            "--apply requires --plan-file and does not accept --apply-report".into(),
        )),
        (false, true, false) => Err(CliRunError::BadRequest(
            "--verify requires --apply-report and does not accept --plan-file".into(),
        )),
        (false, false, true) => Err(CliRunError::BadRequest(
            "--undo requires --apply-report and does not accept --plan-file".into(),
        )),
        _ => Err(CliRunError::BadRequest(
            "select exactly one of --apply, --verify, or --undo".into(),
        )),
    }
}

fn ensure_report_destination(
    report_file: &Path,
    input_file: Option<&Path>,
) -> Result<(), CliRunError> {
    match fs::symlink_metadata(report_file) {
        Ok(_) => {
            return Err(CliRunError::BadRequest(format!(
                "report destination already exists: {}",
                report_file.display()
            )));
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => {
            return Err(CliRunError::BadRequest(format!(
                "cannot inspect report destination {}: {error}",
                report_file.display()
            )));
        }
    }
    if let Some(input_file) = input_file {
        let report_identity = normalized_future_path(report_file)?;
        let input_identity = fs::canonicalize(input_file).map_err(|error| {
            CliRunError::BadRequest(format!(
                "cannot resolve input report {}: {error}",
                input_file.display()
            ))
        })?;
        if report_identity == input_identity {
            return Err(CliRunError::BadRequest(
                "report destination must not overwrite its input report".into(),
            ));
        }
    }
    Ok(())
}

fn normalized_future_path(path: &Path) -> Result<PathBuf, CliRunError> {
    let file_name = path.file_name().ok_or_else(|| {
        CliRunError::BadRequest(format!(
            "report destination has no file name: {}",
            path.display()
        ))
    })?;
    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let parent = fs::canonicalize(parent).map_err(|error| {
        CliRunError::BadRequest(format!(
            "cannot resolve report directory {}: {error}",
            parent.display()
        ))
    })?;
    Ok(parent.join(file_name))
}

fn read_report<T: DeserializeOwned>(path: &Path) -> Result<T, CliRunError> {
    let bytes = fs::read(path).map_err(|error| {
        CliRunError::BadRequest(format!("cannot read report {}: {error}", path.display()))
    })?;
    serde_json::from_slice(&bytes).map_err(|error| {
        CliRunError::BadRequest(format!(
            "cannot parse report {} as JSON: {error}",
            path.display()
        ))
    })
}

fn write_report<T: Serialize>(path: &Path, report: &T) -> Result<(), CliRunError> {
    let bytes = serde_json::to_vec_pretty(report).map_err(|error| {
        CliRunError::BadRequest(format!(
            "cannot serialize report {}: {error}",
            path.display()
        ))
    })?;
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|error| {
            CliRunError::BadRequest(format!(
                "cannot create report {} without overwriting existing data: {error}",
                path.display()
            ))
        })?;
    file.write_all(&bytes).map_err(|error| {
        CliRunError::BadRequest(format!("cannot write report {}: {error}", path.display()))
    })?;
    file.sync_all().map_err(|error| {
        CliRunError::BadRequest(format!("cannot sync report {}: {error}", path.display()))
    })?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    fn args(apply: bool, verify: bool, undo: bool) -> MigratePartLayoutArgs {
        MigratePartLayoutArgs {
            apply,
            verify,
            undo,
            plan_file: None,
            apply_report: None,
            report_file: PathBuf::from("report.json"),
        }
    }

    #[test]
    fn global_dry_run_selects_plan_mode() {
        assert_eq!(
            resolve_operation(&args(false, false, false), true).unwrap(),
            Operation::Plan
        );
    }

    #[test]
    fn modes_require_exactly_one_operation() {
        assert!(resolve_operation(&args(false, false, false), false).is_err());
        assert!(resolve_operation(&args(true, true, false), false).is_err());
        assert!(resolve_operation(&args(true, false, false), true).is_err());
    }

    #[test]
    fn clap_rejects_multiple_operation_flags() {
        let parsed = crate::cli::TicketCli::try_parse_from([
            "ticket",
            "--workspace",
            ".",
            "migrate-part-layout",
            "--apply",
            "--verify",
            "--plan-file",
            "plan.json",
            "--apply-report",
            "apply.json",
            "--report-file",
            "report.json",
        ]);
        assert!(parsed.is_err());
    }

    #[test]
    fn machine_report_is_written_once_without_overwriting() {
        let directory = tempfile::tempdir().unwrap();
        let report_path = directory.path().join("report.json");
        let expected = json!({"status": "planned", "count": 3});

        write_report(&report_path, &expected).unwrap();
        let actual: Value = read_report(&report_path).unwrap();

        assert_eq!(actual, expected);
        assert!(write_report(&report_path, &expected).is_err());
    }

    #[test]
    fn report_destination_must_not_alias_its_input() {
        let directory = tempfile::tempdir().unwrap();
        let input = directory.path().join("plan.json");
        fs::write(&input, b"{}").unwrap();

        assert!(ensure_report_destination(&input, Some(&input)).is_err());
        ensure_report_destination(&directory.path().join("apply.json"), Some(&input)).unwrap();
    }

    #[test]
    fn plan_and_apply_commands_persist_machine_reports() {
        let directory = tempfile::tempdir().unwrap();
        let store = TicketStore::init(directory.path()).unwrap();
        store
            .create(
                None,
                "tracker-improvement",
                Some("CLI migration report"),
                Some("planning"),
                Default::default(),
                None,
                None,
            )
            .unwrap();
        let plan_path = directory.path().join("plan.json");
        let plan_output = directory.path().join("plan-result.json");
        let plan_payload = cmd_migrate_part_layout(
            MigratePartLayoutArgs {
                apply: false,
                verify: false,
                undo: false,
                plan_file: None,
                apply_report: None,
                report_file: plan_path.clone(),
            },
            &store,
            true,
        )
        .unwrap();
        assert_eq!(plan_payload["status"], "planned");
        let plan: PartLayoutDryRunReport = read_report(&plan_path).unwrap();

        let apply_payload = cmd_migrate_part_layout(
            MigratePartLayoutArgs {
                apply: true,
                verify: false,
                undo: false,
                plan_file: Some(plan_path),
                apply_report: None,
                report_file: plan_output.clone(),
            },
            &store,
            false,
        )
        .unwrap();
        assert_eq!(apply_payload["status"], "applied");
        let apply: PartLayoutApplyReport = read_report(&plan_output).unwrap();
        assert_eq!(apply.plan_id, plan.plan_id);
        assert!(apply.failure.is_none());

        let verify_path = directory.path().join("verify-result.json");
        let verify_payload = cmd_migrate_part_layout(
            MigratePartLayoutArgs {
                apply: false,
                verify: true,
                undo: false,
                plan_file: None,
                apply_report: Some(plan_output.clone()),
                report_file: verify_path.clone(),
            },
            &store,
            false,
        )
        .unwrap();
        assert_eq!(verify_payload["status"], "verified");
        let verify: ticket_api::storage::PartLayoutVerifyReport =
            read_report(&verify_path).unwrap();
        assert!(verify.passed);

        let undo_path = directory.path().join("undo-result.json");
        let undo_payload = cmd_migrate_part_layout(
            MigratePartLayoutArgs {
                apply: false,
                verify: false,
                undo: true,
                plan_file: None,
                apply_report: Some(plan_output),
                report_file: undo_path.clone(),
            },
            &store,
            false,
        )
        .unwrap();
        assert_eq!(undo_payload["status"], "undone");
        let undo: ticket_api::storage::PartLayoutUndoReport = read_report(&undo_path).unwrap();
        assert!(undo.failure.is_none());
    }
}
