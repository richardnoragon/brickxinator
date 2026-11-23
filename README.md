# Brickxinator

Offline brick inventory, collection and build planning for LEGO, BlueBrixx and
compatible systems. GPL-3.0-or-later; see [COPYING](COPYING).

**Status, 2026-11-23:** `0.1.0-alpha.1` source review candidate, schema **18**.
No release or tag has been published. Work is reviewed on `alpha-foundation` in
[draft PR #1](https://github.com/richardnoragon/brickxinator/pull/1).
See [current evidence and limits](ALPHA_STATUS.md), [installation](INSTALL.md),
[the user guide](Project%20Documents%20Templates/USER_GUIDE.md) and
[documentation index](Project%20Documents%20Templates/TOC.md).

## Build and run

Target: Kubuntu 26.04, Firefox/Chromium, one local user and one active machine.
Install Rust 1.89+, Node.js 22.12+, npm, a C compiler and Linux build tools.
Dependencies are pinned in Cargo.lock and frontend/package-lock.json.

```sh
git clone https://github.com/richardnoragon/brickxinator.git
cd brickxinator
git switch alpha-foundation
npm --prefix frontend ci --ignore-scripts
npm --prefix frontend run check
npm --prefix frontend run build
cargo test --workspace --locked
cargo build --release --locked
bash scripts/brickxinator
```

The native launcher uses `$XDG_DATA_HOME/brickxinator` or
`~/.local/share/brickxinator`; override with `--data-dir /absolute/local/path`.
It opens the default browser, reuses an instance for that collection, and exits
90 seconds after active app tabs disappear. Keep built assets with the checkout.

For the Linux Docker preview, build the frontend and release binary first:

```sh
docker compose up --build -d
docker compose ps
```

Open [the local app](http://127.0.0.1:4217). The container uses host networking,
UID 1000, and separate persistent data/backup volumes. It stays running between
browser sessions. `docker compose down` retains data; `down --volumes` deletes it.
This locally built preview is not a published portable installer/image.

## Implemented workflows

- Purchase drafts, partial receipts, manual shipments, backorders, receipt
  reversals and independent warranties. Only genuine receipts create ownership;
  unknown copy counts create no invented identities or detailed parts.
- Supplier return plans, actual dispatch, acknowledgment and safe corrections.
  Dispatch removes selected ownership once; acknowledgment/refunds do not repeat
  it. Merged stock returns require explicit acquisition-source quantities.
- Append-only copy lifecycle events: Sold, Donated, Lost, Stolen, Destroyed, Void
  and Returned. UUIDs survive. Explicit linked-part handling supports all/custom
  transfer or retention, with conflict-checked correction to Active.
- Physical holding splits/compatible merges, source layers, loose-stock
  detachment, damage, classification, catalog links, independent storage,
  search and dashboards. Unknown count/weight remains distinct from zero.
- Versioned projects for official sets, MOCs, layouts, dioramas and custom work.
  Reservations, allocations, verified readiness, explicit Start Build conversion,
  progress-only bags/subassemblies and evidence-aware shortages work offline.
- Separate gross amounts, discounts/coupons, payments and refunds. Recorded paid
  totals remain intact. Four automatic financial line-allocation methods use
  exact cents; gift cards/vouchers settle payments without reducing cost again.
- Seven shipping modes including Volume, separate fees, Manual/Piece Count
  acquisition-cost allocation to holdings/copies, shared source budgets,
  residual/disposed cost, reviewed reallocations and non-duplicating cost rollups.
- Reviewed LDraw/MPD, Studio-export CSV/TSV and Rebrickable catalog/BOM/color/theme
  files. Local source caches, UUID-preserving field updates, theme links, local
  catalog images, attributed price observations, explicit BrickLink mappings,
  Wanted List XML and collection/project CSV/Markdown exports.
- Managed images/PDFs, current-schema native JSON/assets exchange, validated ZIP
  snapshots, checked upgrades, interrupted-restore recovery and Restic protection.

Physical holdings are ownership. Assignments reserve stock independently of
assembly/location. Verification is separate from planning coverage. Legacy set
checklists and new project allocations still have some integration limits;
[ALPHA_STATUS](ALPHA_STATUS.md) records the remaining acceptance work.

## Persistence and backup

Rust/Axum serves locally built Svelte assets on 127.0.0.1. Host/Origin checks,
mutation tokens and local-user audit attribution apply. No login, LAN exposure,
telemetry or required cloud service is implemented. Exact large integers remain
readable without JavaScript rounding.

SQLite WAL, foreign keys and immutable typed histories are authoritative;
search/rollups are derived. Migrations 001–018 are immutable. Upgrades create a
validated safety snapshot; use the documented coordinator, not manual DDL edits
or a raw copy of a live database. Native import accepts a fresh collection or an
identical no-op, not an implicit merge of collections.

Daily protection uses a validated ZIP plus external Restic backup/check/retention;
monthly full ZIPs restore independently. Retention is 30 daily / 12 monthly.
Defaults are `<initial working directory>/backups/daily` and `backups/montly`
(requested spelling), editable in Recovery and disabled initially. External/NAS
backup destinations require exact mounts; the live database stays local. Docker
contains Restic but does not configure an independent scheduler. See [INSTALL](INSTALL.md).

## Verification and boundaries

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
cargo build --locked
npm --prefix frontend run format:check
npm --prefix frontend run check
npm --prefix frontend run build
npm --prefix frontend run test:browser
```

Latest feature evidence: **184 Rust tests and 18 Chromium/Firefox scenarios**.
Earlier process/Restic drills and supported-desktop limits are dated separately
in [ALPHA_STATUS](ALPHA_STATUS.md). Dependency/advisory evidence in
[THIRD_PARTY](THIRD_PARTY.md) retains its actual review date.

Imports currently accept scoped, decompressed text files up to 4 MiB. Unknown
mixed-source quantities may block returns; a dedicated provenance-reconciliation
editor remains open. Audited definition reassignment, old/new completeness
integration, project priority/footprint refinements and richer Obsidian packaging
also need further work. Native Studio `.io`, CAD/render/instruction editing,
per-piece assembly, live sync/ordering/price feeds, online catalog/image fetching,
Obsidian import, Kubuntu packages and multi-user/cloud operation are deferred.

Richard's supported-desktop review, release approval and reporting/community
contacts remain gates. Archiving retains ownership; syncing a review branch is
not a release. Public repository: [richardnoragon/brickxinator](https://github.com/richardnoragon/brickxinator).
