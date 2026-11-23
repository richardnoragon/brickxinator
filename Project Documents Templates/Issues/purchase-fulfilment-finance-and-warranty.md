# Alpha scope: purchase fulfilment, finance, returns and warranty

> Status reconciliation, 2026-11-23: accepted requirements and implementation-time notes below are preserved. Current schema-18 implementation, evidence and remaining limits are in [ALPHA_STATUS](../../ALPHA_STATUS.md), [ROADMAP](../ROADMAP.md) and [ADR-023](../ADRs/ADR-023-expanded-alpha-ledgers-and-enrichment.md); old “pending/deferred” text is not the current feature checklist.

Status: accepted alpha requirements, clarified on 2026-11-22. Implementation and
validation pending. Local issue draft; GitHub publication was blocked by HTTP 401
at the last attempt. These requirements supersede the former purchase-aftercare
and partial-receipt deferrals; earlier test evidence covers the old runtime only.

## Principle and separate flows

Orders track intent. Shipments track logistics. Confirmed receipts create
ownership. Confirmed physical returns and valid receipt reversals remove ownership.
Financial events change financial records, not physical stock. Warranty claims
coordinate distinct receipts, returns or refunds, without duplicating their effects.
All operations retain append-only history and support audited correction events
with dependency/conflict checks; do not delete or mutate historical events.

Physical flow: Purchase Order -> Shipment -> Receipt -> Holdings/Set Copies.
Financial flow: source amounts -> discounts -> payments -> refunds/adjustments.
Warranty flow: claim -> decision/resolution -> referenced physical/financial events.
Marking a shipment Shipped or expecting delivery never creates stock. A Received
shipment is supported by actual receipt event(s), not an independent stock writer.

## Orders, shipments, partial receipts and backorders

Support multiple shipments/tracking references per order, partial invoice-line
receipt quantities, outstanding quantities and explicit backorders. Requested
order states include OPEN, PARTIALLY_RECEIVED, BACKORDERED, COMPLETED, CANCELLED;
partial/backordered facets can coexist without hiding outstanding line balances.
Keep fulfilment balances and manual closure/cancellation decisions distinct.
Cancellation of outstanding demand must never erase already received stock.

Shipment states include Expected, Shipped, Received, Lost and Returned. Preserve
tracking references and dated logistics events, manually entered in alpha.

Each receipt has its own permanent identity, source order/line/shipment references,
measurements and once-only stock effects. Receiving the same event twice creates
no extra stock. Multiple genuine receipts against one order remain valid. Receipt
of one line must not mark unrelated outstanding lines as received. Backorders are
expected acquisitions, not missing owned inventory or procurement-ready holdings.

Set receipts create physical copy identities only; never materialize advertised
piece counts or expected checklists as actual parts. Preserve unknown versus zero
and the existing independent piece/weight rules. Unknown copy counts never create
invented identities. Validate measured fulfilment/outstanding totals and surface
insufficient information rather than guessing receipt quantities.

Upgrade the existing whole-purchase once-only receipt into a historical receipt
without replaying its stock creation or rewriting original identifiers. New
fulfilment events and projections must preserve existing retry/request guarantees.

## Receipt reversal

An erroneous receipt is corrected by a referenced compensating receipt-reversal
event. Preserve the original event, measurements, provenance and all copy/holding
identities. Preview stock effects and dependencies, then revalidate atomically.

Allocated/reserved/sold/returned stock, subsequent splits/merges/measurements and
other dependent events may prevent a safe inverse. Block unsafe reversal with
specific holding/copy/claim explanations; no blind subtraction, negative stock,
partial mutation, implicit claim cancellation or resurrection of disposed stock.
A receipt reversal is not a supplier return and does not invent a refund.

## Discounts, coupons, payments and refunds

Keep pre-discount source amounts, named discount components and resulting net
amounts separately with audit history. Coupons and promotional/loyalty reductions change
price. Gift cards and stored-value vouchers are payment instruments, not discounts;
they must not reduce acquisition price a second time. Preserve currency/amount
unknowns and known zero; EUR cents and deterministic exact arithmetic remain.
Informational tax behavior remains; this issue does not add a tax calculation engine.

Refunds are separate from physical returns. Full/partial refunds without a return
leave stock unchanged. A return without a refund changes stock only at handover.
Link any combined workflow's financial and physical events without double effects.
Original purchase/payment history remains; changes appear as explicit events and
updated derived views, not overwritten source prices.

### Required automatic invoice-line allocation

Order-level discounts and refunds must be allocated automatically across invoice
lines. Offer Equal, Price Proportional, Piece Count and Weight; default to Price
Proportional. Use the existing exact-cent largest-remainder rule with stable
invoice-line ordering for ties. Discounts use pre-discount line prices as their
price basis; refunds use post-discount line amounts. Do not reuse a refund share
as a new stock quantity, shipping amount or second discount.

