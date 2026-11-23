# Brickxinator release process

Updated 2026-11-23. `0.1.0-alpha.1` is a source review candidate, not a published
release/tag. Schema 18 is independent of the application prerelease identifier.
[ALPHA_STATUS](../ALPHA_STATUS.md) records actual evidence and outstanding gates.

## Source synchronization versus release

Fetch/reconcile before publishing local work. Preserve unrelated changes and
history, commit a concrete reviewed state, and push the intended review branch.
Do not force-push shared history. Update PR scope/evidence when implementation
scope changes. Ordinary synchronization does not merge main, tag, publish binary
artifacts or approve a release. Never include databases, invoices, secrets,
backups, build output or private configuration.

## Candidate sequence

1. Reconcile scope/docs/ADRs and remaining acceptance work; obtain a reviewed
   candidate with identifiable source commits.
2. Run applicable Rust/frontend/browser and recovery checks in TESTING.md.
3. Verify supported Kubuntu desktop, launcher, default browsers, clipboard and
   offline behavior; headless/Docker checks are not desktop approval.
4. Validate upgrade safety and snapshot/native recovery with referenced assets.
   Refresh daily external Restic and independent monthly restore evidence when
   relevant; do not publish with an untested recovery path.
5. Refresh dependency/content/license review and reporting/enforcement contacts.
   Dated advisory evidence must not be presented as a fresh scan.
6. Prepare an accurate changelog with actual source references, artifacts and
   operating limits. Obtain Richard's release approval.
7. Only then tag/publish the approved prerelease and immutable artifacts, following
   the applicable [versioning policy](../Versioning/Versioning%20%20Standard%20%20Policy.md).

## Rollback and operational boundaries

Never edit migrations 001–018. New upgrades require a validated pre-change safety
snapshot and recovery coverage. Restore through the coordinator with an undo
snapshot and interrupted-restore journal; do not copy a live SQLite file.

Source installation/local Docker preview are documented in INSTALL.md. No portable
published image, Kubuntu installer, LAN/cloud/multi-user support or automatic
backup activation is claimed. Private reporting is explicitly deferred until
before release; community enforcement setup also remains open.
