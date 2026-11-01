# Brickxinator testing plan

Current evidence, 2026-11-01: 140 Rust workspace tests, rustfmt and clippy pass.
Three opt-in real-process/Restic drills pass separately. Eight headless scenarios
cover Chromium and Firefox, including purchase receipt/files, image-paste event
handling, exact rates, archives, set verification/recount, lot classification,
reviewed corrections, native exchange and responsive layouts. Frontend formatting,
type checking and production build pass. Tests use disposable data and block
browser requests outside loopback.

Recovery checks include journal/attachment tampering, stale revisions, transaction
rollback, migration failure, interrupted restore stages and safety undo. Restic
0.19.1 performed 31 real runs; all 30 retained daily and 12 monthly points were
restored and checked. Source commands are in [INSTALL](../INSTALL.md); detailed
results and advisory findings are in [ALPHA_STATUS](../ALPHA_STATUS.md).

The host was Ubuntu 26.04.1; Kubuntu desktop/default-browser/native clipboard
acceptance and Richard's release approval remain open. The cases below retain
the original discovery acceptance plan. Their historical “not implemented”
wording does not supersede the current completion audit. Deferred scope, including
whole-set disposal, is not an alpha acceptance requirement.

## Inventory integrity

- Assembled set contents contribute to owned totals and are unavailable. Switching to loose preserves set-copy links and makes contents available without duplication.
- A set copy and all linked contents retain one storage location even when loose. Alpha does not allow individual relocation of linked contents.
- Unknown detailed set contents retain a separately labeled stated piece count; no fabricated part inventory.
- Optional piece count and g/kg weight describe the same stock. Their totals remain separate; unidentified mixed-lot weight is separate from identified-part totals. No automatic piece/weight conversion.
- Incomplete part identities remain separate until manually linked; unknown-brand parts are not automatically merged with branded stock.

## Purchasing and allocation

- Purchase Level mode records shipping once in totals, with no item shipping share.
- Equal mode splits per row regardless of brick count: two rows containing 100 and 500 pieces receive EUR 5 each from EUR 10 shipping.
- Price-proportional mode allocates EUR 10 shipping as EUR 2/EUR 8 for rows costing EUR 20/EUR 80.
- Weight and piece-count modes use their respective known bases; missing inputs or all-zero bases leave allocation incomplete.
- Largest-remainder rounding allocates EUR 1 across three equal rows as EUR 0.34/0.33/0.33 in stable order. Shares sum exactly to shipping.
- Switching methods leaves paid row prices unchanged.
- Manual shares EUR 3, EUR 4 and unknown against EUR 10 shipping save as a draft with EUR 3 remaining; allocated effective costs stay unavailable and shares are not silently changed.
- Unknown shipping differs from EUR 0 shipping. Missing prices mark totals incomplete without blocking saving.
- Paid row totals of EUR 93 and shipping EUR 6.99 yield EUR 99.99; informational included tax is not added again. Added Separately tax status also does not alter authoritative paid prices.
- EUR 60 for 5 kg and 3500 pieces yields EUR 12/kg and approximately EUR 0.017142857/piece, with user-selectable display precision. Unknown/zero denominators produce no rate.

## Drafts and corrections

- A blank draft can be saved with a generated identity.
- Missing values are accepted. Negative quantities/weights and malformed numbers require correction or clearing.
- Correcting quantity from 100 to 120 preserves previous value and correction timestamp.
- Per-item purchase history exposes historical prices across purchases, without requiring external data.

## Remaining work

Define stock lifecycle, edits/deletion, import/file handling, backup/restore verification, UI/browser coverage and release acceptance gates. Automated test implementation will follow the approved architecture; no test run is claimed during discovery.

## Core workflow additions — not yet executed

