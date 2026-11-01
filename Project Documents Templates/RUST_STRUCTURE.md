# Rust crate/module structure and API plan

Current implementation (2026-11-01): the three-crate workspace now exists in
[crates](../crates), with locked dependencies in Cargo.lock. Core owns typed
commands/invariants; infra owns SQLite, files, search, snapshots, recovery, Restic
and native exchange; app owns the loopback Axum API and CLI. Svelte/TypeScript
lives in [frontend](../frontend). CLI installation/check commands are in
[INSTALL](../INSTALL.md); validation is in [ALPHA_STATUS](../ALPHA_STATUS.md).
The layout below is the historical pre-code plan; actual source paths are
authoritative. API routes are versioned under `/api/v1`.

## Historical specification

Status: proposed implementation layout, 2026-10-31. No Cargo workspace, source code, API server or migrations have been created. Approved technical choices: [ADRs](ADRs/README.md). Detailed Rust dependencies/versions will be pinned before use.

## Workspace shape

```text
brickxinator/
  Cargo.toml                      # future workspace manifest
  crates/
    brickxinator-core/
      src/
        definitions/
        ownership/
        inventory/
          movements.rs
          replay.rs
          verification.rs
          identification.rs
          adjustments.rs
          availability.rs
        storage/
        finance/
        audit/
        ids.rs
        measurements.rs
        money.rs
        commands.rs
        ports.rs
        errors.rs
    brickxinator-infra/
      src/
        sqlite/
          connections.rs
          migrations.rs
          repositories/
          projections.rs
          search.rs
        attachments/
        snapshots/
          coordinator.rs
          manifest.rs
          zip.rs
          restore.rs
          destinations.rs
          external_hook.rs
        exchange/
          json.rs
          csv.rs
        platform/
          paths.rs
          maintenance_lock.rs
          local_actor.rs
    brickxinator-app/
      src/
        main.rs
        cli.rs
        bootstrap.rs
        http/
          routes/
          dto.rs
          errors.rs
          security.rs
        lifecycle.rs
  migrations/
    migration_001.sql             # immutable once created/committed
  frontend/
    src/
      api/
      features/
      components/
      routes/
  scripts/                        # launcher, future systemd setup assets
  tests/fixtures/                 # synthetic collection/restore fixtures
```

Three crates keep pure business logic, I/O and executable composition separate without one crate per table. The six business domains are modules. Naming/file splits can change as implementation reveals cohesion; preserve boundaries rather than empty scaffolding.

## Dependencies and responsibilities

core depends on no Axum, SQLite, OS scheduler or external backup engine. It validates typed commands, folds movement histories, derives availability and calculates shipping deterministically. It can depend on narrowly selected UUID/serialization/decimal utilities where justified. Repository/transaction/file interfaces are ports, not an abstract plugin framework.

infra implements SQLite transactions, immutable migrations, search/projections, file storage, snapshot/restore, native JSON/CSV and platform paths/locks. No embedded Restic/Borg/Kopia repository implementation. External hooks are executable plus argv, inactive until explicitly configured. Daily incremental integration is mandatory: missing configuration must be visible, and ZIP-only success cannot satisfy the daily backup policy. Timeout/exit status plus per-target recovery evidence are separate from snapshot export success.

app composes dependencies and exposes CLI modes such as serve, snapshot, restore, export, migrate and doctor/reconcile. These are proposed command names, not commands users can run yet. Headless snapshot works while web service is stopped and uses the same coordination protocol if running. Initial web assets are served locally from the packaged frontend build.

Frontend owns presentation and preview validation, never authoritative stock/cost persistence. No direct database/filesystem writes from the browser. Multi-tab edits use record revision/conflict handling despite single-user scope; no user/role/tenant framework.

## REST/JSON draft

Version prefix /api/v1. Examples below are proposed contracts, not implemented endpoints.

| Method/path | Meaning |
| --- | --- |
| GET /api/v1/parts | Definition search; filters/pagination |
| GET /api/v1/inventory | Holding/availability view, known/unknown coverage |
| GET /api/v1/dashboard | Aggregates, not a raw purchase sum |
| POST /api/v1/purchases | Create blank or populated draft |
| PATCH /api/v1/purchases/{id} | Audited descriptive/source correction with expected revision |
| POST /api/v1/purchases/{id}/receive | Whole-purchase receive-once command |
| POST /api/v1/verification-lines/{id}/count | Set absolute total with revision/reason; idempotent request key |
| POST /api/v1/verification-lines/{id}/finish | Explicit line completion |
| POST /api/v1/set-copies/{id}/complete-verification | Explicit overall completion |
| POST /api/v1/set-copies/{id}/assembly-state | Record Loose/Assembled/Partially Assembled |
| POST /api/v1/lots/{id}/identify | Atomic source/child transfer |
| POST /api/v1/lots/{id}/complete | Known-empty or explicit physical-empty confirmation |
| POST /api/v1/lots/{id}/reopen | Audited classification reopen |
| POST /api/v1/batches/{id}/adjustments | Lost/Broken/Donated/Sold/Correction command |
| POST /api/v1/records/{kind}/{id}/archive | Allowlisted record kinds; no arbitrary table access |
| DELETE /api/v1/purchases/{id} | Eligible empty never-received draft only |
| POST /api/v1/app-tabs/heartbeat | Same-origin tab liveness; no stock write |

Stock commands return committed revision/operation ID and authoritative resulting view. Repeat request keys return the same result; stale versions/conflicting request-key payloads produce conflict. Missing draft data is a completeness result, malformed input is validation failure, and unavailable backup target is a separately surfaced operational warning. Do not expose SQL errors/private paths verbatim.

## Implementation order

1. Freeze field encodings/connection configuration and write journal/measurement/availability contract tests, using synthetic facts. No stock feature may ship before ledger invariants exist.
2. Add core IDs, money/measure types and replay, then the minimum definitions/batch tables necessary for FK-valid movements. Movement ledger is the first stock feature, not a table created without its references.
3. Add SQLite transaction/idempotency/audit implementation; prove receive-once, absolute verification, partial damage, lot transfer/invalidation and rebuild equivalence.
4. Implement business UI/API incrementally against the core; FTS/query projections remain rebuildable.
5. Add filesystem/manifest consistency, independent snapshot CLI and restore, then scheduling/destinations plus at least one tested external incremental integration. Exercise interruption and recovery before real collection data.
6. Add native export/import compatibility fixtures, source-build/launcher/setup documentation and mandatory release gates.

## Test boundaries

Pure core: deterministic shipping, unknown/known measurements, stock conservation, replay and state transitions. Infra: constraints/transactions/retries, migration checksums/upgrades, FTS rebuild, attachment staging and snapshot/restore. App/browser: REST conflicts/idempotence/origin checks, launcher/90-second lifecycle and core workflows in Firefox/Chromium. Follow TESTING.md; exact tooling commands follow dependency selection.
