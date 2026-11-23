# Brickxinator FAQ

Updated 2026-11-23. Current evidence and limitations: [ALPHA_STATUS](../ALPHA_STATUS.md).

## Can I run it now?

Yes, as a source review candidate on `alpha-foundation`, or the local Linux
Docker preview. See [INSTALL](../INSTALL.md). `0.1.0-alpha.1` is not a published
release/tag. Kubuntu 26.04, Firefox/Chromium and one local user are the target.

## Does it need internet?

Core collection, planning, financial, cost, files and recovery workflows work
locally. Imports/exports use reviewed files and local caches. External image
downloads, live synchronization, ordering and online retrieval are not implemented.

## What if quantities or prices are unknown?

Keep them unknown. Unknown is not zero. Count/weight are independent and no
conversion is inferred. Unknown copy counts create no invented identities.
Expected definitions/BOMs do not materialize owned parts.

## Does receipt or warranty approval create all the expected parts?

No. Actual receipts introduce explicitly received holdings/copies. A set receipt
creates copies, not advertised detailed contents. Warranty status alone creates
nothing; actual replacement receipts use new identities for new physical copies.

## Can I receive part of an order or return only some items?

Yes. Orders & warranty supports partial receipts, manual shipments/backorders,
reviewed reversal and supplier return planning/dispatch/acknowledgment/correction.
Only dispatch removes returned stock; acknowledgment/refunds do not repeat it.
Cross-purchase merged returns need explicit known acquisition-source quantities.
Unsafe/stale corrections block rather than silently cancel claims or invent stock.

## Is a loose set automatically available for another project?

No. Allocation/assignment reserves stock regardless of assembly/location.
Verification is separate from planning coverage. Legacy verified set contents
must be explicitly detached for independent loose use; quantity project claims
must be released/transferred explicitly. Some old/new completeness integration
remains acceptance work; do not treat an unverified gap as confirmed missing.

## Can I split stock or keep parts after selling a copy?

Yes. Reviewed splits, compatible merges, detachment and explicit transfer/keep/
custom lifecycle disposition retain history. Copy UUIDs survive Sold/Donated/
Lost/Stolen/Destroyed/Void/Returned states. Archiving only hides clutter.
Ambiguous source quantities can remain unknown and block later returns.

## How do discounts, gift cards and costs differ?

Original paid invoice-line values remain historical facts. Enter gross amounts
separately before new discounts/coupons. Gift cards/vouchers are payments, not
another price reduction. Refunds are separate from physical returns. Financial
line allocation uses four automatic exact-cent modes; shipping uses seven modes,
including Volume, with no missing-input fallback. Tax is informational.

Derived Manual/Piece Count costs can target real holdings and active copies from
the acquisition. They share a source/component budget. Residual and disposed cost
stay explicit; source changes require reviewed reallocation. Rollups show existing
costs, not extra basis; estimates/market observations and reservations are separate.
Unassigned costs are not zero, and copy costing bases do not verify physical parts.

## Can I import Studio/Rebrickable data or use Obsidian?

Studio LDraw/MPD and CSV/TSV exports satisfy alpha; native `.io` is deferred.
Scoped, decompressed text imports are limited to 4 MiB. Map BOM references explicitly.
Reviewed Rebrickable catalog/color/theme files preserve local UUIDs/unselected
fields. Local images remain managed offline. Wanted List XML requires confirmed
BrickLink IDs. Markdown export is available; Obsidian import/live sync is deferred.

## How do backups work?

Destinations start disabled. Daily success requires a validated ZIP plus external
Restic protection/check/retention; keep 30 daily points and 12 independent monthly
ZIPs. Review/activate destinations and scheduling explicitly. NAS/external backup
storage does not authorize a live NAS database. See [INSTALL](../INSTALL.md).

## What remains outside this candidate?

See [ROADMAP](ROADMAP.md) for acceptance work and post-alpha boundaries. There is
no CAD/geometry editor, per-piece assembly, marketplace ordering, required cloud
service, multi-user mode or packaged installer. A desktop review/release approval
and reporting contacts remain gates.

## Where do I report problems?

[Public source/ordinary bugs](https://github.com/richardnoragon/brickxinator).
Private security reporting is explicitly deferred until before release. Never
post private collection data or sensitive vulnerability details in public issues.