- Draft, Ordered and Shipped purchases do not increase inventory. Explicit receipt adds stock; repeated receipt must not duplicate it. Partial receipts and reversal are deferred.
- Current stock adjustments preserve original purchased quantities and provenance; damage retains ownership; Lost/Donated/Sold reduce ownership.
- For a known 3500-piece mixed lot, decomposition into 2500 identified pieces leaves 1000 unclassified, totaling 3500 rather than 6000. Weight-only identification invalidates unmeasured current residual weight while preserving historical acquisition weight.
- Renaming a set-copy label preserves its internal identity and links.
- End-to-end alpha workflows: record a purchase with images and unknown fields; locate stock by number/location; toggle assembly without duplication; gradually identify a mixed lot without duplication (D-064).

## Round-8 conservation examples

- Starting with a received 5 kg lot, extract 100 unweighed tracks: preserve historical 5 kg purchase weight, mark current remaining weight unknown, and do not add historical 5 kg to child weights in current totals.
- Identified child batches inherit the lot location, then move whole without changing quantities or provenance. Set-linked parts still cannot move independently.
- Preserve EUR 80 purchase cost once after decomposition; child costs may remain unknown. Do not duplicate the cost across children.
- From 100 owned tracks with no assembly restriction, marking 2 broken leaves 100 owned and 98 available. Donating 5 usable tracks then leaves 95 owned, 93 available, 2 broken.
- Attempting to remove 100 from known stock of 50 is rejected. Unknown stock requires clarification before reduction.
- Correction creates a new revision while retaining previous values. Partial receipt and receipt reversal are unavailable in alpha.

## Operation and recovery requirements awaiting detailed cases

- Verify core workflows in Firefox and Chromium on Kubuntu 26.04 LTS.
- Confirm loopback-only binding and external links opening only on explicit action.
- Verify service stops after 90 seconds without connected app tabs, refresh/reconnect cancels shutdown, unrelated browser tabs do not keep the app running, and crash expires liveness.
- Daily incremental and first-of-month backup jobs must both occur on the first; one must not substitute for the other. Verify 30 daily points plus dependencies, 12 independent monthly snapshots, 03:00 configurable local schedule and one current catch-up per due schedule.
- Managed image copies preserve original files; backups cover images and PDFs with the database.
- Archived records retain attachments and can be restored; eligible empty drafts can be permanently deleted.
- Damage remains independent of assembly: disassembling a set does not restore damaged quantities to availability, and overlapping states are not subtracted twice.

## Definition and verification requirements

- Creating a part/set definition does not create owned stock.
- Preserve one EUR 300 invoice price for three physical set copies; individual prices remain unknown and no EUR 100 allocation is invented.
- Copying set contents creates only an unverified checklist, with zero new actual part stock. Receipt creates the copy only; gradual verification creates actual contents once, using absolute totals and audited corrections.
- Condition New/Used/Mixed is independent of damage and supports unknown.
- Unknown damaged weight prevents an invented usable-weight figure.
- Mixed-lot classification requires explicit completion, not inference from weight/count.

## Round-13 workflow cases

- Receiving a set creates its owned copy but no detailed owned parts. Expected checklist changes do not change stock.
- Expected 10, verified 7 before completion is not automatically “3 missing”; explicit verification completion is required before reporting a local shortfall.
- Verified-count correction preserves timestamp, reason, local user identity, previous and new values; unrelated stock is unaffected. Existing-stock reassignment is unavailable in alpha.
- Known lot remainder of 100 pieces blocks completion. Unknown residue can be confirmed physically empty with confirming user/date; preserve unknown measurement rather than fabricating a measured zero.
- Renaming a manufacturer preserves all references. Archived vocabulary values remain on historical records but cannot be newly selected. Unknown appears as one system-managed category.

## Round-14 accepted examples

