use super::{
    integrity::{canonical_path, internal_fingerprints, ticket_path_is_within_store},
    planning::*,
    reports::*,
};
use crate::{
    error::StorageError,
    storage::{TicketStore, ticket_fs::TicketFs},
};
use uuid::Uuid;

impl TicketStore {
    /// Plan deterministic part-path renames and eligible empty-part removals.
    ///
    /// The scan does not write to the store. Its report fingerprints every
    /// manifest-referenced part and discovered orphan for later stale-input
    /// checks and preservation verification.
    pub fn part_layout_dry_run(&self) -> Result<PartLayoutDryRunReport, StorageError> {
        let store_root = canonical_path(&self.index_root)?;
        let tickets_root = canonical_path(&self.index_root.join("tickets"))?;
        let indexed = self.list(None, None, None)?;
        let mut tickets = Vec::with_capacity(indexed.len());
        for ticket in indexed {
            let ticket_path = canonical_path(&ticket.path)?;
            if !ticket_path_is_within_store(&ticket_path, &tickets_root) {
                continue;
            }
            let snapshot = TicketFs::part_layout_snapshot_read_only(&ticket.path)?;
            tickets.push(build_ticket_plan(ticket, ticket_path, snapshot)?);
        }

        let scanned_tickets = tickets.len();
        let planned_tickets = tickets
            .iter()
            .filter(|ticket| ticket.status == PartLayoutTicketStatus::Planned)
            .count();
        let unchanged_tickets = tickets
            .iter()
            .filter(|ticket| ticket.status == PartLayoutTicketStatus::Unchanged)
            .count();
        let conflict_tickets = tickets
            .iter()
            .filter(|ticket| ticket.status == PartLayoutTicketStatus::Conflict)
            .count();
        let renamed_parts = tickets.iter().map(|ticket| ticket.path_changes.len()).sum();
        let cleanup_candidates = tickets
            .iter()
            .map(|ticket| ticket.cleanup_candidates.len())
            .sum();
        let skipped_parts = tickets
            .iter()
            .map(|ticket| ticket.skipped_parts.len())
            .sum();

        Ok(PartLayoutDryRunReport {
            version: PART_LAYOUT_REPORT_VERSION,
            plan_id: Uuid::new_v4(),
            store_root,
            scanned_tickets,
            planned_tickets,
            unchanged_tickets,
            conflict_tickets,
            renamed_parts,
            cleanup_candidates,
            skipped_parts,
            tickets,
        })
    }

    /// Apply a saved dry-run plan using one recoverable transaction per ticket.
    ///
    /// A stopped multi-ticket operation returns its completed and unapplied
    /// ticket ids in the report rather than hiding a partial result.
    pub fn part_layout_apply(
        &self,
        plan: &PartLayoutDryRunReport,
    ) -> Result<PartLayoutApplyReport, StorageError> {
        self.validate_layout_report(plan.version, &plan.store_root)?;
        self.validate_ticket_paths(&plan.tickets)?;

        let planned = plan
            .tickets
            .iter()
            .filter(|ticket| ticket.status == PartLayoutTicketStatus::Planned)
            .collect::<Vec<_>>();
        let mut report = PartLayoutApplyReport {
            version: PART_LAYOUT_REPORT_VERSION,
            apply_id: Uuid::new_v4(),
            plan_id: plan.plan_id,
            store_root: plan.store_root.clone(),
            completed_tickets: Vec::new(),
            unapplied_ticket_ids: planned.iter().map(|ticket| ticket.id).collect(),
            recovery_required_ticket_ids: Vec::new(),
            failure: None,
        };

        if plan.conflict_tickets > 0 {
            report.failure = Some(format!(
                "dry-run report contains {} conflicting ticket(s); no changes were applied",
                plan.conflict_tickets
            ));
            return Ok(report);
        }

        for ticket_plan in planned {
            // A prior invocation may have committed this ticket before its
            // store-wide apply report could be written.
            if self.verify_ticket_after(ticket_plan).is_ok() {
                report.completed_tickets.push(ticket_plan.clone());
                report
                    .unapplied_ticket_ids
                    .retain(|id| *id != ticket_plan.id);
                continue;
            }

            let current = match self.get_indexed(&ticket_plan.id) {
                Ok(Some(current)) => current,
                Ok(None) => {
                    report.failure = Some(format!(
                        "ticket {} disappeared after dry-run",
                        ticket_plan.id
                    ));
                    break;
                }
                Err(error) => {
                    report.failure = Some(format!(
                        "could not resolve ticket {} after dry-run: {error}",
                        ticket_plan.id
                    ));
                    break;
                }
            };
            match canonical_path(&current.path) {
                Ok(path) if path == ticket_plan.ticket_path => {}
                Ok(_) => {
                    report.failure = Some(format!("ticket {} moved after dry-run", ticket_plan.id));
                    break;
                }
                Err(error) => {
                    report.failure = Some(format!(
                        "could not resolve ticket {} path after dry-run: {error}",
                        ticket_plan.id
                    ));
                    break;
                }
            }
            let live_snapshot = match TicketFs::part_layout_snapshot(&current.path) {
                Ok(snapshot) => snapshot,
                Err(error) => {
                    report.failure = Some(format!(
                        "ticket {} could not be re-read before apply: {error}",
                        ticket_plan.id
                    ));
                    break;
                }
            };
            let live_plan = match build_ticket_plan(
                current.clone(),
                ticket_plan.ticket_path.clone(),
                live_snapshot,
            ) {
                Ok(live_plan) => live_plan,
                Err(error) => {
                    report.failure = Some(format!(
                        "ticket {} could not be re-planned before apply: {error}",
                        ticket_plan.id
                    ));
                    break;
                }
            };
            if live_plan != *ticket_plan {
                report.failure = Some(format!(
                    "ticket {} dry-run report is stale or has been modified",
                    ticket_plan.id
                ));
                break;
            }

            let Some(manifest_after) = ticket_plan.manifest_after.as_deref() else {
                report.failure = Some(format!(
                    "ticket {} has no planned manifest output",
                    ticket_plan.id
                ));
                break;
            };
            let expected_files = internal_fingerprints(&ticket_plan.preservation);
            let copies = ticket_plan
                .path_changes
                .iter()
                .map(|change| (change.from.clone(), change.to.clone()))
                .collect::<Vec<_>>();
            let removals = ticket_plan
                .cleanup_candidates
                .iter()
                .map(|candidate| candidate.part.path.clone())
                .collect::<Vec<_>>();
            if let Err(error) = TicketFs::publish_part_layout(
                &current.path,
                &ticket_plan.manifest_sha256,
                &expected_files,
                &ticket_plan.expected_absent_paths,
                &copies,
                &[],
                &removals,
                manifest_after.as_bytes(),
            ) {
                if self.verify_ticket_after(ticket_plan).is_ok() {
                    report.completed_tickets.push(ticket_plan.clone());
                    report
                        .unapplied_ticket_ids
                        .retain(|id| *id != ticket_plan.id);
                } else if self.verify_ticket_before(ticket_plan).is_err() {
                    report.recovery_required_ticket_ids.push(ticket_plan.id);
                }
                report.failure = Some(format!("ticket {}: {error}", ticket_plan.id));
                break;
            }

            report.completed_tickets.push(ticket_plan.clone());
            report
                .unapplied_ticket_ids
                .retain(|id| *id != ticket_plan.id);
        }
        Ok(report)
    }
}
