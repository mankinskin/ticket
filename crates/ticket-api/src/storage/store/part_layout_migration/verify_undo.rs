use super::{integrity::*, reports::*};
use crate::{
    error::StorageError,
    storage::{
        TicketStore,
        ticket_fs::{TicketFs, sha256_bytes},
    },
};

impl TicketStore {
    /// Read back every completed ticket and compare actual files with the plan.
    pub fn part_layout_verify(
        &self,
        apply: &PartLayoutApplyReport,
    ) -> Result<PartLayoutVerifyReport, StorageError> {
        self.validate_layout_report(apply.version, &apply.store_root)?;
        let mut report = PartLayoutVerifyReport {
            version: PART_LAYOUT_REPORT_VERSION,
            apply_id: apply.apply_id,
            store_root: apply.store_root.clone(),
            passed: false,
            verified_ticket_ids: Vec::new(),
            failures: Vec::new(),
            apply_failure: apply.failure.clone(),
        };

        for ticket_plan in &apply.completed_tickets {
            match self.verify_ticket_after(ticket_plan) {
                Ok(()) => report.verified_ticket_ids.push(ticket_plan.id),
                Err(reason) => report.failures.push(PartLayoutVerifyFailure {
                    ticket_id: ticket_plan.id,
                    reason,
                }),
            }
        }
        report.passed = report.failures.is_empty()
            && report.apply_failure.is_none()
            && apply.unapplied_ticket_ids.is_empty();
        Ok(report)
    }

    /// Reverse completed migration transactions unless later edits conflict.
    pub fn part_layout_undo(
        &self,
        apply: &PartLayoutApplyReport,
    ) -> Result<PartLayoutUndoReport, StorageError> {
        self.validate_layout_report(apply.version, &apply.store_root)?;
        let mut report = PartLayoutUndoReport {
            version: PART_LAYOUT_REPORT_VERSION,
            apply_id: apply.apply_id,
            store_root: apply.store_root.clone(),
            restored_ticket_ids: Vec::new(),
            not_restored_ticket_ids: Vec::new(),
            recovery_required_ticket_ids: Vec::new(),
            apply_unapplied_ticket_ids: apply.unapplied_ticket_ids.clone(),
            failure: None,
        };
        let plans = apply.completed_tickets.iter().rev().collect::<Vec<_>>();

        for (index, ticket_plan) in plans.iter().enumerate() {
            if let Err(reason) = validate_ticket_plan_integrity(ticket_plan) {
                report.failure = Some(format!(
                    "ticket {} apply report is invalid: {reason}",
                    ticket_plan.id
                ));
                report
                    .not_restored_ticket_ids
                    .extend(plans[index..].iter().map(|plan| plan.id));
                break;
            }
            let current = match self.get_indexed(&ticket_plan.id) {
                Ok(Some(current)) => current,
                Ok(None) => {
                    report.failure =
                        Some(format!("ticket {} is missing during undo", ticket_plan.id));
                    report
                        .not_restored_ticket_ids
                        .extend(plans[index..].iter().map(|plan| plan.id));
                    break;
                }
                Err(error) => {
                    report.failure = Some(format!(
                        "could not resolve ticket {} during undo: {error}",
                        ticket_plan.id
                    ));
                    report
                        .not_restored_ticket_ids
                        .extend(plans[index..].iter().map(|plan| plan.id));
                    break;
                }
            };
            match canonical_path(&current.path) {
                Ok(path) if path == ticket_plan.ticket_path => {}
                Ok(_) => {
                    report.failure = Some(format!(
                        "ticket {} moved after apply; refusing undo",
                        ticket_plan.id
                    ));
                    report
                        .not_restored_ticket_ids
                        .extend(plans[index..].iter().map(|plan| plan.id));
                    break;
                }
                Err(error) => {
                    report.failure = Some(format!(
                        "ticket {} path could not be resolved during undo: {error}",
                        ticket_plan.id
                    ));
                    report
                        .not_restored_ticket_ids
                        .extend(plans[index..].iter().map(|plan| plan.id));
                    break;
                }
            }
            let snapshot = match TicketFs::part_layout_snapshot(&current.path) {
                Ok(snapshot) => snapshot,
                Err(error) => {
                    report.failure = Some(format!(
                        "ticket {} could not be read during undo: {error}",
                        ticket_plan.id
                    ));
                    report
                        .not_restored_ticket_ids
                        .extend(plans[index..].iter().map(|plan| plan.id));
                    break;
                }
            };
            if snapshot.manifest_contents == ticket_plan.manifest_before
                && fingerprints_match(
                    &snapshot.files,
                    &internal_fingerprints(&ticket_plan.preservation),
                )
            {
                report.restored_ticket_ids.push(ticket_plan.id);
                continue;
            }
            let expected_after = match ticket_plan.manifest_after.as_deref() {
                Some(expected) if snapshot.manifest_contents == expected => expected,
                Some(_) => {
                    report.failure = Some(format!(
                        "ticket {} manifest changed after migration; refusing undo",
                        ticket_plan.id
                    ));
                    report
                        .not_restored_ticket_ids
                        .extend(plans[index..].iter().map(|plan| plan.id));
                    break;
                }
                None => {
                    report.failure = Some(format!(
                        "ticket {} apply report has no post-migration manifest",
                        ticket_plan.id
                    ));
                    report
                        .not_restored_ticket_ids
                        .extend(plans[index..].iter().map(|plan| plan.id));
                    break;
                }
            };
            let expected_after_files = expected_after_fingerprints(ticket_plan);
            if !fingerprints_match(&snapshot.files, &expected_after_files) {
                report.failure = Some(format!(
                    "ticket {} part files changed after migration; refusing undo",
                    ticket_plan.id
                ));
                report
                    .not_restored_ticket_ids
                    .extend(plans[index..].iter().map(|plan| plan.id));
                break;
            }

            let copies = ticket_plan
                .path_changes
                .iter()
                .map(|change| (change.to.clone(), change.from.clone()))
                .collect::<Vec<_>>();
            let empty_files = ticket_plan
                .cleanup_candidates
                .iter()
                .map(|candidate| candidate.part.path.clone())
                .collect::<Vec<_>>();
            let removals = ticket_plan
                .path_changes
                .iter()
                .map(|change| change.to.clone())
                .collect::<Vec<_>>();
            let expected_absent = expected_absent_after_paths(ticket_plan);

            if let Err(error) = TicketFs::publish_part_layout(
                &current.path,
                sha256_bytes(expected_after.as_bytes()).as_str(),
                &expected_after_files,
                &expected_absent,
                &copies,
                &empty_files,
                &removals,
                ticket_plan.manifest_before.as_bytes(),
            ) {
                if self.verify_ticket_before(ticket_plan).is_ok() {
                    report.restored_ticket_ids.push(ticket_plan.id);
                } else if self.verify_ticket_after(ticket_plan).is_ok() {
                    report.not_restored_ticket_ids.push(ticket_plan.id);
                } else {
                    report.recovery_required_ticket_ids.push(ticket_plan.id);
                }
                report.failure = Some(format!("ticket {}: {error}", ticket_plan.id));
                report
                    .not_restored_ticket_ids
                    .extend(plans[index + 1..].iter().map(|plan| plan.id));
                break;
            }

            match TicketFs::part_layout_snapshot(&current.path) {
                Ok(restored)
                    if restored.manifest_contents == ticket_plan.manifest_before
                        && fingerprints_match(
                            &restored.files,
                            &internal_fingerprints(&ticket_plan.preservation),
                        ) =>
                {
                    report.restored_ticket_ids.push(ticket_plan.id);
                }
                Ok(_) => {
                    report.failure = Some(format!(
                        "ticket {} undo read-back did not match the original state",
                        ticket_plan.id
                    ));
                    report.not_restored_ticket_ids.push(ticket_plan.id);
                    report.recovery_required_ticket_ids.push(ticket_plan.id);
                    report
                        .not_restored_ticket_ids
                        .extend(plans[index + 1..].iter().map(|plan| plan.id));
                    break;
                }
                Err(error) => {
                    report.failure = Some(format!(
                        "ticket {} undo read-back failed: {error}",
                        ticket_plan.id
                    ));
                    report.not_restored_ticket_ids.push(ticket_plan.id);
                    report.recovery_required_ticket_ids.push(ticket_plan.id);
                    report
                        .not_restored_ticket_ids
                        .extend(plans[index + 1..].iter().map(|plan| plan.id));
                    break;
                }
            }
        }
        Ok(report)
    }

