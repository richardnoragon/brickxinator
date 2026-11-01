# Brickxinator architecture

Current implementation reference (2026-11-01): the approved stack and domain
boundaries below are implemented. Detailed choices subsequently resolved include
bounded hashed attachments, exact integer/rational quantities, Restic with opt-in
systemd scheduling, checked migration/restore coordination and native JSON/assets
exchange. Read ADR-019 through ADR-022 and [ALPHA_STATUS](../ALPHA_STATUS.md) for
current evidence; “implementation selection” text below is historical design.

Status: approved functional architecture; detailed implementation design remains to be completed. Updated 2026-10-31. Sources: DECISIONS.md, especially D-086 and D-094–D-117.

## Runtime and boundaries

Svelte/TypeScript browser UI communicates with a Rust/Axum local API. Rust owns domain validation, inventory transactions, SQLite persistence and managed filesystem attachments. API style is versioned REST/JSON. SQLite uses WAL, per-connection foreign keys and FTS5. All domain entities use UUIDv7. UI calculations are previews; persisted stock/cost calculations must be validated by the backend. No Tauri, PostgreSQL, external runtime asset dependency, account system or LAN exposure in alpha.

The launcher starts the loopback service and opens the default browser. Firefox and Chromium are mandatory. Track app-tab liveness; stop 90 seconds after the last tab disconnects, cancelling shutdown on reconnect. A refresh is tolerated; a browser crash eventually expires liveness. Unrelated browser tabs do not control app lifetime. Independent backup jobs do not keep the web service running.

## Domain boundaries

Proposed modules follow domain responsibility, not a plugin framework: catalog/settings; purchases/receipt; stock/adjustments; sets/checklists; lot identification; attachments; search/dashboard; audit; backup/restore. These are implementation organization choices, not separately deployed services. Do not build future CAD/train/MOC/plugin infrastructure in alpha.

See [DOMAIN_MODEL.md](DOMAIN_MODEL.md) for entities and relationships. Definitions describe items; ownership records actual holdings; inventory state derives availability. Set copies never count as a second copy of detailed parts. Historical acquisition measurements are not current-stock balances. **Inventory derives from inventory movements, never purchase totals.** Replayable balances are projections of immutable typed movement and lifecycle history.

## Transactional integrity

Receipt, verification correction, lot transfer, ownership adjustment, set-state change and shared-location move must be atomic. Receipt is once per whole purchase; repeat requests cannot duplicate stock. Absolute counts are repeat-safe. Corrections preserve prior values, reason, local identity and timestamp, preview effects and reject impossible balances.

Separate ownership changes from damage/assembly and classification transfers. Damage retains ownership. Transfers conserve valid known measurements and invalidate affected unknown residual measures without converting units. Expected-checklist edits never alter stock. A stock-bearing archive changes visibility only.

Use database foreign keys, constraints and explicit transactions for enforceable invariants. Concrete schema, migration framework, concurrent-tab conflict handling and indexes are implementation design work. SQLite does not imply concurrent network-share operation is supported.

## Numeric and monetary semantics

Piece counts are optional whole-number measures; weight is independently optional, entered in g/kg. No automatic count/weight conversion. Derived usable measures must become unknown when required damage/residual measurements are absent. Current totals expose measurement incompleteness and separate mixed-lot weights from identified holdings.

EUR paid row totals are authoritative. Tax Included/Unknown/Not Applicable/Added Separately is optional metadata and never adds tax twice. Shipping targets invoice lines in all modes; set-copy count is not line brick count. Automatic shares use largest remainder in cents with stable row-order ties. Manual incomplete shares are retained without silent editing and have no complete effective cost. Derived EUR/piece and EUR/kg have more precision than payable amounts and selectable display precision. The schema draft proposes integer pieces, EUR cents and milligrams with exact derived ratios; validate these engineering encodings before schema implementation.

## Files and data ownership

Use configurable XDG per-user data root, SQLite and ordinary managed directories. Copy images/PDFs without altering originals. Proposed layout: database.sqlite, attachments/, config/, imports/, exports/; exact filenames are not a format commitment. Backups include every referenced managed attachment plus database and required configuration.

Store attachment UUID/path/hash/size in SQLite, never image/PDF bytes. Stage file writes and coordinate database references so interruption cannot expose half-created attachments. Restores and snapshots must operate on a consistent database/attachment state; copying a live SQLite file alone is insufficient. File validation, size limits, hash strategy and safe viewer implementation require detailed design.

## Backup and recovery

OS-scheduled backup independent of UI: daily incremental at configurable 03:00 local time, retain 30 recovery points plus required dependencies. Separate full human-accessible monthly snapshot on the first, retain 12. Both schedules run on the first. Coalesce missed work to one current snapshot per due schedule.

Multiple local/external/network destinations; failure never blocks inventory use, but persistent per-target warnings and last-success records are required. Never mistake an unmounted destination for its underlying local directory. The snapshot coordinator creates full ZIPs and invokes a mandatory external incremental tool for daily backup. No backup engine is embedded. ZIP creation alone is not daily backup success; daily external retention is 30 points plus dependencies. Monthly ZIPs remain independent, 12 retained. Exact external executable/adapter is implementation selection.

Restore: validate backup/dependencies, create safety backup, quiesce writes, restore database/attachments coherently, validate result, preserve recovery on failure. Review restored machine-specific paths before enabling destinations. Retention must not remove dependencies required by retained points. See SECURITY.md and TESTING.md.

## Remaining implementation selections

Dependency versions, external incremental executable/adapter, detailed SQL, file limits, UI components and measured performance. Ordered immutable checksummed SQL migrations are required. Detailed field encodings and API are drafted in the linked technical documents. Initial collection expectation is under 10,000 parts; 100,000+ is a growth goal to keep in mind, not a demonstrated capacity. Public reporting contacts remain a release-preparation item.

## Frozen pre-code technical artifacts

[ADR set](ADRs/README.md), [schema draft](DATABASE_SCHEMA.md) and [Rust crate/module plan](RUST_STRUCTURE.md). Native JSON exchange and CSV human export are versioned; future CAD/catalog imports stay deferred. Six domains: Definitions, Ownership, Inventory, Storage, Finance, Audit. Finance does not activate deferred internal mixed-lot cost allocation. All technical drafts are documentation only.
