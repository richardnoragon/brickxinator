# Alpha scope: build reservations, build units and missing-parts planning

> Status reconciliation, 2026-11-23: accepted requirements and implementation-time notes below are preserved. Current schema-18 implementation, evidence and remaining limits are in [ALPHA_STATUS](../../ALPHA_STATUS.md), [ROADMAP](../ROADMAP.md) and [ADR-023](../ADRs/ADR-023-expanded-alpha-ledgers-and-enrichment.md); old “pending/deferred” text is not the current feature checklist.

Status: accepted alpha requirements, clarified on 2026-11-22. Product questions
resolved; implementation and validation pending. Local issue draft; GitHub publication was blocked by HTTP 401
at the last attempt. This issue extends D-143–D-149; it is not evidence that the
current Docker application supports the new operations.

## Goal and boundaries

Track ownership through physical holdings, existing set assignments through
allocations, future intended use through reservations, construction progress
through build units, and procurement needs through missing-parts planning.

Include in alpha: reservations, bag tracking, subassembly tracking, per-set
missing-parts lists, a global planning dashboard and a build-status workflow.
Keep individual-piece assembly and instruction-step tracking out of alpha.
Plans may target owned copies or unowned/future sets, including wishlist/future
purchase and hypothetical projects. Planning creates no copy or owned parts.
This does not add a MOC editor or instruction parser. Expected requirements must
be explicit; a set's advertised total piece count is not a part-level checklist.

## Distinct concepts

- Holding: physical owned stock, with permanent IDs and source-lot provenance.
- Allocation: quantities currently assigned to a set copy; already reserves those
  quantities regardless of assembly (D-146).
- Reservation: a claim on currently unallocated loose stock for a future build
  purpose. It does not create ownership, a new allocation or a physical move.
- Build plan: requirements and intended construction work, with an optional owned
  copy link. Ownership association is independent of construction and readiness.
  PLANNED_OWNED / PLANNED_NOT_OWNED can be derived presentation labels; neither
  purchasing a definition nor creating a plan invents a physical copy.
- Build unit: Bag or Subassembly, not interchangeable concepts despite a shared
  structural entity. Each has its own identity, label and progress history.
- Planning shortfall: uncovered requirements for a build, whose evidence must be
  distinguished from physically verified missing contents.

One stock quantity cannot simultaneously satisfy competing reservations and/or
allocations. Example: 100 known usable pins, 60 allocated and 20 reserved leaves
20 generally available. Reserve only actual eligible stock; an unmet requirement
is not a reservation of nonexistent inventory. Do not assume unknown quantities
are unlimited or turn damaged stock into usable build supply.

Reservations must be auditable and releasable/correctable. Conversion to an
allocation must not count the same stock twice or create ownership. Start Build
is an explicit audited action: verify eligible coverage, review the target copy,
then convert the plan's reservations into allocations atomically. D-184 later
extends targets to projects directly: MOC/layout/diorama/custom projects do not
need a fake Set Copy. Official-set projects may link to real copies; linked views
must not double-claim stock. Planning still creates no physical copy/ownership. Concurrent plans, stock
reductions, splitting, logical merges, lifecycle disposition and correction
reversal must preserve claim/provenance integrity atomically.

## Bags and subassemblies

Bag progress: Unopened, Opened, Completed.
Subassembly progress: Not Started, In Progress, Completed.

Examples: Bag 1/2/3 versus Bow/Midship/Stern/Display Stand. Preserve the type
and its specific progress vocabulary rather than pretending bags and structural
sections mean the same thing. A completed bag/build unit is not proof that every
part is present, nor an automatic stock movement or physical verification.

Alpha units track labels and progress only. Per-unit part checklists, optional
expected contents, instruction parsing and individual installed-piece states are
not alpha requirements. Demand comes from the plan/set requirements, not an
invented sum of bag and subassembly contents. Completed progress never certifies
physical completeness.

## Build status and readiness

Keep construction progress and derived readiness separate while presenting the
requested workflow labels:

- PLANNED: requirement/need analysis is not finished.
- BLOCKED: known shortages prevent readiness (the earlier label
  BLOCKED_MISSING_PARTS describes this same reason).
- POTENTIALLY_READY: declared coverage exists but verification is insufficient.
- READY_TO_BUILD: all known requirements have usable, physically verified,
  committed stock coverage and no known shortage; new supply must be reserved.
- IN_PROGRESS / COMPLETED / DISMANTLED: explicit construction progress/history.

Existing allocations to the relevant copy already commit their quantities; do
not reserve them a second time. Unallocated merely available stock does not
qualify a plan as ready until reserved. Unknown requirements/verification must
not yield READY_TO_BUILD. A current shortage or unverified gap is distinct from
a historical Completed build. Coverage/readiness must be re-evaluated after
relevant stock, reservation, allocation or verification changes; do not erase
construction or verification history.