Missing/unknown basis inputs block allocation; no silent fallback to another
method. Preserve zero/all-zero and incomplete-basis safeguards. Persist the chosen
method, input/source revisions and resulting shares; re-preview after relevant
financial revisions. Financial line attribution does not silently allocate costs
among physical receipt holdings, classified children or merged source layers.
Purchase-Level/unapportioned and Manual modes are not substitutes for this required
automatic order-level allocation. Existing six shipping modes remain separate.

## Returns and copy lifecycle

Planning/requesting/approving a return does not change ownership. Confirmed actual
handover/dispatch to the supplier removes the explicitly selected stock through
an audited return event. Later supplier confirmation/RETURN_RECEIPT records
acknowledgment only; it must not remove the same stock a second time.

Add RETURNED to the copy lifecycle, preserving its permanent UUID, provenance,
attachments and event history. Exact linked-part disposition and shared holding
claims obey the related lifecycle/allocation model: explicit selected quantities,
no hidden cascade, no impact on other copies or unrelated bulk stock.

A physically different replacement set receives a new copy UUID through a linked
replacement receipt. Do not reactivate/reuse the returned object's identity for a
replacement. Audited correction of an erroneous return is distinct from a real
new receipt, and requires the same safe inverse/conflict review as other lifecycle
corrections. The correction itself does not create a financial refund/reversal.

## Warranty

Independent claim states: OPEN, SUBMITTED, APPROVED, REJECTED, REPLACEMENT_SENT,
COMPLETED. Preserve dates, reason, notes/evidence, original acquisition and resolution
links. Claim status alone never adds stock, removes stock or pays a refund.

Resolutions may link replacement-part receipts, new-identity replacement-set
receipts, physical returns and refunds, separately or together. Warranty stock
enters through a genuine receipt marked Warranty/replacement with explicit measured
quantities; no automatic purchase price or zero value is fabricated merely because
no new cash payment was recorded. Preserve original/replacement source links and
financial history. Reject duplicate resolution processing/idempotency conflicts.

## Accepted alpha exclusions

No carrier integration, automatic shipment tracking, marketplace APIs, refund
synchronization or vendor-specific warranty integration. Manual local workflows
must work offline. No payment processing, bank connection, statutory accounting
package, child-lot cost allocation or marketplace ordering is implicitly added.

## Acceptance outline

- Order three set lines; receive two in shipment A: exactly two copies exist,
  the third stays outstanding. Receive the third later without duplicating either.
- Order 100 parts; receive 80 and mark 20 backordered: owned stock is 80, not 100;
  backordered parts are not classified as missing owned stock.
- Repeat a receipt request; no duplicate ownership. Reverse an unused erroneous
  receipt with preserved history; active claims/later stock changes block unsafe
  reversal with explanations and no partial effect.
- Gross EUR 500 less EUR 25 coupon and EUR 10 promotional redemption yields net
  EUR 465. A stored-value payment contributes to settlement, not another discount.
- Allocate one cent across equal-price lines with stable remainder handling;
  shares exactly equal the event total. Exercise all four modes, unknown/zero
  inputs and stale previews; never silently fall back or change stock.
- Refund EUR 10 for missing instructions without returning the set: stock unchanged.
  Confirmed physical return removes selected ownership once, even when the supplier
  acknowledgment and refund arrive later. A planned return changes no stock.
- Returned set identity remains RETURNED; a physically different warranty
  replacement receipt creates a new copy UUID, linked to the claim/original copy.
- Claim approval/REPLACEMENT_SENT alone creates no stock. Genuine warranty receipts,
  returns and refunds apply their own distinct effects exactly once.
- Audited corrections retain originals. Replay, concurrent revision handling,
  rollback, native exchange, snapshots and interrupted restore retain physical,
  financial and warranty references and produce identical current projections.

## References and implementation dependencies

[Set lifecycle](set-copy-lifecycle-proposal.md),
[holdings/allocations](stock-holdings-and-set-allocations.md),
[build planning](build-planning-reservations-and-units.md).
D-159–D-168 record these requirements. Product questions are resolved; technical
migration/event/projection design and validation remain implementation work.


## Implementation checkpoint — 2026-11-23

Orders & warranty now includes separate gross amounts, exact automatically
attributed discounts/refunds, payment instruments, last-first audited financial
corrections, supplier plans/dispatch/acknowledgment/safe correction, and linked
claims/replacement receipts. D-191 preserves recorded paid totals. D-193 requires
explicit acquisition quantities for returns from cross-purchase merged stock.
A regression test confirms direct correction submission rechecks metadata
dependencies; a returned original and a replacement retain different UUIDs.
See ALPHA_STATUS for current validation and remaining acceptance work.