    pub(super) fn validate_layout_report(
        &self,
        version: u32,
        store_root: &str,
    ) -> Result<(), StorageError> {
        if version != PART_LAYOUT_REPORT_VERSION {
            return Err(StorageError::Other(format!(
                "unsupported part-layout report version {version}"
            )));
        }
        let actual_root = canonical_path(&self.index_root)?;
        if actual_root != store_root {
            return Err(StorageError::Other(format!(
                "part-layout report targets store {store_root}, but this command opened {actual_root}"
            )));
        }
        Ok(())
    }

    pub(super) fn verify_ticket_after(&self, plan: &PartLayoutTicketPlan) -> Result<(), String> {
        validate_ticket_plan_integrity(plan)?;
        let indexed = self
            .get_indexed(&plan.id)
            .map_err(|error| error.to_string())?
            .ok_or_else(|| format!("ticket {} is missing", plan.id))?;
        if canonical_path(&indexed.path).map_err(|error| error.to_string())? != plan.ticket_path {
            return Err("ticket path changed after dry-run".to_string());
        }
        let snapshot =
            TicketFs::part_layout_snapshot(&indexed.path).map_err(|error| error.to_string())?;
        if plan.manifest_after.as_deref() != Some(snapshot.manifest_contents.as_str()) {
            return Err("manifest read-back does not match the planned result".to_string());
        }
        if !fingerprints_match(&snapshot.files, &expected_after_fingerprints(plan)) {
            return Err("part-file read-back does not match the planned result".to_string());
        }
        Ok(())
    }

    pub(super) fn verify_ticket_before(&self, plan: &PartLayoutTicketPlan) -> Result<(), String> {
        let indexed = self
            .get_indexed(&plan.id)
            .map_err(|error| error.to_string())?
            .ok_or_else(|| format!("ticket {} is missing", plan.id))?;
        if canonical_path(&indexed.path).map_err(|error| error.to_string())? != plan.ticket_path {
            return Err("ticket path changed".to_string());
        }
        let snapshot =
            TicketFs::part_layout_snapshot(&indexed.path).map_err(|error| error.to_string())?;
        if snapshot.manifest_contents != plan.manifest_before {
            return Err("original manifest is not present".to_string());
        }
        if !fingerprints_match(&snapshot.files, &internal_fingerprints(&plan.preservation)) {
            return Err("original files are not present".to_string());
        }
        Ok(())
    }
}
