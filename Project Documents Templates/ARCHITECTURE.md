# Brickxinator architecture

Current implementation, 2026-11-23. [ADR-023](ADRs/ADR-023-expanded-alpha-ledgers-and-enrichment.md)
supersedes earlier scope/storage/allocation assumptions where noted.

## Runtime

A Rust/Axum loopback API serves local Svelte/TypeScript assets. SQLite uses WAL,
foreign keys, FULL synchronous mode and FTS5. Native launcher and local Docker
preview are documented in [INSTALL](../INSTALL.md). One local user/active machine;
no account, telemetry, required cloud dependency or LAN service.

Host/Origin validation, process mutation tokens and service-user audit attribution
protect local operations. Large integer reads use decimal strings where needed.
UI previews do not authorize stale writes: backend commands revalidate revisions,
source history and dependencies under transactions.

## Responsibilities

- **Core:** typed identities/measures, inventory and copy lifecycle, verification,
  purchase/fulfilment/return/financial rules, projects/claims, exact costing and
  bounded LDraw/delimited/XML interchange transformations.
- **Infra:** SQLite command/audit persistence, source-prefix replay, provenance,
  catalog enrichment, managed files, derived search/rollups, exchange, Restic,
  coordinated snapshots/upgrades/restore and safety recovery.
- **App:** CLI, local API/security/liveness and asset serving. Frontend components
  implement purchase, sets, stock/lifecycle, project, cost and enrichment workflows.

## Authoritative histories and projections

Inventory operations alone introduce/change owned quantities. Receipt intent,
expected checklists, reservations, estimates and catalog imports create no stock.
Holdings are ownership; copy/project assignments reserve quantities independently
of assembly and location. Source quantities may remain unknown after ambiguous
movements; no implicit FIFO is used to make a return succeed.

Purchase/settings/catalog/profile, project, fulfilment, child-cost, integration
and enrichment histories retain source inputs and results. Atomic transactions
check global request keys, expected revisions, stock/profile/settings prefixes
and canonical effects. Repeated identical commands return their original result;
conflicting payloads do not overwrite it. Corrections append compensating facts.

Search and cost rollups are views. A shared source/component budget covers direct
copy and holding costs; rollups cannot add the same basis again. Actual allocated
quantities govern project cost shares. Unallocated/residual/disposed/unknown costs
are separate. Financial settlement and market estimates do not create ownership
or silently reprice reviewed acquisition costs.

## Files and integration

Managed image/PDF files retain original bytes, generated safe paths, hashes and
associations. Reviewed catalog/BOM/color/theme imports cache bounded local text
and preserve unselected overrides/UUIDs. Theme links reference local IDs; external
IDs require explicit mappings. No remote image fetch or live synchronization.

## Recovery

Immutable checksummed migrations 001–018 are executable schema. Cross-process
maintenance locks coordinate consistent SQLite/asset snapshots and replacement.
Upgrades validate safety ZIPs before changes; restore validates manifests/hashes,
creates undo safety data and uses a durable interrupted-restore journal. Native
exchange uses a fixed current-schema table registry and no imported SQL.

Restic is an external adapter, not an embedded engine. Daily protection requires
validated ZIP plus successful backup/check/retention; monthly ZIPs are independently
restorable. Configuration is reviewed before activation and passwords are not
embedded in exports. See [security](SECURITY.md), [schema](DATABASE_SCHEMA.md) and
[current acceptance limits](../ALPHA_STATUS.md).
