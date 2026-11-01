# Brickxinator FAQ

## Can I run it now?

A source-built alpha candidate is available on the `alpha-foundation` review
branch; see [INSTALL](../INSTALL.md). The first intended prerelease is
0.1.0-alpha.1; it has not shipped.

## What platform and language?

Kubuntu 26.04 LTS, Firefox and Chromium, English UI. Local Rust/Axum + SQLite backend and Svelte/TypeScript browser interface. No login, LAN server or Tauri in alpha.

## Does it need internet?

No for core use. Collection search, purchases, set management and managed photos/PDFs must work offline. Online integrations are deferred; clicking an external URL is explicit.

## What if I do not know quantities or prices?

Leave them unknown. Blank cost is different from known zero. Count and weight are independently optional; no automatic piece/weight conversion. Invalid supplied values require correction or clearing. Blank drafts receive internal IDs.

## Does receiving a set add all its expected parts?

No. It creates the owned set copy. Expected contents are an unverified checklist; actual parts count only when verified. Unverified never automatically means missing.

## Can assembled sets remain unverified?

Yes. Assembly, verification and damage are independent. Assembled and Partially Assembled make all linked actual parts unavailable. Loose releases usable actual parts; damaged pieces remain owned but unavailable.

## Can I move a set's loose parts to different drawers?

Not in alpha. A set copy and all linked contents share one location. Independent batches move whole. Identified children of a mixed lot may move independently after identification.

## How are shared purchase prices and shipping handled?

Keep the original invoice-line price. Multiple set copies can share a price without invented per-copy values. Six shipping modes allocate to invoice lines; allocation never overwrites source prices. Tax metadata never adds tax twice. Internal mixed-lot child cost allocation is deferred.

## Are archived records still owned?

Yes. Archiving hides clutter but retains searchability, attachments, history and inventory totals. It is not a stock reduction.

## How do backups work?

Required: automatic daily incrementals, 30 daily points plus dependencies; separate full monthly snapshots on the first, 12 retained. Monthly backups do not replace daily ones. Independent OS scheduling, multiple destinations, warnings for failures and validated restore are required. Restic and the opt-in systemd user timer are implemented and tested with
disposable data; destinations start disabled. See INSTALL.md for activation. Monthly snapshots must remain accessible without specialized backup software.

## What is deliberately outside alpha?

Reservations, per-part assembly, global missing-parts planning, CAD/train/MOC features, online integrations, partial receipt/reversal, return/refund flows, volume shipping allocation, internal child-lot cost allocation and Obsidian import. See ROADMAP.md.

## Where do I report a bug or security issue?

Public source/ordinary bugs: https://github.com/richardnoragon/brickxinator.
Private security reporting is explicitly deferred until before release. No
reporting address has been invented. See SECURITY.md.

## What is the inventory source of truth?

The movement journal, not purchase totals. Purchases record historical acquisition; receipt/verification/adjustment/transfer commands record stock facts. Current balances/search indexes are rebuildable projections. Damage and assembly affect usability without incorrectly reducing ownership.
