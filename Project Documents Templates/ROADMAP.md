# Brickxinator roadmap

No deadline. Functionality and integrity determine readiness.

## Current alpha candidate — 2026-11-01

The Rust foundation, local browser application, managed attachments, reviewed
corrections, archive/purge workflows, native exchange, migration/recovery and
Restic integration are implemented. Current schema is 12. Validation includes
140 standard Rust tests, three opt-in process/Restic drills and eight Chromium/
Firefox scenarios. All 30 retained daily and 12 monthly recovery points were
restored using disposable data. See [the completion audit](../ALPHA_STATUS.md)
for exact evidence, limitations and remaining release gates.

Richard's Kubuntu desktop acceptance/release approval and reporting destinations
remain open. Whole-set disposal is explicitly deferred; archiving retains stock.
The dated increments below describe earlier implementation states, not current
missing features.

## Implementation progress — 2026-11-01

Shipping/cost increment: implemented all six shipping modes, exact cent allocation
with invoice-order remainder ties, preserved manual drafts, acquisition-based
exact unit rates and source-revision-validated configuration history. Allocation
changes preserve paid prices and stock. 103 tests, rustfmt and clippy pass; the
demo retains manual values on reopening and shows 34/33/33 cents, EUR 12/kg and
unchanged purchase totals. Schema 8 is for new databases; schema-7 search remains
usable without silent upgrades. See [ADR-018](ADRs/ADR-018-shipping-costs.md).

Search/dashboard increment: added current-stock FTS5 search, exact identifiers,
managed-attribute/location/subtree/availability filters, stable SQL pagination and
all-match coverage-aware totals. Dashboard separates actual parts, physical
copies, unknown-copy lines and original unidentified residue, with pending/recent
purchases. Transactional invalidation and explicit rebuild preserve source data.
87 tests, rustfmt and clippy pass; the demo returns 150 pieces across two matches
despite a one-row page and retains totals after reopening/rebuilding. Schema 7 is
for new databases; schema-6 workflows remain usable without silent upgrade.
See [ADR-017](ADRs/ADR-017-search-dashboard.md).

Stock/lifecycle increment: added named Lost/Donated/Sold/Broken/Correction stock
commands, explicit damaged removal subsets, physical-empty classification
completion, audited reopening and current-inventory exclusion of closed residue.
History preserves original measurements, child provenance and closure dates.
76 tests, rustfmt and clippy pass; the stock demo reopens without fabricated
residual measures and produces 95 owned / 2 damaged / 93 available after damage
and donation. Schema 6 is for new databases; schema-5 verification stays usable
without silent upgrades. See [ADR-016](ADRs/ADR-016-stock-lifecycle.md).

Verification increment: implemented expected checklists/copying, absolute counts,
explicit line/overall completion, audited reopening and whole-set assembly actions.
Actual contents inherit copy location/condition, reject independent relocation and
preserve damage through assembly transitions. Recounts after disposals require
reconciliation. 65 tests, rustfmt and clippy pass; the verification demo survives
reopen/retry and shows 7 → 9 correction without duplication. Schema 5 is for new
databases; schema-4 catalogs remain usable without silent upgrades. See
[ADR-015](ADRs/ADR-015-set-verification.md).

Catalog/storage increment: added part/set definitions without ownership creation,
known-identity uniqueness, independent incomplete definitions, audited catalog
edits/archive, and inventory links to definitions/colors/conditions/locations.
Whole-target moves preserve stock. Classification inherits location atomically,
with independent child relocation afterward. 53 tests, rustfmt and clippy pass;
the catalog demo reopens with 80/20 pieces and retained links/history. Schema 4
is for new databases; schema 3 remains usable without silent upgrade. See
[ADR-014](ADRs/ADR-014-catalog-inventory-links.md).

Earlier increments below record the state at their implementation time:

Settings increment: implemented protected Unknown manufacturer/color/condition
records, initial conditions, audited rename/archive, nested locations, cycle
checks and explicit duplicate/typo warnings. Concurrent location moves cannot
create a cycle; archived ancestors block descendant selection without deleting
history. 42 tests, rustfmt and clippy pass; the settings demo reopens successfully.
Schema 3 is for new databases; older schemas remain usable without silent upgrades.
Catalog definitions and inventory/settings/location links remain pending.
See [ADR-013](ADRs/ADR-013-managed-settings.md).

