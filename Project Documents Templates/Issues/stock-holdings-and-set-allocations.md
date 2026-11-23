# Alpha scope: physical holdings and audited set allocations

> Status reconciliation, 2026-11-23: accepted requirements and implementation-time notes below are preserved. Current schema-18 implementation, evidence and remaining limits are in [ALPHA_STATUS](../../ALPHA_STATUS.md), [ROADMAP](../ROADMAP.md) and [ADR-023](../ADRs/ADR-023-expanded-alpha-ledgers-and-enrichment.md); old “pending/deferred” text is not the current feature checklist.

Status: accepted alpha requirements, clarified on 2026-11-22. Ready for
implementation design; not implemented. This is a local issue draft. GitHub
publication was blocked by HTTP 401 at the last attempt.

## Goal and core model

Physical holdings are the sole source of owned part quantities. Set copies
reference quantities through allocations/assignments; a copy and its allocation
never add a second inventory balance. Loose stock is first-class inventory.
A holding can have no allocation or multiple quantity-bounded allocations.

Allocations reserve their quantities regardless of assembly state. Assembly
records physical construction only; Loose does not mean Available. Storage and
physical verification are independent of ownership, allocation and assembly.
This explicitly supersedes D-112's assembly-based reservation policy.

## Accepted alpha operations

- Split Holding: partition measured stock into independently addressable holdings
  with traceable source relationships and no new ownership.
- Merge Holdings: logically consolidate compatible physical stock, including
  multiple purchases, while preserving each underlying source lot and its history.
- Allocate Stock To Set: assign existing owned stock without receiving, creating
  or counting it again. This explicitly replaces D-101's deferral.
- Remove Allocation From Set: release all or some assigned stock as loose stock;
  current ownership is unchanged.
- Transfer Allocation Between Sets: move an assignment between copies atomically,
  without duplicating ownership or intermediate double allocation.

Arbitrary splitting and independent set-part handling are now requested alpha
scope, replacing the restrictions in D-041/D-066. Allocated parts may be stored
separately from their set without releasing their allocation. This extends D-139/D-140 beyond lifecycle-only disposition.
MOC/restoration examples initially motivated only these stock operations.
D-150–D-158 subsequently add a separate alpha build-reservations/planning issue;
see [build planning](build-planning-reservations-and-units.md). Its product
policies are resolved, including future/unowned plans, verified readiness,
progress-only units and offline downloads. MOC authoring is not included.

## Inventory and provenance invariants

- Every operation creates an immutable audit event with UUIDv7, local actor,
  timestamp, reason, previous/new values and referenced holdings/allocations/copies.
- Internal holding, allocation, copy and catalog identities are permanent. A
  merged-away holding remains in history with traceable source/target links.
- Split/merge/allocation conserve total owned stock. Allocation totals across
  copies must never exceed the source holding's owned quantity in a known unit.
  Unknown stock is not an unlimited allocation budget.
- Keep piece and weight measurements independent. No estimated conversions or
  arbitrary proportional weight/cost allocation. Preserve Unknown versus zero.
- Damage remains owned but unavailable. Split/merge/assignment must account for
  damaged subsets without repairing damage. General availability excludes the
  union of allocated and damaged stock; an allocated damaged piece must not be
  subtracted twice. Assembly changes neither ownership nor reservation.
- Stock metadata/compatibility must be checked: definition, color, condition,
  location and relevant physical attributes. Never silently combine incompatible
  stock. Preserve every purchase/source layer during cross-source merges;
  source invoice prices and historical costs remain authoritative. Display one
  compatible inventory group while retaining individually traceable source lots,
  rather than destructively replacing them with an average acquisition record.
- Whole-holding location and quantity layers must remain consistent: a stock
  portion moved separately needs its own physical location representation, even
  while allocation remains linked to a set. A copy's location cannot silently
  overwrite its allocated stock's independent locations. A logical merged view
  must retain source/location layers and must not imply a physical relocation.
- Previews, stale-revision checks, idempotent retries and atomic transactions
  protect every multi-holding/multi-copy operation. Reject negative balances and
  over-allocation without partial mutation.

## Set verification and completeness

Derive current set completeness from expected requirements and actual allocated
stock, rather than treating a stored COMPLETE flag as permanent truth. Calculate
coverage per part/color requirement: an excess of one part must not compensate
for another missing part. Unknown/incomplete expectations or measurements must
not produce an exact percentage or an unsupported COMPLETE result.

Physical verification remains explicit and separate: unfinished observations
are Unverified, not automatically Missing (D-100/D-107). Project completeness as
Unknown, Complete or Incomplete with the relevant verification timestamp/context.
Only sufficiently verified evidence may support Complete/Incomplete; nominally
matching expected and allocated counts alone are insufficient. For example,
expected=9090 and allocated=9090 can still mean Unknown when not checked.
Allocation coverage and verified completeness are separate readouts; verification
completion and physical completeness must not be conflated.

