# Alpha completion audit

Target: 0.1.0-alpha.1. This is an implementation candidate, not a released tag.
Evidence recorded on 2026-11-01 using disposable data.

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
- [ ] Source branch published and draft review prepared.
- [ ] Richard's supported-desktop candidate review and release approval.
- [ ] Private security-reporting and community enforcement destinations finalized before release; security reporting is explicitly deferred.

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

Whole-set disposal is explicitly deferred by Richard. Archiving preserves ownership.
Copy-count corrections cannot remove existing physical copy identities. Partial
receipt/reversal, returns/aftercare, arbitrary splits, independent movement of
set-linked contents, child-lot cost allocation and market/online imports remain deferred.

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