Purchase increment: implemented blank/populated drafts, audited pre-receipt
edits, receipt-only ownership, independent known set copies, explicit unknown
copy count receipt, source provenance and atomic receive-once transactions.
31 behavioral tests pass with rustfmt/clippy; the purchase demo runs and survives
reopen/retry without duplicate stock. Migration 002 applies only to new databases;
schema-1 inventory remains usable without silent upgrades. Received corrections,
settings/catalog, storage, attachments, set verification, API/UI and all recovery
release gates remain pending. See [ADR-012](ADRs/ADR-012-purchase-receipt.md).

Previous increment: added the collection ledger, UUIDv7 validation, atomic source/child
classification, and a SQLite adapter with versioned immutable operation history,
global sequence, UTC commit timestamps, transaction retries/revision validation,
new-database migration and checksum validation. Twenty behavioral tests,
rustfmt and clippy pass; the synthetic create/reopen/replay demo runs on bundled
SQLite 3.54.1. See [ADR-011](ADRs/ADR-011-initial-journal-persistence.md) for precise
limits. Purchase receipt, safe schema upgrades, projections, backup/restore and
the user-facing API/UI remain pending.

Initial step:

Started the Cargo workspace and dependency-free `brickxinator-core`. Implemented
known/unknown measurement balances, damage/assembly availability, and an in-memory
per-holding append-only journal with replay, revision checks and idempotent retry
results. Ten behavioral tests, rustfmt and clippy pass on Rust 1.98.1. This does
not yet implement durable transactions, global operation sequencing, receipt,
UUIDv7 identities, multi-holding transfers, SQLite, an API or a user interface.
See the root README for current development commands. Historical discovery
documents describe the pre-implementation baseline; no alpha release gate is
claimed complete by this initial kernel.

## Completed: discovery foundation

Fifteen question/answer rounds plus a pre-code technical freeze recorded in DISCOVERY.md and DECISIONS.md. Alpha scope, core workflows, domain relationships, integrity rules, stack, platform, backup policy, license and release gates established.

## Alpha implementation sequence — proposed organization

1. Foundation: implement the ADR/schema/crate drafts; movement journal and replay invariants first, UUIDv7, SQLite WAL/FKs/FTS5, immutable migrations, Rust/Axum REST and Svelte/TypeScript, launcher/liveness and offline assets.
2. Catalog/settings and purchases: controlled vocabularies/Unknown, nested locations, reusable definitions, drafts, invoice lines, independent set copies, attachments and receipt.
3. Inventory integrity: audit/revisions, stock adjustments, damage/assembly independence, verification/checklist completion, mixed-lot identification and reopen.
4. Daily use: dashboard, global search/filters, price history, unit costs and six shipping modes; incomplete-data presentation throughout.
5. Recovery and release: ZIP snapshot coordinator, mandatory external daily incremental integration, OS scheduling, multi-destination backups, retention, restore/undo safeguards, browser validation, migration drills, source-build instructions and release reporting setup.

Implement behavioral checks with each integrity-sensitive change rather than waiting for the final milestone. Trial data must be disposable before using the real collection.

## Alpha exit

All agreed workflows and mandatory tests pass, including receive-once, find stock, gradual lot identification, set verification/assembly, no negative or duplicate stock, shipping arithmetic, audits, Firefox/Chromium, restore and migration. Target first prerelease identifier: 0.1.0-alpha.1; not released yet.

## Deferred, without a release date

Reservations; per-part assembly; set bag/subassembly decomposition; global missing-parts planning/dashboard; partial receipts/shipments, backorders and receipt reversal; discounts/coupons/refunds/returns/warranty flows; child-lot cost allocation; volume shipping allocation; external market history; Obsidian import; installable Kubuntu package; CAD, train planning, MOC authoring and online integrations. User suggested v1.1 for purchase aftercare flows, without a scheduled commitment.

Cloud/always-on NAS operation, other platforms and multi-user/multi-collection expansion remain future directions. Do not infer live multi-PC SQLite access support.
