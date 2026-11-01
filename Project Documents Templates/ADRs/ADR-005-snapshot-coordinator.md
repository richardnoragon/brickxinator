# ADR-005: Snapshot coordinator and external backup hooks

Date: 2026-10-31. Status: Accepted architecture and clarified mandatory external incremental policy. Source decisions: D-074, D-079–D-088, D-122, D-128.

## Context

The user requests a durable pre-code foundation for a long-lived offline collection, ordinary-file portability and growth beyond the initial collection size. This ADR refines the existing alpha charter; deferred business features stay deferred.

## Decision

Brickxinator owns consistent full ZIP snapshot export and validation, with an external backup hook required for daily incremental protection. Do not embed Restic/Borg/Kopia implementations or repository formats. Monthly ZIP is standalone.

## Design and constraints

Coordinator quiesces domain/file writes using a cross-process maintenance lock, copies SQLite through backup API, captures referenced immutable attachments/config and writes metadata.json. Release lock after consistent staging; compress and verify before atomic publication. OS jobs call the same headless coordinator with UI stopped or running. External tool executes after publication via explicit argv configuration, never interpolated shell text. Snapshot success and hook/destination success are tracked separately.

## Consequences

User explicitly confirms daily incremental external backup is mandatory. First create/validate a full daily application ZIP, then invoke the configured incremental-capable external tool. ZIP export success alone is not daily backup success. Do not claim ZIP is incremental. Retain separate 30 daily/12 monthly recovery points, 03:00 local daily schedule, first-of-month separate full monthly, per-target warnings and catch-up. No scheduler is installed during documentation.

## Verification

Implement relevant contract tests from TESTING.md before the first release. No runtime behavior is claimed by this ADR. User-selected architecture is accepted; details marked draft are engineering proposals to validate, not retrospectively attributed user answers.

## Daily external policy after clarification

At least one configured incremental external backup integration must be implemented and validated before alpha release. Keep tool-specific logic at the external-process boundary, never embed its repository engine. Tool executable/arguments/credentials and restore commands are deployment configuration; generic arbitrary-hook success is insufficient to prove incremental recovery. Report missing configuration as backup-not-configured and failed hook as daily-backup-failed even if ZIP export succeeded. Preserve staged snapshot for bounded retries, protect queued files from pruning and report storage exhaustion without silently deleting the only unprotected snapshot.

Retain 30 daily external recovery points plus tool-required dependencies and 12 independently restorable monthly ZIPs. A local staging cache has bounded separate cleanup, not permanent copies of every daily ZIP. Destinations have independent completion state; success to local storage cannot mask NAS failure. Offline unavailable destinations retry without blocking inventory use. On the first, execute and record both daily external backup and monthly full snapshot jobs.

A new full compressed ZIP can reduce external deduplication efficiency; do not promise bandwidth/storage savings merely from selecting an incremental tool. Use stable archive entry order/metadata where possible and benchmark changing large attachment collections. Snapshot and daily external restore drills are both required.