- A received already-built set can be Assembled and verification Not Started without fabricated detailed part quantities.
- Finish Expected 10 / Verified 7 to report Short 3; unfinished state never labels that difference missing. Expected 10 / Verified 12 may finish with Excess 2.
- Submit absolute verified 7 twice: actual stays 7. Correct to 9: actual becomes 9, audit records +2, old/new values, local user, timestamp and mandatory reason.
- Equal shipping across a line with three sets and a second line sees two targets, not four. Piece mode cannot use the three-copy count as three bricks.
- System Unknown values reject rename/archive/delete.
- Confirm unknown residual physically empty: current residue excluded while history stays unknown. Audited reopen retains the previous completion and new reason/date without inventing quantities.

## Final state and release gates

- Partially Assembled and Assembled have identical whole-set availability effect; no per-part allocation. Loose releases usable parts, never repairs damage.
- Every checklist line can be finished without automatically completing overall verification. Explicit Complete Verification is required after all active lines are finished.
- Changing expected/verified counts visibly reopens affected line and overall verification, retaining history; expected edits alone never alter stock.
- Confirm every retained daily point restores with its dependencies and every monthly snapshot restores independently without the daily engine. Validate database, attachments and reviewed configuration together.
- Exercise failed/unavailable destination, absent mount, interrupted snapshot/restore and failed migration with recovery; inventory use is not blocked by backup-target failure.
- Source-build reproducibility, Rust formatting/clippy, TypeScript checks, relevant behavior tests, Firefox/Chromium, offline operation, core workflows, audit/corrections, exact costing, backup restore and migration recovery are mandatory before 0.1.0-alpha.1.

No tests have been run because no application has been implemented. Exact commands and evidence records are added with source tooling. Find-inventory acceptance remains required alongside purchase receipt, set state/verification and gradual lot identification.

## Pre-code technical freeze acceptance cases

- Rebuild all current owned/damaged/classification/availability projections from the inventory operation/movement journal; compare results before/after dropping rebuildable caches. Purchase metadata edits alone never increase inventory.
- Unknown-count/known-weight receipt and unweighed extraction preserve unknowns. SET_MEASURED events are not summed as DELTA. Physically-empty closure does not fabricate measured zero.
- Damage retains owned quantities. Assembly replay never double-subtracts damage or applies both a cached availability delta and the same assembly exclusion again.
- Verify every connection has foreign keys enabled, WAL mode is set, FTS5 is available and runtime includes the upstream WAL-reset fix. Test bounded busy handling with snapshot process and UI activity.
- Fail on changed historical migration checksum. Test fresh database, ordered upgrades, interrupted migration and validated recovery.
- Confirm UUIDv7 entity IDs round-trip JSON/files and remain stable through rename/archive; integer FTS rowids and local journal ordering never replace entity identity.
- Rebuild FTS; compare exact identifier search, text search, archive visibility and relational filters. Test pagination/escaped queries and synthetic 50k/100k record growth without claiming unmeasured capacity.
- Snapshot a database with committed WAL changes and managed images/PDFs while app is active; all references/hash sizes agree. Headless snapshot works with service stopped.
- Reject corrupted/incomplete/path-traversing ZIPs, unsupported schemas and failed validation before overwriting current data. Prove safety backup recovers from interrupted restore.
- Daily full ZIP export plus external incremental backup is required. ZIP-only success, absent adapter, failed external exit and unavailable target all remain visibly incomplete/failed daily protection. Verify 30 external daily recovery points plus dependencies and independent 12 monthly full ZIPs.
- Restore actual daily external recovery and monthly ZIP without daily tool. Measure full compressed ZIP impact on incremental storage; do not infer deduplication from tool name.
- REST duplicate request returns prior stock result; stale-tab writes/conflicting idempotency payloads are rejected. Same-user tabs never silently overwrite stock.
- Native JSON round-trip preserves journal ordering/unknowns/identities/provenance; conflicting UUID import is rejected, not merged by assumption. CSV is explicitly a scoped human export, not a restore.

These remain unexecuted requirements. Documentation validation is not application test evidence.
