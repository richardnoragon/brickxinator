# ADR-018: Exact invoice-line shipping allocation and cost views

Date: 2026-11-01. Status: implemented engineering decision within accepted
EUR purchase-cost and six-mode shipping scope. No accounting integration or
payment action is introduced.

## Allocation behavior

Purchase Level is the unpersisted default until explicitly changed. It records
shipping once in purchase totals and computes no line shares/effective allocated
line totals. Paid-price unit costs remain independently usable. Unknown shipping
keeps the purchase total unknown; known zero means free shipping.

Equal, Price Proportional, Piece Count and Weight target invoice lines, never
individual copies or parts. Equal treats a three-copy set line as one target.
Piece Count requires a known invoice brick count; copy quantity is not a basis.
Weight uses explicit acquisition milligrams. Missing inputs and all-zero bases
produce incomplete allocation. Valid zero bases receive zero shares when the
overall basis is valid. Zero shipping with missing/zero-total basis remains
incomplete rather than fabricating missing measurements.

Automatic shares use integer u128 products and largest remainders, with invoice
position as the tie breaker. Complete shares sum exactly to shipping. No floating
point or intermediary rounded percentage is used. A EUR 1 three-way split is
34/33/33 cents. Paid source prices and measurements never change with methods.

Manual shares remain exactly entered. Missing/unbalanced entries are valid draft
configuration, with an explicit remainder/overshoot and no effective allocated
costs. Balanced known shares complete allocation even if some paid prices remain
unknown; those particular effective costs remain unknown. Method switching keeps
the manual map untouched. Explicit SetManualShares replaces that map, retaining
the previous configuration in history. Newly supplied targets must belong to the
current purchase. If a later draft edit removes a targeted line, the retained
manual draft reports obsolete targets until explicitly corrected; nothing is
silently reassigned to another line.

## Cost and rate views

Cost views report known paid subtotal, unknown-price-line count, independent
shipping and an exact total only when all needed amounts are known. The total is
paid line prices plus shipping once. Optional tax information is not implemented
yet and must never be added again to authoritative paid amounts.

Paid and allocated unit rates are separate, exact reduced rational EUR/piece and
EUR/kg values. Only positive known acquisition measures yield a rate; missing
or zero denominators yield none. EUR 60 / 5 kg / 3500 pieces gives EUR 12/kg and
approximately EUR 0.017142857/piece. Callers choose 0–12 display decimals with
integer half-up rounding; display precision never changes stored ratios.
Wide amounts/ratios use u128 and serialize as decimal strings. There is no
per-copy price allocation, mixed-lot child costing or use of current residual
stock as the acquisition-cost denominator.

## Audit, persistence and concurrency

Migration 008 adds shipping configuration and immutable command audit. Allocation
revision and expected source-purchase revision both guard writes. Configuration
can change after receipt without changing purchase revision or inventory history.
IMMEDIATE transactions commit state/audit together. Shared request keys reject
cross-family reuse; identical retries return their original source/cost result
even after subsequent purchase/configuration edits.

Each audit freezes the complete source purchase revision and resulting cost view.
Replay verifies the source against the immutable purchase audit, re-applies the
configuration action and recomputes the exact result. Current views instead use
the current purchase snapshot, preserving manual configuration while recomputing
automatic shares. History returns UTC timestamps, actor/reason, source and
before/after configuration. Descriptive reasons are optional; actor/key are required.

The purchase FK is relational. Manual line references live in audited JSON maps
and are validated against the referenced source revision. A live per-share FK to
invoice_line is intentionally not used in this increment because pre-receipt
editing replaces invoice rows and removed manual targets must remain in history.
This is an engineering refinement of the proposed schema, not a claim that
foreign/current manual targets are accepted without validation.

Only new databases install schema 8. Known older schemas keep their workflows
without automatic upgrades. Cost/allocation methods explicitly require schema 8.
Earlier migrations remain unchanged. Inventory search does not need invalidation
for an allocation-only change because paid purchase totals and ownership stay
unchanged; finance views are derived separately from audited source metadata.

## Validation and remaining work

103 behavioral tests pass with rustfmt/clippy. New cases cover all modes, stable
rounding, grouped-copy lines, missing/zero bases, manual drafts/remainders,
unknown prices, exact unit rates, sums/products beyond u64, durable retries,
source/config conflicts, source edits/obsolete targets, atomic failure, tampering,
concurrent edits, shared keys, received-stock changes and schema-7 compatibility.
The demo ran: 34/33/33 cents, a preserved 50-cent manual remainder, EUR 12.00/kg
and EUR 0.017142857/piece, unchanged EUR 111.00 total, eight shipping revisions
and no inventory operations.

Purchase dates/tax metadata, per-definition price history, persistent precision
preferences/UI, received-purchase reconciliation, attachments, backup/restore,
safe upgrades and the API/UI remain pending. Audit/cost reads replay history and
have no measured large-collection performance claim.
