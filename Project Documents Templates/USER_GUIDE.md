# Brickxinator alpha user guide

Guide to the alpha review candidate. Source-build and timer setup instructions
are in [INSTALL](../INSTALL.md); exact validation/release status is in
[ALPHA_STATUS](../ALPHA_STATUS.md). UI is English; the supported target is
Kubuntu 26.04 LTS with Firefox or Chromium.

## Start and stop

A launcher starts the loopback-only service and opens your browser. No login or LAN access. Closing the last Brickxinator tab starts a 90-second grace period; reconnect/refresh cancels shutdown. A browser crash also eventually disconnects tabs. Independent backup scheduling continues without the browser service.

## Buy and receive

Create a purchase draft, record seller/type/country, date, order reference, optional paid prices/shipping/tax and notes. Attach images, paste pictures and add PDF invoices/instructions. Original files remain untouched; copies are managed by the app. Notes/attachments/source URLs can belong to purchase or item.

Add invoice lines for sets, loose batches or unidentified mixed lots. One line can fund several separately labeled set copies; keep a combined invoice price without inventing copy prices. Statuses: Draft, Ordered, Shipped, Received, Cancelled. Only explicit whole-purchase receipt adds stock. Receiving a set creates its copy, not phantom detailed parts. Partial receipts/reversal are deferred.

Unknown information is allowed, including otherwise blank drafts with generated IDs. Unknown prices/shipping differ from known zero. Invalid supplied numbers require correction or clearing.

## Find and store

Dashboard: inventory totals, pending deliveries, unidentified lots, incomplete records and recent purchases. Search is available everywhere; priority filters are number, storage subtree and availability. Other filters include manufacturer, description, color, assembly state, incomplete records, purchase date, seller and condition. Archived data remains searchable.

Choose nested storage from a tree. Names are managed in settings with duplicate/similarity warnings, never silently corrected. Manufacturer/color/condition use permanent IDs and one protected Unknown choice. Initial conditions New/Used/Mixed; damage is separate.

**Alpha restriction: one set copy and all linked contents share one location, even loose. Independent batches move whole; no partial batch splitting.** Newly identified mixed-lot children may move independently after creation.

## Verify a set

Copy/create expected contents as an unverified checklist. Expected counts do not create stock. Count this set's actual contents gradually; reassignment of other recorded stock is deferred.

Enter absolute totals: entering 7 twice leaves 7. Correcting to 9 records +2 with reason/timestamp/local user/old/new values. Finish lines explicitly. Expected 10 / counted 7 is 3 unverified until finish, then Short 3. Excess is allowed. Overall Complete Verification is explicit after all lines are finished. Count edits visibly reopen line/overall verification and preserve history; expected edits never change actual stock.

Assembly is independent: Loose, Assembled, Partially Assembled. Either assembled state makes all actual linked parts unavailable. Loose releases usable actual stock; damage remains unavailable. Already-built unverified sets are valid. Stated box piece counts are displayed separately from verified quantities.

## Sort a mixed lot

Original count/weight/cost stay on the purchase history. Identify child batches, preserving source/photos/seller. Transfer known measures without duplicate holdings. If children are unweighed, remaining weight becomes unknown; no automatic piece/weight conversion. Counts and weights are separately reported.

Known nonzero remainder blocks completion. If unknown residue is physically empty, explicitly confirm emptiness, preserving confirmation/date without inventing measured zero. Later findings use audited Reopen Classification. Child costs may remain unknown; internal lot-cost allocation is deferred.

## Adjust and archive

Broken/Damaged stays owned but unavailable. Lost/Donated/Sold reduce ownership. Reject over-removal; unknown stock must be clarified first. Changes preserve audit history; receipt is never repeated to apply a correction. Stock changes need a reason; descriptive edits can have an optional reason.

Archive hides clutter but preserves stock totals, attachments and search. Restore archived records with unarchive. Only never-received drafts without lines or attachments may be erased permanently,
including drafts with entered metadata.

## Costs

Prices are EUR paid line totals; optional tax is informational and never added twice. Shipping belongs to purchase, with six line-target modes: Purchase Level (default, no shares), Equal, Price Proportional, Piece Count, Weight, Manual. Allocation never changes paid prices. Manual drafts retain unbalanced shares and show remainder; effective allocated costs wait for completion. Missing automatic inputs produce incomplete status. Rounding shares reconcile exactly.

Unit rates EUR/piece and EUR/kg derive only from known valid data, with selectable precision. Purchase-price history shows what you paid, not external market values.

## Back up and restore

Ordinary files/directories under configurable XDG data root. Automatic daily incrementals at configurable 03:00 local time, 30 recovery points plus dependencies; separate first-of-month full snapshots, 12 retained. **Monthly never replaces daily.** Missed work catches up once per due schedule. Daily recovery can require standard backup software; monthly snapshots must remain independently accessible.

Multiple local/external/network destinations; unavailable targets show persistent warning and last-success status without blocking collection use. Restore validates backup, creates safety backup, restores coherent data/attachments and validates result. Review old machine paths before reactivating restored destinations. Brickxinator creates a full ZIP snapshot; a mandatory external incremental backup then protects the daily recovery point. ZIP creation alone does not mean daily backup succeeded. Monthly ZIPs restore independently. Restic is the daily adapter; setup and timer commands are in INSTALL.md.

## Current correction and recovery controls

Received purchases use **Review correction** before applying acquisition changes.
The preview shows residual stock effects and added copies. Existing classified
children and independently measured stock remain authoritative; a correction
cannot consume them again. Existing physical copy identities cannot be removed
by reducing a purchase count. Whole-set sale/donation/loss is deferred.

Archiving hides a purchase/copy from default lists while retaining ownership and
history. Permanent draft deletion is allowed only for never-received drafts with
no invoice lines or attachments, even when metadata was entered. Review the
confirmation: entered metadata and its snapshots are purged; minimal tombstones
prevent old requests from recreating the draft.

Set checklists can come from definition templates or another owned copy. Correct
part/color identity, retire/restore lines and rename copies without erasing their
history. Retirement retains actual stock. After a part disposal, use the explicit
fresh physical-count review to reconcile a new measurement; ordinary count entry
will not silently recreate removed parts.

Recovery contains backup destinations, schedule time, persistent status, manual
backup, snapshots and native/CSV exchange. Destinations start disabled and default
to the initial working directory's `backups/daily` and `backups/montly`. Add local,
external-drive or share destinations; external/share destinations require an
exact mounted path. Install the separate user timer as described in INSTALL.
Keep the Restic password independently; it is not included in snapshots.

Native export contains versioned JSON and original managed assets. Import
initializes a fresh collection or accepts an identical re-import; it does not
merge conflicting collections. CSV reports current inventory only. Restore
validates the archive and creates a safety snapshot before replacement; restored
backup configuration stays inactive for review. Use `recover` if interrupted.
