# Brickxinator database schema draft

Current implementation (2026-11-01): the authoritative executable schema is
[migrations 001–012](../migrations). Canonical state uses an immutable typed
movement journal plus revisioned purchase/settings/catalog/verification/stock/
shipping commands and derived search. Migrations 009–012 add attachment audit,
eligible-draft deletion tombstones, acquisition reconciliation links and
verification identity references. SQLite stores wide quantities in typed JSON
where needed; do not infer the runtime integer representation from the draft
column plan below. Native exchange uses the explicit current-schema table/field
registry in `crates/brickxinator-infra/src/exchange.rs`, never imported SQL.
See ADR-019 through ADR-022 for implemented recovery/exchange contracts.

The remaining tables below retain the pre-code engineering design for context.

## Historical specification

**Inventory is not calculated from purchases. Inventory is calculated from inventory movements.**

Status: pre-implementation design, 2026-10-31. Approved architecture is recorded in [ADRs](ADRs/README.md); detailed fields/constraints below are an engineering draft, not deployed migrations. Preserve all functional rules in DOMAIN_MODEL.md. The six domains are Definitions, Ownership, Inventory, Storage, Finance and Audit; attachment/backup/search tables are supporting infrastructure.

## Database contract

SQLite with WAL, foreign keys enabled and verified on every connection, FTS5 available. One local user/one active machine; web tabs share the same local service. Application-controlled writes, exclusive cross-process maintenance for migration/snapshot/restore. No direct filesystem-sync or network-share live database.

Primary keys: application-generated UUIDv7 TEXT, canonical lowercase 36-character representation; foreign keys use identical representation. UUID format is validated at boundaries. Draft human labels never replace IDs. Technical ordering/FTS/migration counters may be integers without becoming entity IDs.

Draft numeric representation: owned/expected counts as nullable INTEGER whole pieces; money as nullable INTEGER EUR cents; weight as nullable INTEGER milligrams, presented as g/kg. Decimal input above supported weight precision is rejected/explained rather than silently rounded. Derived unit prices use exact integer ratios/decimal arithmetic, not binary floating-point money; display precision is a setting. NULL numeric means unknown, zero means measured/known zero. Categorical unknown uses a protected real UUID row, not NULL/empty/duplicate Unknown labels.

All mutable aggregate roots have revision INTEGER and timestamps. Domain/audit timestamps are UTC; schedule evaluation uses machine-local time. Archive is timestamp/state, not physical deletion. Index referencing columns and common filters. Foreign-key deletion is RESTRICT for history/stock; narrowly scoped deletion of eligible empty drafts is an explicit command.

## Definitions

| Table | Proposed columns beyond id | Constraints / relationships |
| --- | --- | --- |
| manufacturer | display_name, normalized_name, is_system_unknown, archived_at, revision | Exactly one protected Unknown; rename/archive audited. |
| color | display_name, normalized_name, is_system_unknown, archived_at, revision | Optional future source-code mapping belongs to adapter data, not required online catalog. |
| condition | display_name, is_system_unknown, archived_at, revision | Seed Unknown/New/Used/Mixed; damage is separate. |
| part_definition | manufacturer_id FK, part_number nullable, name/description nullable, archived_at, revision | Preserve raw supplied identifier; unique normalized known manufacturer/number only where identity is actually complete. Unknown/incomplete identities do not auto-merge. |
| set_definition | manufacturer_id FK, set_number nullable, name/year/theme/stated_piece_count nullable, archived_at, revision | Separate numbering namespace from parts; metadata creates no stock. |
| set_definition_line | set_definition_id FK, part_definition_id nullable FK, color_id FK, expected_count nullable, position | Expected template only; copied checklist is a snapshot, not a live alias. |

Unknown brand does not match a branded part simply because its number matches. Missing part/set identifiers are allowed; internal IDs still exist. Changes to templates never silently rewrite existing copied checklists or owned stock.

## Ownership

