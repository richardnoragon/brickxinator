# Alpha scope proposal: cost provenance, child allocation and landed cost

> Status reconciliation, 2026-11-23: accepted requirements and implementation-time notes below are preserved. Current schema-18 implementation, evidence and remaining limits are in [ALPHA_STATUS](../../ALPHA_STATUS.md), [ROADMAP](../ROADMAP.md) and [ADR-023](../ADRs/ADR-023-expanded-alpha-ledgers-and-enrichment.md); old “pending/deferred” text is not the current feature checklist.

Status: accepted alpha requirements, clarified on 2026-11-22. Volume shipping,
explicit line volumes and separate acquisition fees are implemented in the local
increment. Child-cost allocation/split/reallocation implementation remains pending. Local issue draft. GitHub publication
was blocked by HTTP 401 at the last attempt.

## Principle

Acquisition/source history remains authoritative and auditable. Child sets and
holdings receive derived cost shares through immutable allocation events. Neither
allocation nor reallocation overwrites original paid amounts, receipts or earlier
cost results. Audited source corrections remain possible with old values retained;
“never overwrite” does not prohibit an explicit correction event.

Keep physical quantity movements, set allocations/reservations, cost allocations,
market value and payment settlement separate. A cost event creates no stock or
ownership. Acquisition basis must not be counted both at the source and again at
its derived children in a collection-value total.

## Alpha child-cost methods

Bring mixed-lot cost attribution into alpha, superseding D-067's deferral.
Support Manual and Piece Count. Market-value-based allocation stays deferred;
Equal Distribution was illustrated but is not a separate requested alpha method.

Cost allocation events preserve source UUID, source-cost revision, target IDs,
method, effective date/recorded timestamp, local actor, reason, basis inputs and
exact component/result shares. Reallocation creates a new event superseding the
prior effective result without erasing it. Reversal is an audited correction,
with impact/conflict review across affected descendants/source layers.

Manual allocation must reconcile to the applicable known source basis; unbalanced
or missing shares remain draft/incomplete and cannot appear as complete child
costs. Piece Count needs explicit known eligible counts and a positive total
basis, with no hidden fallback, inferred physical contents or weight/count
conversion. Unknown source costs never become zero through allocation.

Use exact EUR cents and deterministic largest-remainder rounding with stable
target order. Shares plus any approved explicit residual cost must reconcile
exactly to the applicable source basis. Original acquisition history retains
its original amounts even after derived effective costs are reallocated.

## Splits, logical merges and provenance

A known 100-piece holding with EUR 25 basis split into 40/60 carries EUR 10/15
respectively, conserving the source basis. Store the source-layer linkage and
rounding result. Do not leave both parent and children independently carrying
the same current owned basis. Repeated requests must not allocate it twice.

Compatible stock merged across purchases keeps each acquisition/cost layer.
Subsequent splits/dispositions must preserve those layers rather than average
away provenance. A proportional split needs a known supported physical basis;
when missing, require an explicit valid/manual cost decision, not invented counts,
weights or equal-cost assumptions. Physical events and their required derived
cost consequences must remain atomic or visibly incomplete, never falsely final.

Keep unidentified residual lot cost as an explicit unallocated source balance.
Child allocations plus this residual reconcile to the applicable source basis;
partial identification never forces invented child identities or cost shares.

## Landed-cost components and shipping

Show acquisition item price and allocated landed cost separately. Preserve item
price, shipping, tax information and fees as distinct components, with source
links. Never add included tax twice or subtract a stored-value payment instrument
as a discount. Retain D-051/D-161 informational tax rules; add separately recorded fees.

Volume is the seventh shipping mode and the default for new app/API purchases.
Retain Purchase Level, Equal, Price Proportional, Piece Count, Weight and Manual;
preserve existing saved choices/defaults when upgrading. Unknown volume leaves
allocation incomplete; changing method is explicit, never a silent fallback.

Volume must have explicit units/known basis. A line's total volume must account
for its physical packaging/quantity without guessing from set piece counts,
stock weight or an unknown copy count. Unknown versus zero stays distinct. Use
exact proportions/cent reconciliation. Example: volumes 10/30 liters and EUR 20
shipping yield EUR 5/15 without changing item prices or physical quantities.

Shipping allocation is distinct from child cost allocation and the mandatory
financial line allocation from D-165/D-166. Changing shipping method alone must
not overwrite paid prices. Financial discounts/refunds/source corrections after
child cost allocation require reviewed downstream cost-reallocation events,
never silent automatic repricing; preserve every historical result.

## Resolved product rules

D-187–D-190 confirm: retain six modes and add Volume as the new-purchase default;
missing volume stays incomplete; retain explicit residual source cost; preserve
informational tax and add fees; require reviewed downstream reallocation.
No cost product questions remain open. Do not confuse these accepted requirements
with implemented child-cost allocation, which remains pending.

## Acceptance outline

- EUR 200 source price plus EUR 20 allocated shipping yields EUR 220 basis once.
  Manual child shares 100/50/70 reconcile to 220; original history is unchanged.
- Piece Count shares based on 100/200/700 total 1000 distribute a known basis
  in those proportions with exact cent reconciliation. Unknown/zero basis cannot
  silently produce equal shares or fabricated complete costs.
- Reallocate Piece Count to Manual: both events remain inspectable; only the
  selected current result contributes to current cost projections.
- Split 100 pieces/EUR 25 into 40/60: child costs EUR 10/15, no duplicate parent
  balance. Exercise non-divisible cents and stable remainder handling.
- Cross-source logical merge and later split retain every source layer, paid
  history and derived cost references. Cost totals reconcile before/after replay.
- Volume example yields EUR 5/15 from EUR 20 and 10/30 liters. Missing basis,
  saved configurations and other modes follow the clarified policy.
- Later refund/source correction, disposition and reversal cannot silently lose
  cost provenance, double-charge tax/shipping or invent a current child cost.
- No cost-only command changes owned quantities, allocations or reservations.
  Stale/repeated requests, audit failure and unsafe inverse fail atomically.
- Migration/native exchange/snapshot/restore retain sources, allocation revisions,
  exact component shares, residual balances and correction references.

## Dependencies

[Stock/provenance](stock-holdings-and-set-allocations.md),
[lifecycle](set-copy-lifecycle-proposal.md),
[purchase finance](purchase-fulfilment-finance-and-warranty.md).
D-169–D-173 and D-187–D-190 record accepted direction and resolved policies.
Market-value allocation, live valuation feeds and currency expansion are excluded.


## Implementation checkpoint — 2026-11-23

A separate child-holding cost ledger now supports Manual/Piece Count, item/
shipping/purchase-fee components, residual source balances, historical reallocation,
proportional split costs and separate layers through merges. Explicit merged
returns affect only selected source quantities/cost layers. Source component
changes flag required review; reviewed disposed-cost overrides preserve audit
history when downstream financial changes require them. Direct physical-copy
assignment and cost rollups remain follow-up work; current allocation targets are
holdings. See ALPHA_STATUS for validation and operating limits.


## Copy-cost and rollup checkpoint — 2026-11-23

D-194 is implemented: existing active copies and holdings share one source/
component budget. Direct copy shares, actual allocated-part rollups, exact project
shares, residual source balances and disposed costs remain distinct. Corrections
restore safe lifecycle effects and permit reviewed cost-only adjustments after a
full refund without fake active targets. Copy piece-count inputs are explicit
costing bases, never ownership/verification. Rollups are views of reviewed costs
and do not add duplicate acquisition basis. Native restore preserves these facts.
