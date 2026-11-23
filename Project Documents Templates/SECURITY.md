# Security and data protection

Current candidate, 2026-11-23, schema 18: loopback Host/Origin checks, mutation
tokens, service-user audit attribution, bounded hashed files and exclusive
maintenance/recovery remain implemented. Reviewed import/enrichment validates
source formats/hashes, local identities, hierarchy and settings prefixes; it does
not fetch remote images. Stock, return, financial and cost commands revalidate
expected revisions/request keys/dependencies atomically. Restic credentials are
private files referenced by path, not embedded in snapshots/exports.

Actual checks/desktop limits are in [ALPHA_STATUS](../ALPHA_STATUS.md). Dependency
advisory results retain their dated review in [THIRD_PARTY](../THIRD_PARTY.md),
not an implied fresh zero-vulnerability assertion. No release exists. Private
reporting remains explicitly deferred and must be finalized before release;
never publish private collection data or sensitive reports in public issues.

## Historical specification

Status: requirements, not claims of implemented security. No released version exists. Private vulnerability-reporting destination and supported-version policy must be set before public release; do not use an invented email or publish sensitive collection data in public issues.

## Local service

Loopback-only, no login/LAN access. Firefox/Chromium; explicit-click external links only. Service stops 90 seconds after last app-tab disconnect, reconnect cancels pending stop. Core assets/workflows operate offline. No telemetry, uploads or cloud synchronization are approved for alpha.

Implementation must validate origins, protect mutation requests against cross-origin abuse and bind only intended local addresses. File paths and attachments must not escape managed roots. Browser PDF/image handling, size limits, MIME validation and safe rendering are implementation design work. Do not assume loopback access removes browser-origin risks.

## Sensitive data and files

Purchases can contain seller data, addresses in invoices, prices and source URLs. Use synthetic test fixtures and avoid sensitive logs. Managed images/PDFs preserve original source files. Archive retains attachments/history and inventory totals. Empty never-received drafts may be permanently deleted; stock/history-linked records are recoverably archived.

Default data root follows XDG_DATA_HOME or ~/.local/share/brickxinator, configurable. Ordinary files must remain accessible to standard Linux backup tools. No concurrent network-share database deployment is supported by the alpha plan. Retain third-party content rights and attribution separately from GPL project code.

## Backup requirements

Daily incremental at configurable 03:00 local time, 30 retained recovery points plus dependency baselines. Separate full, self-contained, human-accessible first-of-month snapshots, 12 retained. Monthly never replaces daily. Independent OS scheduling, including while UI service is stopped. Missed runs coalesce to a current catch-up per due schedule.

Multiple local/external/network destinations. Failures never block use; persistent per-target warning and last-success status. Detect unavailable/unmounted targets rather than writing into the wrong location. Daily restore may depend on established backup software; monthly restore must not. Architecture is full ZIP snapshot export plus a mandatory external incremental integration, never an embedded backup engine. ZIP-only success is not daily-backup success. Exact executable/adapter, encryption/credentials and retry cadence remain implementation selections; do not claim encrypted backups before implemented.

Snapshots must capture consistent SQLite data and referenced files/configuration. A live database file copy alone is not a proven backup. Retention must preserve dependencies required by retained daily points. Backup run integrity and restoration must be tested.

## Restore and migrations

Validate source/dependencies/checksums; create safety backup; stop writes; restore database/attachments consistently; validate before resuming. Preserve failure recovery and ability to undo a mistaken restore. Require review before restoring machine-specific paths/destination activation. Migrations use ordered schema versions and tested recovery paths; irreversible changes require validated pre-migration backup.

## Release gates

Restore and migration checks, both-browser local service checks and integrity tests are mandatory. Reporting contacts, dependency/content review and deployment instructions must be resolved before public release. See RELEASE_PROCESS.md and TESTING.md. No scheduler, backup task or external publication has been created during discovery.

Pre-code technical requirements: immutable movement journal, UUIDv7 entity identity, SQLite WAL/per-connection foreign keys/FTS5, immutable checksummed SQL migrations, filesystem asset hashes and versioned JSON/ZIP manifests. See ADRs/README.md and DATABASE_SCHEMA.md. Unknown/invalid and provenance rules still apply to imports.