| Table | Proposed columns beyond id | Constraints / relationships |
| --- | --- | --- |
| purchase | status, vendor nullable, seller_type, seller_country nullable, purchased_at nullable, order_number nullable, currency, shipping_cents nullable, tax_status, informational_tax_cents/rate nullable, notes, archived_at, revision | EUR only. Draft/Ordered/Shipped/Received/Cancelled; Received status is command-controlled. Shipping/tax/prices nonnegative when supplied. |
| invoice_line | purchase_id FK, position, line_kind, description, acquired_copy_count nullable, acquired_piece_count nullable, acquired_weight_mg nullable, paid_total_cents nullable, notes, revision | Stable position for rounding. Independent copy/brick units. Price belongs here once. Corrected current view has immutable revisions. |
| owned_set_copy | invoice_line_id nullable FK, set_definition_id nullable FK, display_label, condition_id FK, location_id nullable FK, archived_at, revision | Assembly/verification current state is a projection of recorded operations. Nullable provenance supports existing collection/drafts; creating a draft must not activate ownership. |
| owned_batch | invoice_line_id nullable FK, part_definition_id nullable FK, manufacturer_id FK, color_id FK, condition_id FK, set_copy_id nullable FK, parent_lot_id nullable FK, location_id nullable FK, batch_kind, archived_at, revision | Identity/provenance metadata only. Current numeric balances are not independently editable here. Incomplete part definition can remain unlinked. |
| mixed_lot | batch_id UNIQUE FK, display_label, notes | Lot is a specialization of one holding, not additional stock. Lifecycle is derived from classification operations. |
| receipt | purchase_id UNIQUE FK, inventory_operation_id UNIQUE FK, received_at | Exactly one whole-purchase receipt. Set-copy receipt does not fabricate part stock. |

Provenance links are established during receipt/verification/transfer. A 3-set invoice line can link to three copies with unknown individual prices. A batch attached to a set uses the set's effective location; do not store independently conflicting set-part locations. Standalone children inherit a location once and then move whole.

## Inventory — authoritative operation journal

| Table | Proposed columns beyond id | Purpose |
| --- | --- | --- |
| inventory_operation | local_sequence UNIQUE INTEGER, operation_kind, committed_at, occurred_at nullable, actor, reason nullable, request_key UNIQUE, expected_revision, invoice_line_id nullable FK, set_copy_id nullable FK, verification_line_id nullable FK, lot_id nullable FK, supersedes_operation_id nullable FK, event_schema_version, payload_json | Atomic command envelope and explicitly typed lifecycle facts; references validated for each kind. Commit order comes from local_sequence, not UUID/time. Every owned-stock creation/change has an operation. |
| inventory_movement | operation_id FK, batch_id FK, movement_type, owned_count_action/value, owned_weight_action/value, damaged_count_action/value, damaged_weight_action/value, notes | Append-only per-batch numeric/validity effects; one operation can have paired source/destination movements. Signed values allowed only for delta; result cannot be negative. |
| verification_session | set_copy_id FK, template source nullable, created_at, revision | Long-lived verification aggregate; finished/completed state is recorded/audited, source checklist retained. |
| verification_line | session_id FK, part_definition_id nullable FK, color_id FK, expected_count nullable, position, revision | Actual count is backed by this line's associated batch movements, not a second editable stock balance. Finish/reopen facts reference operation history. |
| verification_line_batch | verification_line_id FK, batch_id UNIQUE FK | Can represent counted stock of different conditions while preserving one expected line; verify-total command reconciles actual aggregate atomically. |
| adjustment | operation_id UNIQUE FK, adjustment_type | Lost/Broken/Donated/Sold/Correction; mandatory reason for stock correction. No duplicate quantity source in this table. |
| identification_transfer | operation_id UNIQUE FK, source_lot_id FK, child_batch_id FK | Conserved typed extraction links. Each child inherits purchase provenance; no child price is inferred. |

Draft action vocabulary for each measure: UNCHANGED, DELTA, SET_MEASURED, INVALIDATE. UNCHANGED/INVALIDATE require no value; DELTA requires known previous measure and signed integer; SET_MEASURED records an explicit supplied nonnegative observation. Unknown state cannot be transformed by treating NULL as zero. Corrections to an unknown measure use SET_MEASURED, with reason. Never sum absolute SET observations as deltas.

