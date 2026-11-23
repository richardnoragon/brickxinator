# Alpha scope: unified projects, MOC versions and inventory-aware design planning

> Status reconciliation, 2026-11-23: accepted requirements and implementation-time notes below are preserved. Current schema-18 implementation, evidence and remaining limits are in [ALPHA_STATUS](../../ALPHA_STATUS.md), [ROADMAP](../ROADMAP.md) and [ADR-023](../ADRs/ADR-023-expanded-alpha-ledgers-and-enrichment.md); old “pending/deferred” text is not the current feature checklist.

Status: accepted alpha requirements, clarified on 2026-11-22. Product questions
resolved; implementation and validation pending. Local issue draft; GitHub
publication was blocked by HTTP 401 at the last attempt.

## Boundary

Brickxinator manages collection, stock, projects, procurement and build readiness.
External applications author CAD geometry, render models, generate instructions
and design physical layouts. Alpha imports/exports supported data and analyzes
BOMs; it does not edit geometry, render, geometrically validate models or replace
Studio/LDraw editors. No promise of native CAD-file round-trip editing is implied.

## Unified Project

Project types: Official Set Build, MOC, Train Layout, Diorama, Custom Project.
Use shared build status, reservations, allocations, missing-parts planning,
procurement, version references, notes and attachments rather than separate
inventory/planning systems per type.

Each project represents one planned/build execution, with permanent project UUID
and its own claims/progress. Two executions of the same MOC are separate projects
that may reference the same design/BOM version. This does not create stock,
authorship or an owned Set Copy. Ownership, authorship and progress are independent.

Projects are valid allocation targets directly. Non-set projects never require
an artificial Set Copy merely to receive assigned physical stock. An official-set
build may link to its real owned copy; source quantities committed through that
link must not be allocated/reserved twice or counted as two distinct demands.

This explicitly extends D-154/D-157's earlier copy-only allocation boundary and
D-143's set-only target scope. It does not authorize duplicating existing stock
or assigning the same quantity simultaneously to a project and its linked copy.
Start Build still explicitly converts the project's reservations to allocations,
revalidating usable verified coverage and the target, without inventing ownership.

## First-class MOC/design records and immutable BOM versions

A MOC/design records name, author, description, source files, notes and versions.
Project views expose status, source/BOM version, inventory coverage, missing needs
and completion-cost estimates. A design can exist without an execution or owned
parts; a user can build someone else's design without claiming authorship.

Each published BOM snapshot/version retains a permanent ID, version label,
timestamp, source-file reference and notes. Inventory/readiness/procurement
analysis is always against an explicitly selected version, not an automatically
changing “latest” requirement list. Store imported mappings and source provenance.

Importing/editing a BOM to create v2 preserves v1. Existing projects remain pinned
to their selected version until an explicit reviewed version change. Show added,
removed and quantity/identity changes and affected reservations/allocations/
verification before apply. Re-evaluate readiness with relevant current stock;
never silently move claims, rewrite observations or delete prior analysis/history.
Expected requirements, even in a complete imported BOM, never create owned stock.

Reusing a design/version does not share two projects' reservation budgets or
construction progress. Per-part installed-state tracking and unit-level parts
checklists remain excluded under D-156.

## Train layouts without geometry authoring

A layout project records name, footprint information, external source files,
required parts/track, inventory coverage, missing parts, estimate and build status.
Footprint is descriptive metadata, not a geometry engine. Use explicit BOMs or
supported import adapters; an arbitrary attachment is not proof of an extractable
part inventory. Show unknown/unmapped coverage rather than invent track counts.

Analyze requirements such as curves and switches by mapped part identity and
quantity, preserving relevant variants. No track editor, route simulation, signal
simulation, train scheduling or geometry authoring in alpha.

## Coverage, claims, procurement and estimates

Workflow: import design -> review BOM/version -> compare eligible inventory ->
distinguish confirmed/potential/unknown needs -> procurement plan -> explicitly
reserve available stock -> verify ready -> Start Build/allocate -> record progress.

Use the same holdings/source layers and allocation/reservation constraints across
all project types. Display owned, allocated, reserved and general available
coverage separately; do not treat stock held for another project as available.
Location and assembly remain independent; protected claims are not automatically
released by changing progress or design version.

READY_TO_BUILD follows D-157: usable verified committed supply, known requirements,
no known shortage. Merely importing a source file or matching nominal counts is
not readiness or verification. Previously allocated coverage is not reserved again.
Missing/procurement outputs preserve D-155's confirmed/potential/unverified evidence.

Completion-cost estimates may use explicitly entered unit estimates and selected
stored market-price snapshots. Retain estimate source/date, condition/currency
context and version/basis; do not silently choose a “true” price or fetch live data.
Missing prices leave an incomplete estimate, not zero cost. No hidden currency
conversion or unsupported comparison. Planning estimates never alter acquisition
cost, source-price history or the separately pending child-cost allocation rules.

## Acceptance outline

- Create/import an unbuilt MOC with authored-by attribution and v1 BOM. A project
  pinned to v1 changes no stock/copy ownership by being created.
- Build the same design twice: separate project UUIDs, claims and progress, shared
  immutable version. Competing claims cannot double-use the same holdings.
- Reserve/allocate existing loose stock directly to a layout/MOC/custom project
  without generating a fake Set Copy. Link an official-set project to a real copy
  without double allocation/reservation or doubled requirement totals.
- Import v2: v1 and existing project selection/history remain intact. Reviewed
  project version switch explains requirement/claim changes; stale/conflicting
  changes fail without partial mutation or erased verification.
- Layout track analysis distinguishes required part variants and unknown coverage;
  no geometric output, simulation or unsupported inference from attachment alone.
- Estimate a known priced requirement and an unpriced one: preserve the known
  subtotal and explicit unknown remainder; acquisition values remain unchanged.
  A selected market observation remains attributed, not a live valuation promise.
- Version-aware coverage/missing lists and offline exports retain UUID references,
  authorship/provenance, current claims, verification context and exact quantities.
- Native exchange, snapshots, replay and recovery retain project types, pinned
  versions, source files, claims and estimate source/context without altering stock.

## Dependencies and exclusions

[Integrations](offline-import-export-and-catalog-enrichment.md),
[build planning](build-planning-reservations-and-units.md),
[stock allocations](stock-holdings-and-set-allocations.md),
[lifecycle](set-copy-lifecycle-proposal.md),
[cost provenance](cost-provenance-allocation-and-landed-cost.md).
D-181–D-186 record scope and clarifications. The earlier cost questions are resolved by D-187–D-190; child-cost
implementation remains pending. CAD authoring, rendering, instruction generation,
per-piece tracking, simulations, live synchronization and marketplace ordering
remain excluded. No further project product clarification is currently required.
