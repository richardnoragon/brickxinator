# ADR-023: Expanded alpha ledgers, cost rollups and reviewed enrichment

Status: implemented source candidate, 2026-11-23; schema 18. No release approved.
Decision sources: D-136–D-195 and the accepted issue specifications.

## Context and supersession

Alpha scope expanded after the original discovery/early implementation ADRs.
This record supersedes their whole-purchase-only receipt, assembly-driven stock
release, single linked-part location and deferred lifecycle/project/cost/import
assumptions. Original dated ADRs remain historical evidence, not current feature
status. ADR-019–022 recovery/security/native-exchange safeguards remain applicable.

## Decision

Physical holdings are ownership. Copy/project allocations reserve quantities
independently of assembly/location. UUIDs and histories survive lifecycle, stock,
receipt/return and financial corrections. Unknown counts/weights/provenance do not
become zero or automatic source choices. Expected BOMs never create inventory.

Separate fulfilment intent, physical receipts/dispatch, warranty coordination,
financial price adjustments and payment settlement. Preserve paid line amounts;
explicit gross precedes new discounts. Corrections revalidate dependencies and
request/revision identities inside the write transaction, not only in previews.

Holdings and active copies share source/component cost budgets. Preserve residual/
disposed amounts and original prices. Reviewed reallocations append results;
rollups are non-owning views and cannot duplicate acquisition basis. Project cost
shares use actual allocated stock with exact rounding; estimates/reservations are
not owned cost. Explicit copy piece counts are costing inputs, not verification.

External systems enrich rather than own local truth. Review file mappings/fields
then apply atomically, preserving local UUIDs/overrides and cached source evidence.
Color/theme enrichment validates canonical settings prefixes and hierarchy. Theme
links use local UUIDs. Local image attachments retain originals/hashes; online
retrieval/synchronization and native Studio .io remain deferred.

## Persistence and consequences

Migrations 013–018 add project, physical/source-flow, fulfilment, derived-cost,
integration-cache and enrichment histories. Earlier migrations are immutable.
Typed replay, source snapshots/prefixes and the fixed native table registry retain
all new facts through snapshots/native restore. Search and rollups stay derived.

Current feature evidence: 184 Rust tests, 18 Chromium/Firefox scenarios and
format/lint/type/build checks. ALPHA_STATUS records dated backup/Docker evidence
and remaining acceptance. In particular, dedicated provenance reconciliation,
legacy/new completeness integration, definition reassignment and desktop/release
gates remain open. No automatic merge/tag/release follows source synchronization.