A physically-empty confirmation is a separate typed lot lifecycle fact, not SET_MEASURED(0). It suppresses the residual current holding while preserving historical uncertainty. Reopen reverses only that lifecycle flag; it does not restore the acquisition 5 kg as the residual. New measured facts are explicit.

Assembly events belong to set-copy operations, including unverified copies with no batches. Replay uses current set-state facts plus movement balances to derive availability. Do not record both “-100 available on assembly” and subtract assembly again at query time. Damage uses damaged measures and leaves owned measures unchanged. Replayed availability for a known loose batch is owned minus damaged where both are known; either assembled state yields known zero loose-use availability for its known contents; unknown coverage is still disclosed.

### Movement examples

| Command | Immutable facts | Ownership effect |
| --- | --- | --- |
| Receive 100 loose tracks | RECEIVED, explicit measured owned count 100 | 100 owned; not derived later from invoice quantity |
| Receive unknown 5 kg lot | RECEIVED, count unknown, owned weight SET_MEASURED(5000000 mg) | Known weight only |
| Verify total 7, then 7 again | First measured observation/validated change; repeat is no-op for stock | 7, never 14 |
| Correct verified 7 → 9 | CORRECTION delta +2 plus prior/new audit and checklist reopen | 9 current |
| Mark 2 broken out of 100 | BROKEN damaged-count delta +2; owned unchanged | 100 owned, 98 usable if loose |
| Donate 5 known usable tracks | DONATED owned-count delta -5; damaged portion unchanged | 95 owned, 93 usable in preceding example |
| Identify 20 pieces from known 100 | One operation: source delta -20, child measured 20 | Total remains 100 |
| Identify 100 unweighed tracks from 5 kg unknown-count lot | Source weight INVALIDATE, source count remains unknown; child count SET_MEASURED(100), child weight unknown | Historical 5 kg preserved; no weight/count conversion |
| Mark Assembled/Partially Assembled | Set state event, no owned deduction | Contents unavailable, damage/ownership retained |
| Confirm unknown remainder physically empty | Lifecycle event with actor/time | Residue excluded without fabricated measured zero |

## Storage

location: id, parent_id nullable FK, display_name, normalized_sibling_name, archived_at, revision. Check self-parenting; command checks ancestor cycles. Warn on duplicate/similar sibling names; never silently rename. Set moves are whole-container operations. Archived locations remain historically resolvable; moving/reassigning referenced locations requires explicit validation.

## Finance

shipping_allocation: id, purchase_id FK, method, revision, completeness. shipping_allocation_share: id, allocation_id FK, invoice_line_id FK, manual_share_cents nullable, derived_share_cents nullable; unique line within an allocation. Enforce that each target line belongs to the same purchase. Manual is never overwritten by automatic recalculation. Previous manual configurations/revisions retained in audit when method changes.

Purchase-level shipping is recorded once. Complete allocated shares sum to that amount. Price, piece and weight inputs come from valid historical invoice-line measures; current lot sorting/stock loss must not rewrite shipping history. Use largest remainder and stable row order. Known zero basis can get zero; missing/all-zero total basis is incomplete.

cost_allocation is a reserved future domain concept, **not an alpha table/engine**: allocation of mixed-lot price among identified child batches remains deferred. Derived unit costs/price history use only known attributed costs. Shipping allocation does not fabricate a per-copy or child purchase price.

## Audit and infrastructure

