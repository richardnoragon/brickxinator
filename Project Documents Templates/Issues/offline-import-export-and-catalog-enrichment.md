# Alpha scope: offline imports/exports, catalog enrichment and price snapshots

> Status reconciliation, 2026-11-23: accepted requirements and implementation-time notes below are preserved. Current schema-18 implementation, evidence and remaining limits are in [ALPHA_STATUS](../../ALPHA_STATUS.md), [ROADMAP](../ROADMAP.md) and [ADR-023](../ADRs/ADR-023-expanded-alpha-ledgers-and-enrichment.md); old “pending/deferred” text is not the current feature checklist.

Status: accepted current alpha requirements on 2026-11-22, explicitly revisable
following testing. Implementation and validation pending. Local issue draft;
GitHub publication was blocked by HTTP 401 at the last attempt. No service,
format adapter or library is claimed implemented by this scope decision.

## Governing rule

External systems enrich Brickxinator; they are never authoritative for local
ownership, receipts, stock, allocations/reservations, lifecycle or acquisition
cost history. Core workflows and previously imported/cached data work offline.
Network access is explicit and optional, never required at startup or to open
local records. No background sync silently writes domain records.

Integration tiers: (1) file import/export, (2) catalog enrichment, (3) market/price
information, (4) live synchronization. Alpha focuses on tiers 1/2 plus local
price-snapshot storage. Tier 4 remains excluded. Current acceptance can evolve
through later dated decisions/test evidence without rewriting earlier history.

## Obsidian: Markdown export first

Export collection/set pages, purchase history, build notes and missing-parts
planning to an ordinary Markdown folder structure, e.g. Sets/, Purchases/,
Builds/, Missing Parts/. It must be usable without an Obsidian plugin/account.
Preserve confirmed/potential/unverified distinctions in planning exports.

Obsidian import is a later priority, not an alpha inclusion from this request.
No bidirectional synchronization, file watcher or application plugin. Export to
an explicit destination with safe names and a reviewable overwrite policy; do
not overwrite unrelated vault files. Stable UUID frontmatter was suggested as a
long-term enhancement, not an independently required alpha synchronization model.

## LDraw and Studio: source model -> BOM -> build plan

Alpha imports LDraw .ldr/.mpd model files to extract a BOM and create/update a
project/build plan. Support Studio model/parts-inventory import and reviewed BOM
updates. Import is not a live editor link or an automatic Studio synchronization.

Imported requirements are not physical stock. Creating a project, matching a
part or updating a BOM never receives a purchase, creates a copy or claims
ownership. Existing stock can be reserved/allocated only through explicit stock
workflows. Plans may remain unowned under D-154.

Resolve nested model references deterministically, bound recursion and file
expansion, and report unsupported/ambiguous/missing part/color references rather
than dropping them or guessing replacements. Retain source file/version/hash,
namespace identifiers and mapping decisions. Re-import previews added/changed/
removed requirements and detects stale/conflicting local edits. It must not
reset progress, physical verification, reservations or allocations silently.

No geometry validation, rendering/CAD authoring, instruction generation/parsing,
per-piece assembly state, per-unit part checklist or Studio plugin is implied.
MOC BOM/project import is included; MOC authoring remains excluded.

## Rebrickable: highest-priority enrichment source

One-way reviewed import of catalog definitions, colors/themes, permitted images,
official-set inventories and BOM requirements. Explicit user-initiated refresh
and local caching; no automatic user-profile or collection synchronization.
Catalog/BOM data remains usable when offline or a refresh fails.

External IDs are namespaced references, not replacements for local UUIDs. Preview
new/matched/conflicting records and apply accepted catalog changes through audited
local commands. Preserve local overrides and review changed/disappearing external
identifiers rather than deleting local records or orphaning references. Importing
an official inventory creates expectations, never actual owned parts.

Record source, retrieval time/version, attribution and applicable usage constraints.
Validate data/image rights and API/file contracts before selecting adapters;
provider data rights are distinct from a helper library's software license.
Cache approved assets locally; core views must not depend on remote image URLs.
Credentials, if needed for explicit requests, are not written to exported catalog
files, collection archives, Markdown pages or ordinary logs.

The supplied libraries/projects are background examples, not selected runtime
dependencies or proof of every provider/format capability. Keep the accepted
Rust/local architecture unless a separate implementation decision changes it.

## BrickLink procurement exchange

Include BrickLink Wanted List export, in addition to general downloadable lists.
Parts-list exchange is a high-priority direction; no live purchasing workflow is
implied. External part/color identifiers must be confirmed for the target format;
report unmapped/ambiguous items instead of silently producing wrong order lines.
Preserve quantities and distinguish verified shortages from planning gaps.