Dismantling describes construction, not ownership or automatic claim release.
Keep D-146: Loose/disassembled does not release an allocation. No build status
alone invents parts, sells stock, deallocates it or verifies it physically.

## Per-set and global procurement planning

Compute requirements by part definition/color and applicable compatibility, not
one summed piece-count difference. Excess of one part cannot cover another part.
Show required quantities, assigned coverage, reserved coverage and unresolved
shortfalls, preserving Unknown and measurement/evidence coverage.

Display Confirmed Missing, Potential Gap and Unknown/Unverified separately,
with evidence/verification context and contributing plan/part drill-down. Nominal
expected=100 and allocated=93 does not prove seven are physically missing.
Only verified evidence supports Confirmed Missing; an uncovered planning need
can be Potential Gap, and insufficient verification remains Unknown/Unverified.
These categories may describe overlapping evidence; do not sum them blindly as
three disjoint procurement totals or present unverifiable counts as exact.
Matching nominal allocations alone remains insufficient to assert physical
completeness under D-148.

Aggregate across included plans, with drill-down to the contributing sets and
units, while counting each demand and stock commitment once. Shared physical
stock cannot satisfy several plans simultaneously. Avoid claiming an exact global
shortfall when some requirements are unknown; expose partial coverage.

Requested prioritization examples: Critical/Normal/Optional and Active Build/
Backlog. Keep part requirement importance distinct from project priority. No
automatic reassignment or cancellation of lower-priority reservations is implied.
Show blockers for active work before backlog where priority is configured.

## Downloadable procurement only

Alpha provides offline export/download acquisition lists, not marketplace
transactions. CSV, XML, BrickLink Wanted List and BrickStore formats were given
as examples; format-specific adapters must validate their target requirements
and identifier mappings rather than guessing external part/color codes. The
accepted boundary is downloadable lists, not a promise of live marketplace data
or an implicit requirement to implement every example format.

Keep confirmed shortages separate from potential/unverified needs in display and
export. Preserve unknown coverage; do not silently turn every potential gap into
a confirmed purchasing requirement. No live login, cart management, order
placement/synchronization, store comparison or price optimization in alpha.

## Resolved decisions — 2026-11-22

1. Plans can exist without an owned set copy and may target future/hypothetical
   builds without creating ownership.
2. Separate confirmed shortages, potential planning gaps and unknown/unverified
   coverage; do not equate unverified with missing.
3. Bags/subassemblies are progress-only in alpha; no per-unit part checklists.
4. Ready requires verified usable committed coverage, no known shortage and
   explicit audited reservation-to-allocation conversion when starting a build.
5. Procurement is export/download only; live marketplace operations stay deferred.

No further product clarification is currently required. Technical design and
acceptance must preserve these rules and the related stock/lifecycle invariants.

## Acceptance outline

- Of 100 usable pins, allocate 60 and reserve 20: availability is 20 regardless
  of assembly. A competing reservation of 21 fails atomically.
- Create a plan without an owned copy: no stock or copy is created. Reserve
  eligible loose stock for it without claiming that the future set was received.
- Release/correct a reservation without changing owned totals. Start Build uses
  explicit confirmation and a valid project/copy target; convert reservations atomically,
  without duplicate claims or fabricated stock. Stale verification/claims conflict.
- Display bags and subassemblies with their distinct statuses and audited changes.
  Progress edits never create part stock or mark contents physically verified.
- Per-part/color needs are calculated correctly; a surplus of another part cannot
  hide a shortfall. Unknown expected/observed counts remain explicit.
- Several plans aggregate requirements with per-plan drill-down and no double
  use of reserved stock or duplicate unit/set requirements.
- Missing-parts/procurement output distinguishes confirmed, potential and unknown
  evidence without double-counting. No live marketplace request is needed.
- Declared sufficient but unverified stock yields POTENTIALLY_READY. Sufficient
  available but unreserved stock is not READY_TO_BUILD. Verified usable reserved
  coverage with no shortage can yield READY_TO_BUILD. Relevant changes invalidate
  unsupported readiness while construction/verification histories remain intact.
- Priority affects presentation without silently stealing another plan's stock.
- Native exchange, snapshots, restore and replay retain reservations, targets,
  units, statuses, requirement identities and provenance.
- Existing data migration neither creates stock nor invents verified status,
  construction progress, bag contents or reservations.

## Related issues

[Physical holdings and set allocations](stock-holdings-and-set-allocations.md)
and [set-copy lifecycle](set-copy-lifecycle-proposal.md). These dependencies must
be designed together; this issue extends the former build-planning deferrals,
not the exclusions on per-piece assembly or instruction-step tracking.

## Subsequent project scope

D-181–D-186 establish one execution per Project, share immutable design/BOM
versions across repeated projects and allow direct non-set allocation targets.
See [the design-project issue](design-projects-moc-versions-and-layouts.md).
Earlier copy-only targeting text is superseded where it conflicts.
