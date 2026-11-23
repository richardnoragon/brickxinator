# Brickxinator user guide

Current candidate: 2026-11-23, schema 18. Start with [INSTALL](../INSTALL.md).
This is a local source/Docker review candidate, not a released installer.

## Start and navigate

The native launcher opens the local browser app; the Docker preview is at
http://127.0.0.1:4217. Pages are Dashboard, Purchases, Inventory, Sets, Catalog,
Projects, Stock & lifecycle, Orders & warranty, Cost allocations, Imports & exports,
Settings and Recovery. Use a single local collection/machine. Native sessions
stop after the idle grace; Docker stays running until stopped.

## Record purchases and genuine receipts

Save a draft with whatever is known: seller/date/order reference, notes, invoice
lines, paid EUR totals, shipping/fees and informational tax. Unknown is not zero.
Set copy count and detailed part count are separate; stated box counts create no
actual stock. Attach original photos/PDFs after saving. Eligible never-received
drafts without lines/attachments may be purged; received history is retained.

Use Orders & warranty for manual shipments/tracking, partial actual receipts,
outstanding quantities, backorders and closure/reopening. Select only what
arrived. Unknown set copy count creates no invented copy identities. A whole-
purchase legacy receipt cannot be used to duplicate explicit partial receipts.
Review a receipt reversal/correction when a receipt was mistaken; later stock
changes, claims or metadata can block an unsafe inverse.

## Manage stock, copies and verification

Catalog definitions/checklists describe products. Link actual holdings/copies to
local definitions, colors, condition and storage in Inventory/Sets. Counts and
weights remain independent. Verification records absolute observations; expected
requirements do not manufacture parts. Unverified does not mean confirmed missing.

In Stock & lifecycle, review splits, compatible merges and detachment of legacy
set contents into loose holdings. Source history survives; unknown transferred
dimensions can make residual measurements unknown. Resolve claims explicitly
before incompatible movements. Assembly, allocation and storage are independent;
changing assembly does not release a reservation.

For a copy's sale/donation/loss/theft/destruction/void/return, select the real copy
and explicitly transfer all, keep all or retain custom linked quantities. Retained
parts become loose. UUID/history remains. Safe corrections reference the terminal
event; later stock/profile/claim changes may require review instead of restoration.
Archiving only controls visibility and never removes ownership.

## Plan and build

Create immutable design/BOM versions and separate execution projects, including
unowned future plans. Projects can be official builds, MOCs, layouts, dioramas or
custom work; they do not edit CAD geometry. Reserve usable loose stock, verify it,
and explicitly Start Build to convert reservations to allocations. Ready requires
verified usable committed supply and no known shortages.

Review confirmed missing separately from potential gaps/unverified supply. Track
bags/subassemblies by progress, not per-piece checklists. Manual estimates and
selected price observations are planning data, not acquisition cost. Export
procurement lists without marketplace login/order placement. Further integration
of legacy set verification and project completeness remains acceptance work.

## Supplier returns and warranties

Create a return plan, select actual stock/copy and review physical handover. Only
dispatch removes ownership; supplier acknowledgment does not remove it again.
For merged holdings, explicitly enter acquisition-source quantities from the
selected order. Unknown/insufficient provenance blocks dispatch; there is no
implicit FIFO. A dedicated provenance reconciliation editor remains follow-up.

Record warranty status separately. Status alone creates no stock or refund.
Link actual replacement receipts, returns and financial events as appropriate.
A physically different replacement receives a new copy UUID; the original remains
Returned. A correction is not a genuine new receipt.

## Financial adjustments

In Orders & warranty, enter an audit reason. Original paid invoice amounts remain
historical facts. Enter separate gross line amounts before new discounts/coupons.
Financial events use dates/labels and reviewed automatic line allocation: Equal,
Price Proportional, Piece Count or Weight. Missing inputs block, with no fallback.
Gift cards/stored-value vouchers are payments, not another discount. Refunds are
separate from physical returns and can link claims/returns. Correct later dependent
financial entries before earlier ones; originals remain in history.

Shipping uses Purchase Level, Equal, Price Proportional, Piece Count, Weight,
Manual or Volume. New purchases default to Volume; supply total packaged line
volume or explicitly choose another method. Missing basis stays incomplete.
Fees are separate and tax information is not added twice.

## Cost allocations and rollups

Choose a purchase/component in Cost allocations, then actual source-derived
holdings, existing active copies, or both. They share one source budget. Enter
manual shares or explicit piece-count costing bases; copy costing bases do not
create or verify stock. Review shares, unallocated source balance and disposed
cost before applying. Original paid values are not overwritten.

New reviewed allocations supersede old effective results while preserving history.
Source financial changes can require reallocation; unknown cost is not zero.
The optional reviewed disposed-cost field supports downstream financial corrections.
A zero-target/zero-allocation correction can handle a fully refunded source whose
stock has left without fabricating active inventory.

Cost rollups show direct copy and actual allocated-part costs separately. Project
views use their allocated quantities; linked copy direct costs remain in the copy
view. Collection reviewed totals count each share once. Reservations and market
estimates stay separate. Reviewed totals can be partial; inspect missing/stale
coverage and source residuals before interpreting them as complete acquisition cost.

## Reviewed files, colors, themes and images

Imports & exports accepts scoped, decompressed text files up to 4 MiB: LDraw/MPD,
Studio-exported CSV/TSV and Rebrickable catalog/inventory/color/theme CSV. Native
Studio .io is deferred. Choose explicit parts-list columns/inventory/spare policy,
map local part/color IDs, and save a BOM version before using it in Projects.

Select catalog/enrichment rows and local targets. Only selected fields change;
UUIDs/unselected overrides survive refresh. Color names can have an explicit local
override; protected Unknown cannot be repurposed. RGB/transparency may remain
Unknown. Themes preserve mapped parents and reject missing parents, duplicate
provider identities and cycles. Import parents in the batch or map/import them
first. Catalog definitions can link stable theme UUIDs alongside free text.

Source files/reviewed choices stay cached offline. Attach local PNG/JPEG/WebP
files to catalog definitions; existing images remain. No external image URL is
fetched. Wanted List XML requires confirmed BrickLink part/color IDs; save mappings
for reuse, review positive quantities and download. Local UUIDs/provider IDs are
not silently substituted. Collection/project Markdown and CSV downloads support
local notes/Obsidian; live synchronization and Obsidian import are deferred.

## Recovery and backup

Use Recovery to review backup destinations, snapshots, native exchange and restore.
Destinations start disabled. Daily protection requires validated ZIP plus external
Restic; keep 30 daily points and 12 independent monthly ZIPs. Configure scheduling
explicitly; the Docker preview has no independent scheduler.

Native import accepts a fresh collection or identical no-op, not a collection
merge. Restore validates and creates undo safety data; interrupted restore requires
recover. Restored machine configuration stays inactive pending review. Never copy
a live SQLite file or put the active database on a filesystem-sync/NAS share.
See INSTALL.md for commands and ALPHA_STATUS.md for validation/release limits.