- audit_log: UUID, typed target references, operation_id nullable FK, changed_at, local_actor, reason nullable, before_json, after_json, event_schema_version. Stock-change reason mandatory; old rows append-only. Typed association tables/FKs must preserve referential integrity; do not rely solely on unchecked source_type/source_id strings.
- attachment: UUID, managed_relative_path UNIQUE, sha256, size_bytes, media_type, original_name, created_at, archived_at. Use explicit purchase/invoice-line/set-definition/set-copy attachment association tables with real FKs. Never use UUID as trust proof or accept arbitrary absolute file paths.
- backup_destination and backup_run: configuration reference, reviewed activation, schedule kind, immutable snapshot ID/manifest hash, published paths, hook status, per-target last success/failure. Do not include credentials in portable exports by default.
- schema_migration: ordered migration number/name/checksum/applied_at; infrastructure version numbers are not UUID domain entities. Immutable migration_001.sql etc. Runner validates all historical checksums before upgrade.
- inventory_projection: per-batch replay result, known/unknown measures, effective availability, lifecycle state and last operation sequence. Rebuildable; never a second editable source. A replay/check command compares persisted projection with journal-derived result.
- search_document plus FTS5 virtual table: technical integer rowid maps to UUID/entity kind, text columns for names/descriptions/notes/vendor/manufacturer. Rebuildable from current records, indexes archive state separately. Exact part/set numbers and location/availability use structured filters.

## Transaction protocol

1. Begin a write transaction; acquire the expected aggregate revision and idempotency key.
2. Validate current journal/projection, ownership/provenance, known units and invariants. Reject negative results/over-extraction/stale edits.
3. Append operation, movements and audit, update aggregate metadata/checklist state and derived projections together.
4. Commit once. Retry of an identical request returns prior result; conflicting reuse is rejected.

Snapshot/restore/migration use a separate cross-process maintenance lock. No lock expiration may allow a writer through during a still-active protected operation. Crash recovery and lock ownership must be tested. File publication is staged outside raw DDL; retain immutable referenced bytes during snapshots.

## Keys, indexes and growth

Index FK columns; batches by part/color/condition and provenance; set copies by definition/location; movements by (batch_id, operation local sequence); invoice lines by purchase/stable position; audit by target/time; locations by parent. If operation sequence is joined rather than stored on movement, choose indexes/query plan accordingly. Exact search number indexes remain distinct from FTS. Use pagination, avoid full-ledger replay per page and provide projection rebuild/reconciliation.

Add synthetic benchmark fixtures at 50,000 and 100,000 holdings/records with multiple movement histories. Record memory, startup, search/filter, correction/transfer, backup size/time and replay/restore behavior before claiming capacity. Piece count is not record count; performance targets remain to be measured rather than invented.

## Exchange and snapshot metadata draft

JSON envelope: format_name, format_version, export_id UUIDv7, created_at UTC, app_version, schema_version, scope, entities/relationships and assets manifest. Numeric unknowns explicit null; categorical Unknown uses stable seeded IDs. Weights carry declared units, money currency and integer cents. Historical events include schema version/order; import assigns validated local replay order without relying on UUID order. Never duplicate receipt or conflicting IDs. Validate references/invariants in staging; explicit preview before commit. External-source imports remain deferred.

CSV export uses explicit scoped tables, UTF-8, declared units and identifiers; blank numeric cells mean unknown, textual spreadsheet-formula input must be escaped safely. CSV is for humans, not a complete restore representation. Exact headers/version fixtures are defined with implementation.

Snapshot ZIP: database.sqlite, attachments/, images/ if used, config/ and metadata.json. Metadata lists snapshot UUID, daily/monthly/manual/safety kind, timestamp/timezone, app/schema/snapshot-format versions, maximum committed operation sequence, relative included paths, sizes and SHA-256 hashes. No recursive inclusion of backups/ or staging/. imports/exports only if retained as managed referenced assets or explicitly configured. Metadata excludes passwords/secrets and local paths requiring review where portable. Verify ZIP entries, size limits and hash/reference coverage before restore; prevent path traversal/symlink extraction and unsupported schema restore.

Full daily ZIP snapshots feed a mandatory external incremental backup integration (D-128, ADR-005). ZIP publication alone is not daily-backup success. Keep 30 external daily recovery points plus dependencies; bounded local staging is separate. Monthly 12 full ZIPs remain independently restorable. Format version must change when compatibility rules change; JSON/ZIP filenames are not the sole format identifier.
