# Architecture decision records

Pre-code technical baseline, 2026-10-31. The user-selected architecture is accepted. Detailed engineering refinements are explicitly marked draft; daily external incremental backup is confirmed mandatory alongside full ZIP snapshot export (D-128). No source code, SQL migration or runtime configuration has been created by this set.

| ADR | Decision |
| --- | --- |
| [ADR-001](ADR-001-local-offline-stack.md) | Local offline stack and ownership |
| [ADR-002](ADR-002-sqlite-operating-mode.md) | SQLite operating mode and safe snapshots |
| [ADR-003](ADR-003-movement-ledger.md) | Movement-based inventory source of truth |
| [ADR-004](ADR-004-filesystem-attachments.md) | Filesystem attachments with database metadata |
| [ADR-005](ADR-005-snapshot-coordinator.md) | Snapshot coordinator and external backup hooks |
| [ADR-006](ADR-006-rest-json-api.md) | Versioned REST and JSON API |
| [ADR-007](ADR-007-sqlite-fts5.md) | SQLite FTS5 and structured indexes |
| [ADR-008](ADR-008-exchange-formats.md) | Versioned JSON, CSV and snapshot manifests |
| [ADR-009](ADR-009-immutable-migrations.md) | Immutable ordered SQL migrations |
| [ADR-010](ADR-010-uuidv7.md) | UUIDv7 entity identifiers |
| [ADR-011](ADR-011-initial-journal-persistence.md) | Initial standalone journal persistence (implemented engineering decision, 2026-11-01) |
| [ADR-012](ADR-012-purchase-receipt.md) | Purchase drafts and atomic whole-purchase receipt (implemented, 2026-11-01) |
| [ADR-013](ADR-013-managed-settings.md) | Managed vocabularies and location trees (implemented, 2026-11-01) |
| [ADR-014](ADR-014-catalog-inventory-links.md) | Catalog definitions and descriptive inventory links (implemented, 2026-11-01) |
| [ADR-015](ADR-015-set-verification.md) | Journal-backed set verification and assembly (implemented, 2026-11-01) |
| [ADR-016](ADR-016-stock-lifecycle.md) | Typed adjustments and mixed-lot lifecycle (implemented, 2026-11-01) |
| [ADR-017](ADR-017-search-dashboard.md) | Rebuildable inventory search and dashboard read models (implemented, 2026-11-01) |
| [ADR-018](ADR-018-shipping-costs.md) | Exact invoice-line shipping allocation and cost views (implemented, 2026-11-01) |

Read alongside [database schema draft](../DATABASE_SCHEMA.md), [Rust structure](../RUST_STRUCTURE.md) and [decision history](../DECISIONS.md). Changes supersede ADRs explicitly; do not erase their original context.


Alpha application/recovery implementation:

- [ADR-019: Local application, managed files and recovery](ADR-019-local-application-and-recovery.md)
- [ADR-020: Restic and headless backup schedules](ADR-020-restic-and-headless-backups.md)
- [ADR-021: Corrections, checklists and record lifecycle](ADR-021-corrections-checklists-and-record-lifecycle.md)
- [ADR-022: Native exchange and CSV](ADR-022-native-exchange-and-csv.md)

Earlier ADRs retain their implementation-time statements; these records and the
root ALPHA_STATUS.md describe the subsequent completed work and remaining gates.