No login/cart management, order placement, store comparison, price optimization
or live procurement synchronization. Export works from local data/mappings.

## Market price snapshots

Store independent observations with source, observation date, value and condition;
also retain target identity, currency and sufficient basis/context to distinguish
sealed/used, average/asking/sold observations where supplied. Unknown context is
not an invented comparable sample. Preserve successive snapshots/history.

Do not calculate one authoritative “true value,” overwrite paid acquisition
prices, silently convert currencies or use market observations to allocate source
cost. Market-value child-cost allocation remains deferred under D-169. Existing
EUR acquisition arithmetic is not changed by storing an external observation.

Snapshot storage/manual or reviewed file entry is included; a live market-feed
adapter, provider API access or historical scraping is not implicitly approved.
Any future source-specific retrieval must be explicit, permitted and reviewed.
No real-time feeds or automatic price refresh.

## Integration priorities and exclusions

High: Rebrickable enrichment; LDraw/Studio BOM imports; BrickLink procurement
exchange. Markdown export is included. Medium references: Brick Owl, Peeron and
Brickset; these are not committed alpha adapters. Social-platform integrations
are low priority and outside this alpha inclusion list.

Exclude live synchronization, collection/profile sync, marketplace transactions,
real-time price feeds, Studio/Obsidian plugins and cloud-required operation.
No automatic ownership import or destructive merge of complete collections is
introduced. Existing native fresh-collection import/no-op semantics stay intact;
reviewed catalog/BOM enrichment is a distinct, narrower import contract.

## Acceptance outline

- Import model -> review mapping/BOM -> apply requirements -> create an unowned
  build plan: owned stock, acquisition values and lifecycle remain unchanged.
- Re-import changed BOM: preview differences and affected claims/verification;
  reject stale application rather than silently resetting history or allocations.
- Nested/unresolved files, malformed archives, ambiguous part/color mappings and
  duplicate imports have bounded, explained outcomes; no path escape or dropped
  quantities hidden behind a success result.
- Refresh external catalog explicitly, preserve local overrides/UUIDs and review
  differences. Offline/retrieval failure leaves cached/local collection usable.
- Markdown exports contain meaningful local pages and evidence-aware missing lists,
  with no unapproved overwrite of unrelated files or dependence on an Obsidian app.
- BrickLink Wanted List export preserves mapped identities/quantities and exposes
  unmapped items. No network login or order request is performed.
- Two conflicting price sources/dates coexist without rewriting paid cost or
  inventing one true value. Snapshot import changes no stock/cost allocation.
- Native exchange/snapshots/restore preserve imported provenance, confirmed
  mappings, local requirements and price history without exporting credentials.

## References

User-provided background inspected on 2026-11-22:
- https://hbmartin.github.io/rebrickable/ldraw-bridge/
- https://github.com/scubar/brickgen
- https://pypi.org/project/rebrickable/
- https://github.com/cgans/brickpipe

These describe their respective projects; implementation must validate official
provider/format contracts and rights rather than assume general compatibility.
Related: build-planning, stock-allocation, purchase and cost-provenance issues.
The cost product questions were resolved by D-187–D-190; child-cost implementation remains pending.


## Implementation checkpoint — 2026-11-23

Local text adapters/cache and reviewed part/set catalog enrichment, BOM versions,
Wanted List XML with saved explicit mappings, and combined collection Markdown
are implemented. D-192 confirms Studio exports satisfy alpha; native `.io` is
deferred. Current adapters accept scoped/decompressed files up to 4 MiB. Online
retrieval and color/theme/image enrichment are not implemented. See ALPHA_STATUS
for current validation and remaining acceptance work.

Format contracts checked against primary documentation:
- https://studiohelp.bricklink.com/hc/en-us/articles/6502197862679-Exporting-to-other-formats
- https://rebrickable.com/api/v3/docs/
- https://www.bricklink.com/help.asp?helpID=207


## Color/theme/image checkpoint — 2026-11-23

D-195 is implemented with scoped Rebrickable color/theme CSV import, local color
UUIDs/RGB/transparency, theme hierarchies and stable theme UUID links on catalog
definitions. Per-field review preserves local overrides. Protected Unknown,
stale reviews, invalid RGB, duplicate provider identities and hierarchy conflicts
are guarded. Local image files can be attached to catalog definitions, retained
in managed asset history and served offline. No external image download or online
synchronization is performed. Migration 018 preserves source/override history and
canonical settings prefixes. See ALPHA_STATUS for current validation evidence.
