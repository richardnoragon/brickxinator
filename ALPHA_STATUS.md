# Alpha completion audit

Target: 0.1.0-alpha.1. This is an implementation candidate, not a released tag.
Validation evidence recorded on 2026-11-01 using disposable data.
Scope was expanded on 2026-11-22; the new lifecycle/allocation/build-planning work is not covered by that evidence. Implementation commit:
[`c1551b5`](https://github.com/richardnoragon/brickxinator/commit/c1551b5).

## Historical baseline checklist

The current implementation is described in the latest checkpoint below. These
checkboxes record the earlier baseline and acceptance gates, not the current
implementation status of every expanded feature.

- [x] Rust inventory, purchase, settings, catalog, verification, stock, search and costing foundation.
- [x] Loopback Axum API, Host/Origin/token defenses, local-user audit context and exact large integer reads.
- [x] Svelte/TypeScript workflows, global search and locally built assets.
- [x] Managed image/PDF uploads and image-paste handler; immutable asset hashes and associations.
- [x] OS maintenance coordination, consistent ZIP snapshots, bounded validation, restore and safety undo.
- [x] Ordered checksummed upgrades, pre-upgrade snapshots and injected migration-failure rollback.
- [x] Real Restic daily integration, 30-point retention and independent 12-point monthly ZIP retention.
- [x] All 30 retained daily points restored and validated, with changing stock and a referenced PDF.
- [x] All 12 retained monthly ZIPs restored through the coordinator without Restic.
- [x] Editable multiple destinations, local/drive/share types, exact mount checks, due/catch-up periods and persistent status.
- [x] Opt-in systemd timer installer; generated units validated with systemd-analyze using disposable paths/fake systemctl.
- [x] Purchase date/seller/tax/item metadata, archive/unarchive, eligible draft purge, copy labels and price history.
- [x] Reviewed acquisition corrections preserve children and independent stock observations; discovered copies do not repeat receipt.
- [x] Definition checklists, editable part/color identity, retirement and explicit physical-recount reconciliation.
- [x] Versioned native JSON/assets exchange, conflict/no-op import handling, CSV and browser downloads.
- [x] Source build, Rust formatting/clippy, frontend formatting/type checks and production asset build.
- [x] Dependency license inventory and advisory review: see THIRD_PARTY.md.
- [x] Source branch published and [draft PR #1](https://github.com/richardnoragon/brickxinator/pull/1) prepared.
- [ ] Newly accepted unified project targets, first-class versioned MOCs, non-geometric layout planning and attributed completion estimates (D-181–D-186); implementation and validation pending.
- [ ] Newly accepted Markdown export, LDraw/Studio BOM imports, Rebrickable enrichment/cache, BrickLink Wanted List export and market-price snapshot storage (D-174–D-180); implementation and validation pending.
- [ ] Newly requested child-cost allocation/provenance, proportional split costs, Volume shipping and landed-cost components (D-169–D-173); remaining implementation and validation pending.
- [ ] Newly accepted partial fulfilment, receipt reversals, financial adjustments, returns and warranties (D-159–D-168); implementation and validation pending.
- [ ] Newly accepted build reservations, unowned/future plans, progress-only build units, verified readiness and offline procurement planning (D-150–D-158); implementation and validation pending.
- [ ] Newly accepted stock splitting/logical merging, reservation allocations, independent locations and verified completeness (D-143–D-149); implementation and validation pending.
- [ ] Newly accepted whole-set lifecycle, partial disposition/detached loose holdings, safe audited reversal and definition reassignment (D-136–D-142); implementation and validation pending.
- [ ] Richard's supported-desktop candidate review and release approval.
- [ ] Private security-reporting and community enforcement destinations finalized before release; security reporting is explicitly deferred.

## Latest local implementation — 2026-11-23: copy costs, rollups and enrichment

Implemented in this increment:

- Direct costs on existing active physical copies, alongside holding shares in
  the same source/component allocation. Manual allocation and explicit copy
  piece-count costing bases are supported. Costing bases do not create or verify
  physical pieces. Foreign acquisition targets and overspending are rejected.
- Read-only holding, physical-copy and project rollups. Direct copy and allocated
  part costs are shown separately. Collection reviewed-owned totals count each
  source share once; linked-copy project views never add direct cost again.
  Actual allocated quantities determine project shares, with deterministic
  cent rounding that includes the unallocated holding remainder. Reservations
  and market estimates stay separate. Unassigned/unknown costs stay explicit.
- Terminal copy transitions move direct costs into disposed cost. Safe audited
  corrections restore both direct and linked holding cost effects. A reviewed
  cost-only reallocation can adjust fully refunded/disposed copies without
  inventing active targets or stock. Historical allocations remain intact.
- Reviewed Rebrickable color CSV imports into stable local color UUIDs, with RGB
  and transparency metadata and source identity. Existing local names/RGB/
  transparency change only when selected; protected Unknown remains protected.
- Reviewed theme CSV hierarchies with stable UUIDs, local display-name overrides,
  parent resolution and source IDs. Duplicate source identities, missing parents,
  cycles and stale reviews are rejected atomically. Catalog definitions can
  explicitly link imported theme UUIDs while retaining legacy free-text themes.
- Explicit local catalog image attachments using managed content-hash storage,
  preserved by snapshots/native exchange. Existing images are retained. No
  external image URL is fetched.
- Migration 018 adds immutable enrichment/source history, including canonical
  settings prefixes and reviewed field choices. Migrations 001–017 are unchanged.

Validation: 184 Rust tests pass, including shared-budget/copy lifecycle/rollup
checks, local override and hierarchy checks, native restoration and theme UUID
links. Rust formatting/clippy, frontend formatting/type checks and production
builds pass. All 18 Chromium/Firefox scenarios pass. Browser acceptance covers direct copy assignment, rollups, color/
parent-child theme imports, catalog image upload and unchanged stock.

Operating boundaries: direct copy targets must already exist and belong to the
selected acquisition source. Rollups show reviewed acquisition cost, not market
valuation; missing/unreviewed cost remains visible. Catalog inputs remain scoped,
decompressed text files up to 4 MiB. Online catalog retrieval/image downloading,
live synchronization and Studio native `.io` remain out of scope.

The local Docker app is running schema 18. Its integrity doctor and new rollup/
enrichment endpoints pass, and deployed binary/assets hashes match the tested build.

Safety snapshot before deployment: `/data/before-copy-cost-rollups-enrichment-20261123.zip`
in the Docker data volume (schema 17). The earlier checkpoints below are historical
and their pending labels are superseded for the features implemented here.

## Earlier local implementation — 2026-11-23: finance, returns, costs and interchange

Implemented and regression checked in this increment:

- Separate audited gross amounts and financial events for discounts, coupons,
  loyalty reductions, payments and refunds. Recorded paid line totals remain
  unchanged. Discounts/refunds allocate exact cents automatically by Equal,
  Price Proportional, Piece Count or Weight; missing basis inputs block rather
  than switch methods. Gift cards/vouchers settle payment without reducing cost
  again. Correcting the latest active entry retains its original record; later
  dependent entries must be corrected first. Refunds may link claims/returns.
- Supplier return plans, actual physical dispatch, supplier acknowledgment and
  audited dispatch correction. Partial loose-stock quantities and whole-copy
  returns support explicit linked-part disposition. Only dispatch removes stock;
  acknowledgment/refund does not remove it again. Returned identities remain;
  warranty receipts create new physical identities. Later changes/claims block
  unsafe inverses. Cross-purchase merged returns require explicit source quantities
  (D-193), and consume only their selected acquisition quantity and cost layers.
- A separate child-holding cost allocation ledger, with Manual/Piece Count, item/
  shipping/purchase-fee components, exact shares, explicit residual source balances,
  preserved original prices, proportional split costs and separate layers through
  merges. New reviewed allocations supersede previous effective results without
  erasing them. Changes to source component amounts flag required reallocation;
  stock movements do not silently rewrite source costs. Reviewed disposed-cost
  overrides support downstream financial corrections while retaining old results.
- Reviewed Rebrickable part/set CSV catalog enrichment, Studio CSV/TSV and
  Rebrickable inventory BOM adapters with explicit columns/inventory/spare policy,
  existing LDraw/MPD extraction, local source caching and manual refresh. Catalog
  refreshes keep UUIDs and unselected local fields. BOMs become version snapshots
  with local part/color mapping; imports create no ownership. Studio native `.io`
  compatibility is deferred by D-192.
- BrickLink Wanted List XML download with explicit external part/color IDs and
  auditable saved mappings. XML escaping and exact quantity aggregation are tested.
  Collection Markdown includes sets, loose holdings, purchase history, project
  notes and evidence-aware gaps; there is no live sync or marketplace transaction.
- Migrations 016–017 add immutable child-cost and cached-import history. Native
  exchange preserves both, and replay validates canonical financial/source inputs,
  cost results, imported hashes and reviewed catalog effects. Migrations 001–015
  remain unchanged.

Validation: 176 Rust tests pass; rustfmt, clippy with warnings denied, frontend
format/type checks, production builds and 16 Chromium/Firefox scenarios pass.
Cost split/merge/source-specific return attribution and cached catalog refreshes
survive replay/native restore. Desktop/mobile browser checks include financial
review, physical return, cost allocation, cached catalog import and Markdown
downloads. The new mobile screens were visually inspected.

Operating limits and remaining expanded-alpha acceptance work:

- Local text adapters accept scoped, decompressed files up to 4 MiB; full bulk
  download streaming, compressed provider files, automatic retrieval, color/theme/
  image enrichment and live synchronization are not implemented. Studio exports
  are supported; native `.io` is explicitly outside alpha.
- Unknown mixed-source quantities after an ambiguous split/disposition remain
  unknown and can block returns. No source quantities are fabricated to unblock
  them. A dedicated provenance reconciliation editor remains acceptance work.
- Cost targets are actual holdings. Direct physical-copy cost assignment and
  set/project cost rollups, broader allocation/correction acceptance, and richer
  per-entity Obsidian vault packaging remain follow-up work.
- Independent progress/verification versus old set contents, audited definition
  reassignment, project priority/footprint refinements and supported-desktop review
  remain open. This increment does not declare the entire expanded alpha released.

Docker deployment: the local app at http://127.0.0.1:4217 was upgraded to schema
17. Its database/journal/metadata/attachment doctor passes. The prior schema-15
collection remains available in the named pre-upgrade snapshot below.

Safety snapshot before deployment: `/data/before-finance-cost-imports-20261123.zip`
in the Docker data volume. The earlier dated sections below are historical
implementation checkpoints; their pending items are superseded where described
above.

## Earlier local increment — 2026-11-23: physical/fulfilment foundation

The local Docker app now includes the physical and fulfilment increment. This is
still a partial implementation of the expanded alpha, not release completion.
Migrations 014–015 preserve immutable movement/source-layer and fulfilment history;
previous migration checksums remain unchanged.

Implemented:

- Reviewed physical splits (including partial quantities), compatible loose-stock
  merges with historical source links, and detachment of set contents into loose
  holdings. Unknown dimensions remain unknown; counts are never inferred from mass.
- Whole-copy Sold/Donated/Lost/Stolen/Destroyed/Void/Returned lifecycle events with
  explicit Transfer All, Keep All or custom partial retention. Copy UUIDs survive.
  Safe audited corrections restore original stock effects; later stock, profile
  changes and active claims block unsafe corrections. Project claims must be
  explicitly released/transferred before incompatible disposal or stock movement.
- Separate order fulfilment screen: manual shipment/tracking events, actual partial
  receipts, outstanding quantities, backorders, closure/reopening, and reviewed
  receipt reversal/correction. Unreceived intent creates no stock; unknown copy
  counts create no fabricated identities. Fully delivered orders stop appearing
  as pending deliveries in the dashboard.
- Independent warranty claim states and linked actual replacement receipts,
  including against closed orders. Claim status does not create stock or refunds.
- Native export/import preserves physical and fulfilment histories, with replay
  checks against canonical purchase snapshots and stock/profile history prefixes.

Validation: 164 Rust tests pass, along with rustfmt, clippy with warnings denied,
frontend type checking, production builds, and all 12 Chromium/Firefox scenarios.
The new browser scenario verifies receipt preview is read-only, partial receipt,
audited reversal, reload and a 390-pixel layout. The mobile screenshot was inspected.
Docker is healthy at http://127.0.0.1:4217 and its database/journal/metadata/attachment
integrity doctor passes. A pre-upgrade snapshot is retained inside its data volume
at `/data/before-physical-fulfilment-20261123.zip` (schema 13, sequence 0).

Remaining acceptance work:

- Purchase-linked planned/dispatched supplier returns, loose-stock return handling,
  complete return/warranty financial outcomes, payments, coupons, discounts and
  automatically attributed refunds. Whole-set Returned currently uses the explicit
  lifecycle workflow. Gross-versus-already-paid line amount clarification is pending
  before discount calculation to avoid double deduction.
- Audited child-cost allocation/reallocation, proportional cost layers on splits,
  and reviewed downstream reallocation after source financial changes. The source
  graph is not yet a cost allocation ledger.
- Studio native/parts-list and Rebrickable adapters/cache, BrickLink Wanted List
  export, full collection/purchase Markdown export, managed model source attachments.
- Unified handling of existing set verification contents versus new project
  allocations, audited definition reassignment, project priority/footprint
  refinements, and wider end-to-end lifecycle/return acceptance coverage.

The earlier dated increment and checklist below are historical evidence. Their
pending labels must not be read as completion of the remaining work above.

## Local implementation increment — 2026-11-22

This increment implements part of the expanded scope; it does not complete the
new alpha requirements. Migration 013 adds immutable project-command history and
extends the shipping projection for Volume without editing migrations 001–012.

Implemented and regression checked:

- Projects for official builds/MOCs/layouts/dioramas/custom work, including unowned
  plans and independent repeated executions sharing immutable BOM versions.
- Manual requirements, pinned version switching after releasing claims, direct
  loose-stock reservations/allocations, partial claim transfer and release,
  explicit verification and Start Build conversion. No ownership creation.
- Global/per-project confirmed missing versus potential/unverified coverage,
  readiness, progress-only bags/subassemblies and recent project audit history.
- Attributed manual price observations and selection of stored EUR observations
  for estimates. Unknown estimates stay unknown and acquisition prices stay intact.
- Bounded LDraw/MPD BOM preview with explicit local part/color mapping; project
  procurement CSV and Markdown downloads. No geometry or external service access.
- Volume shipping (new app/API purchase default), explicit total packaged line
  volumes, separate fee entry/recorded totals and saved-method preservation.
- Inventory/dashboard reservation-aware availability; disassembly does not release
  assigned stock. Stock/damage/identity changes cannot invalidate active claims.
- Native exchange/restore preserves project history; strict replay checks stock
  and profile history prefixes and current references. A snapshot of the Docker
  collection was created before upgrade (`before-alpha-expansion-20261122.zip`).

Validation: 153 Rust tests, rustfmt, clippy, frontend type checking and release/
asset builds pass. All ten Chromium/Firefox scenarios pass, including project reservation/verification/
Start Build, download/reload and 390-pixel layout checks. The rebuilt Docker
container is healthy; its database/journal/metadata/attachment doctor passes and
the new Projects endpoint returns HTTP 200.

Still pending: whole-set lifecycle/disposition/reversal, physical stock split and
logical source-layer merge, independent relocation of existing set contents,
complete partial purchase/return/warranty/financial workflows, child cost allocation
and reallocation, Studio/Rebrickable adapters, BrickLink Wanted List export,
full collection/purchase Markdown export, managed model source-file attachments,
project priority/footprint refinements and further acceptance coverage. Existing
set contents remain protected from direct independent edits until those physical
migration/disposition workflows are implemented.

The cost-policy questions are resolved (D-187–D-190). All expanded areas above
remain open acceptance work; do not mark alpha complete from this increment.

## Executed evidence

Build environment: Ubuntu 26.04.1 LTS, Rust 1.98.1, Node 22.22.1, npm 9.2.0.
The supported product target remains Kubuntu 26.04 with Firefox/Chromium; a real
Kubuntu desktop/launcher/OS-clipboard acceptance check remains part of Richard's
candidate review. Headless browser checks do not claim that desktop sign-off.

- 140 Rust workspace tests pass; three explicit opt-in process/Restic drills pass separately.
- Chromium and Firefox each exercise draft/receipt, PDF upload, simulated image paste,
  acquisition metadata, archives, exact rates, set verification/assembly, copy labels,
  retirement/recount, mixed-lot classification, negative correction prevention,
  native export/identical re-import, and responsive dashboard/recovery. Stale/conflicting
  writes are separately covered by Rust/API regression checks.
- Browser acceptance blocks requests outside loopback. Desktop and 390 px mobile
  screenshots were inspected. Native OS clipboard interaction remains a manual check.
- Real-process checks verify second-launch reuse, idle shutdown and restart after a crash.
  Unit checks cover the configured 90-second grace and heartbeat expiry without waiting 90 seconds.
- Backup/maintenance/export commands refuse missing databases. Startup refuses automatic
  initialization in a nonempty directory whose database has disappeared.
- Restore exercises corruption/traversal/future-format refusal, interrupted file-install
  stages, attachments, audit history, safety undo and inactive restored configuration.
- Restic 0.19.1: 31 actual backup runs with changing stock, retain 30 daily points and 12
  monthly ZIPs; every retained point restored. The run took about 107 seconds.
- Small-fixture storage measurement: 572,273 bytes of retained compressed ZIP input,
  606,883 bytes of repository files (about 6.0% overhead). This is not a capacity or
  deduplication forecast for a real collection.
- RustSec cargo-audit 0.22.2: 137 lockfile packages, zero vulnerabilities/warnings,
  database commit 2295466c4e9df26647fd67d8fa1c705ac2fc7f81.
- npm: zero production advisories; two low build-tool findings, no reported fix available.
  The production source map contains no deepmerge source. See THIRD_PARTY.md.

## Decisions and operational limits

Restic was selected by Richard. Defaults are `<initial working directory>/backups/daily`
and `<initial working directory>/backups/montly` (literal spelling), editable after
installation. Destinations start disabled. No real user timer or backup destination
was activated during development. The timer installer was tested with a fake
systemctl and temporary configuration; only generated unit syntax was validated.

The public repository is https://github.com/richardnoragon/brickxinator.
No release/tag/merge is authorized by these test results. The implementation stays
on a review branch until Richard reviews it.

Whole-set lifecycle was brought into alpha by Richard on 2026-11-22 (D-136–D-142).
It is not implemented yet; explicit part disposition and audited reversal are
required. Retained parts detach into loose holdings; partial disposition and
safe reversal are confirmed. See the lifecycle issue proposal for accepted
requirements and validation cases.
Archiving preserves ownership.
Copy-count corrections cannot remove existing physical copy identities.
Stock splitting/merging, loose-to-set allocation, allocation removal/transfers and
verified completeness were accepted for alpha on 2026-11-22 (D-143–D-149).
Allocations reserve even Loose stock, locations remain independent, physical
verification can remain Unknown, and logical merges preserve source lots.
These are not implemented. The current runtime still enforces its earlier
storage/splitting/assembly-based availability rules. Partial fulfilment/reversal,
discounts/coupons, refunds/returns and warranties are now accepted alpha scope
(D-159–D-168), not implemented. Returns act at handover, replacement copies get
new identities, and financial adjustments require automatic line allocation.
Child-lot cost allocation (Manual/Piece Count), proportional cost provenance,
Volume shipping and landed-cost component tracking were additionally requested
for alpha (D-169–D-173). Product questions are resolved by D-187–D-190; child-cost implementation and
validation remain pending. Market-value cost allocation remains deferred.
Offline-reviewed imports/enrichment and market-price snapshot storage are now
accepted alpha scope (D-174–D-180), pending implementation and validation.
Live synchronization/ordering/feeds remain excluded; Obsidian import stays later.

Build reservations, bag/subassembly progress, per-set/global missing-parts
planning and a build-status workflow were additionally requested for alpha
(D-150–D-158). Plans may target unowned sets; units are progress-only; planning
separates confirmed/potential/unknown needs. Readiness requires verified committed
stock and Start Build explicitly converts reservations to allocations. These are
not implemented. Per-piece assembly, unit checklists, instruction-step tracking
and live marketplace ordering stay excluded.

Unified Official Set Build/MOC/Train Layout/Diorama/Custom Project planning,
versioned design BOMs, direct project allocations, separate repeated executions
and attributed cost estimates are now accepted alpha scope (D-181–D-186).
They are not implemented. CAD geometry, rendering, instruction generation,
route/signal simulation and train scheduling remain excluded.

Native v1 initializes a fresh collection or accepts an identical re-import; implicit
merging is unsupported. It requires the current declared schema. Large collections
and 50k/100k growth performance are unmeasured. Browser editors reject integer
inputs outside JavaScript's exact range while read values remain exact strings.
Snapshot creation holds a maintenance gate through validation; UI requests may
need retrying. Safety/manual export files remain available for user-directed cleanup.

Image/PDF upload limit: 64 MiB. Browser archive request limit: 512 MiB; larger
supported archives use CLI. ZIP validation bounds: 100,000 entries, 4 GiB per
payload, 16 GiB total expanded data, 4 MiB snapshot manifest. Native JSON limit:
256 MiB; native asset files retain the 64 MiB bound.
