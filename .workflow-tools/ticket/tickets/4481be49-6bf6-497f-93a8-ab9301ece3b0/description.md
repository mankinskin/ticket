# Objective

Make ticket-part writes non-empty and recoverable, make `ready` freeze only existing planning parts, and replace opaque UUID-derived part filenames with deterministic kind-slug paths. Provide a safe migration for existing paths and the narrowly approved empty-part cleanup.

## Requirements

1. Reject whitespace-only content before any mutation on every externally reachable part-content write path, including creation, overwrite, amendment, undo, and the legacy implicit objective/description path.
2. Entering `ready` freezes only planning-kind parts already present in the manifest; it does not create missing parts. Reject creation of new planning-kind parts while `ready`, while preserving non-planning and amendment writes.
3. Route part content, related manifest updates, and path changes through a ticket-local transaction/recovery protocol. Interrupted creates, overwrites, manifest-only part changes, and migrations must recover to a complete old or new state without exposing partial content. Add failure-injection coverage at each publish boundary.
4. Use deterministic normalized kind slugs for new and migrated part paths, with manifest-order ordinals for repeated normalized slugs (for example, `review.md` and `review-2.md`). Preserve stable part IDs and manifest metadata/order. Reject collisions instead of overwriting unrelated files.
5. Provide dry-run, apply, and undo for part-path migration. Rename existing UUID-named parts, but remove only zero-byte, frozen planning parts. Preserve other empty files and all orphan files. Recheck the dry-run inputs under the ticket lock before applying.
6. Keep the ticket API, CLI, and other part-write transports behaviorally consistent; add focused regression and migration tests and update directly related documentation.

## Acceptance Criteria

- Blank and whitespace-only writes fail before changing part bytes or the manifest on every listed write path.
- Freezing `ready` leaves absent planning kinds absent; existing planning parts are frozen. New planning-kind writes are rejected in `ready`; non-planning and amendment writes still work.
- Failure-injection tests demonstrate recoverable creates, overwrites, manifest-only part updates, and migrations, including recovery after reopening the ticket store.
- Dry-run reports every proposed path change, collision, cleanup candidate, and skipped/conflicting entry. Apply refuses stale inputs or conflicts and preserves all stable part IDs and metadata. Undo restores only the migration's own prior state and refuses conflicting later edits.
- Only zero-byte, frozen planning parts are cleanup candidates; other empty parts and every orphan remain unchanged.
- The focused ticket-api and ticket CLI tests pass, formatting is clean, and the exact ticket/spec relationship is recorded.

## Validation

- `cargo fmt --check`
- `cargo test -p ticket-api`
- `cargo test -p ticket-cli`
- Exercise migration dry-run, apply, and undo against isolated fixtures and read back the resulting manifest and files.

## Related Spec

See [Structured Ticket Entities](workflow-tools/memory-kernel/.workflow-tools/spec/specs/24b3d22b-e235-4c4f-b53c-75fb819ea95b/spec.toml).
