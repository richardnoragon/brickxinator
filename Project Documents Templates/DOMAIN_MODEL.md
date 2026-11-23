# Brickxinator domain model

Current implementation contract, 2026-11-23. See [ADR-023](ADRs/ADR-023-expanded-alpha-ledgers-and-enrichment.md),
[DECISIONS](DECISIONS.md), [executable schema](../migrations) and
[remaining acceptance work](../ALPHA_STATUS.md).

## Identity and physical truth

Domain entities use immutable UUIDv7 IDs. Business identifiers/metadata can be
corrected through audit history. Catalog definitions/templates describe products;
they do not create stock. Physical set copies are separate from definitions and
from detailed parts. An unknown copy count creates no fictitious copies.

Holdings represent owned parts/residue. Count, milligrams and damaged subsets are
independent; Unknown is not zero. No piece/weight conversion is inferred. Damage
reduces usable stock while remaining owned. Classification, splits, merges,
verification and disposal conserve recorded quantities or explicitly preserve
measurement uncertainty. Merges retain source identities/cost layers. Ambiguous
mixed-source quantities stay unknown; returns cannot silently choose FIFO.

Allocations/assignments reserve stock independently of construction state and
location. Loose does not mean unallocated. Legacy set-verified holdings have an
explicit assignment until detached; project claims represent quantity-based
reservations/allocations. Their remaining integration limits are documented.

## Set-copy lifecycle and verification

Active copies may become Sold, Donated, Lost, Stolen, Destroyed, Void or Returned.
Never delete a received identity. Disposal explicitly transfers, keeps or
partially retains linked holdings; retained stock becomes loose. Corrections
reference prior events and restore effects only if stock, claims and metadata
remain safe. Archiving only controls visibility.

Assembly describes construction. Verification records observed reality, with
unknown/incomplete/complete evidence independent of expected BOM coverage.
Expected checklist changes do not create stock; counts are absolute observations.
Later stock changes can require reconciliation rather than an implicit recount.
Audited reassignment to a different definition remains follow-up acceptance work.

## Projects and planning

Each project is one execution (Official Set Build, MOC, Train Layout, Diorama or
Custom), referencing an immutable design/BOM version. Multiple executions may
share a version. Plans can be unowned. Official builds may link real copies;
other projects receive allocations directly without artificial copies.

Reservations hold future supply; allocations assign actual quantities. Both
reduce generally available supply. Verified usable reserved stock and no known
shortages are required for Ready To Build; Start explicitly converts reservations
to allocations. Confirmed missing, potential gap and unverified assignment are
separate. Bags/subassemblies track progress, not per-part checklists or geometry.
Market observations/manual estimates inform planning, never acquisition cost.

## Purchases, returns and finance

Orders describe intent; only actual receipts create ownership. Partial receipts,
shipments/backorders and closure remain separate. The original whole-purchase
receipt is a preserved historical fact, not an alternate way to duplicate partial
receipts. Reviewed reversal compensates a mistaken receipt; dependent stock
movements/claims can block it.

Supplier return plans do not change stock. Actual dispatch removes selected
ownership once; acknowledgment does not repeat removal. Merged returns explicitly
identify acquisition quantities and cannot exceed known source availability.
Returned copies retain UUIDs. Genuine warranty replacement receipts create new
physical identities; warranty status alone creates neither stock nor refunds.

Original paid invoice-line amounts remain historical facts. Explicit gross
amounts precede new discounts/coupons. Refunds are financial, independent of
returns; payments/gift cards/vouchers are settlement, not another price reduction.
Automatic financial allocation supports Equal, Price Proportional, Piece Count
and Weight, with exact stable cent handling and no missing-input fallback.
Later dependent entries must be corrected before earlier ones.

Shipping has seven modes: Purchase Level, Equal, Price Proportional, Piece Count,
Weight, Manual, Volume. New purchases default to Volume; missing volume remains
incomplete. Tax stays informational; fees are separate.

## Cost provenance and rollups

Derived Manual/Piece Count allocations assign source/component budgets to genuine
holdings and active copies of the acquisition. Copy piece-count inputs are
explicit accounting bases, not physical verification. Direct copy and part shares
share the same budget; residual and disposed costs are explicit. Reallocation
appends history, preserves original paid amounts and requires source/stock
revision checks. Safe copy lifecycle corrections restore cost effects; reviewed
cost-only corrections can handle refunded sources with no active targets.

Holding/copy/project rollups are views. Direct copy cost and allocated-part cost
stay distinct; collection totals count each share once. Project shares use actual
allocated quantities and exact rounding with an unallocated-stock remainder.
Reservations/market estimates are not owned acquisition cost. Missing or stale
cost information is shown, not converted to zero.

## Catalog enrichment and files

Reviewed imports enrich local definitions, BOM versions, colors and themes.
External IDs are explicit mappings, never substituted for local UUIDs. Source
files, hashes and reviewed field choices persist. Color names/metadata and theme
parents retain unselected local overrides. Protected Unknown cannot be repurposed.
Theme links use stable UUIDs alongside legacy free text. Local images/PDFs retain
original bytes, hashes and associations; no external image fetch is performed.

## Recovery and scope

SQLite command/movement histories are authoritative; search/cost rollups are
rebuildable. Upgrades, snapshots, native exchange and restore validate histories
and referenced assets before replacing data. Restic daily protection and monthly
ZIPs are separate. Filesystem sync of the live database is unsupported.

File limits, unresolved reconciliation/UI work and release gates are tracked in
ALPHA_STATUS.md. Discovery-era deferrals do not override later accepted decisions
or the current tested implementation.