Changes to relevant quantities/identity/allocations must invalidate or re-evaluate
previous verification coverage rather than retain an unjustified Complete badge.
Historical verification timestamps and observations remain inspectable. Moving
allocated stock physically does not by itself change its ownership/allocation.

An allocation removal/transfer changes derived current coverage without erasing
historical observations. Expected/checklist changes create no stock. Moving
already-owned loose stock into a set creates no fresh ownership; counting newly
observed stock must remain explicitly distinguishable to prevent double counts.

## Interaction with set-copy lifecycle

The lifecycle issue's explicit Transfer all / Keep all / Custom quantity operates
on allocated physical stock rather than a second set-owned balance. Only an
explicit sale/loss/donation/etc. stock disposition reduces ownership; releasing
an allocation alone does not. Kept parts become unallocated independent loose
stock. Never touch unallocated bulk stock merely because it shares provenance.

Shared holdings require precise quantity-level effects: disposing one copy must
not remove another copy's allocation. Stock reduction must reconcile affected
allocations atomically. An allocated quantity cannot remain usable by a copy
following an ownership reduction that makes the allocation exceed owned stock.

Lifecycle reversal impact analysis must include later splits, merges, allocation
changes and physical stock movements; unsafe inverses are blocked for review.
Preserve original events and definition/copy UUIDs.

## Resolved policy decisions — 2026-11-22

1. Availability: allocation reserves stock even when a set is Loose. Construction
   state does not release allocations. Only unallocated, otherwise usable stock
   contributes to general availability, subject to measurement coverage.
2. Location: allocated parts may remain linked while stored anywhere. Stock
   location and allocation are independent; no automatic deallocation on a move.
3. Verification: keep explicit physical verification and Unknown. Not checked is
   not Missing. Derive completeness using sufficiently verified evidence rather
   than nominal allocation totals alone, preserving verification history.
4. Merge scope: compatible holdings from multiple purchases may be logically
   merged for usability. Preserve source-lot quantities, acquisition references,
   historical prices and audit layers underneath the combined inventory view.

No further product clarification is currently required for this issue. Migration,
source-layer tracking, conflict checks and projection design remain engineering
work. This does not approve new valuation calculations, MOC authoring, instruction/box inventory entities or
other features merely illustrated by the examples. Subsequent build-planning
requirements are tracked separately under D-150–D-158.

## Acceptance outline

- Split 100 known bricks into 40/20/40 with unchanged total ownership and sources.
- Allocate 40 to one set and 20 to another; ownership remains 100 and, if all
  stock is known undamaged, general availability is 40 even if both sets are Loose.
  An attempt to allocate another 41 fails without mutation.
- Change a copy from assembled to Loose: its allocation and reservation remain
  unchanged. Release an allocation explicitly to make usable stock available.
- Move an allocated portion to another location: allocation and ownership remain
  unchanged, and the copy and source/location history remain traceable.
- Remove 10 from a set allocation; loose quantity increases by 10 and that copy's
  coverage decreases; ownership and the other copy are unchanged.
- Allocate purchased loose parts into an incomplete set without creating stock.
- Transfer an assigned quantity between two copies atomically and audit both ends.
- Logically merge compatible source lots of 100 from A, 200 from B and 50 from C:
  display 350 while history still exposes exactly 100/200/50 with each acquisition
  source. Splits/transfers remain traceable through those layers. Incompatible
  merges fail without mutation; no automatic average price/cost is invented.
- Matching expected and allocated counts without physical verification project
  Unknown. Verified per-part evidence can support Complete/Incomplete, without
  excess parts compensating for a different shortfall. Relevant allocation edits
  re-evaluate verification coverage; unverified shortfalls never become Missing.
- Retired checklist stock, damage, unknown units, assembly changes and partial
  quantities obey the approved policies and preserve current/historical views.
- Competing tabs and repeated requests cannot over-allocate or duplicate movements.
- Native exchange, journal replay, search/totals, snapshots and restore preserve
  allocation provenance and yield the same ownership/completeness projections.
- Existing collections upgrade without generating stock, rewriting historical
  receipt facts or silently changing unknown measurement coverage.

## References

D-041/D-066: former storage/splitting restrictions. D-069: damage stays owned;
D-112: former assembly-based availability, superseded by D-146. D-096/D-100/D-101/D-104/D-107: ownership/checklist/verification
separation. D-120: movement-based stock. D-136–D-142: accepted lifecycle changes.
D-143–D-149: expanded alpha stock/allocation scope and resolved policies.
See set-copy-lifecycle-proposal.md for the related lifecycle acceptance outline.
