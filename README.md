# Brickxinator

Offline brick inventory and collection management for LEGO, BlueBrixx and
compatible systems. Copyright (C) 2026 Richard. GPL-3.0-or-later; see [COPYING](COPYING).

The `0.1.0-alpha.1` implementation is being prepared for review. **No alpha release
or tag has been published.** Review [draft PR #1](https://github.com/richardnoragon/brickxinator/pull/1). See [completion evidence](ALPHA_STATUS.md),
[source installation](INSTALL.md) and the [project specifications](Project%20Documents%20Templates/TOC.md).
The public repository is [richardnoragon/brickxinator](https://github.com/richardnoragon/brickxinator).

## Run from source

Target: Kubuntu 26.04, Firefox or Chromium, one local user and one active machine.
Rust 1.89 or newer, Node.js 22.12+ and a C compiler are required. Tested build:
Rust 1.98.1, Node 22.22.1, npm 9.2.0 on Ubuntu 26.04.1 LTS.

```sh
cd frontend
npm ci --ignore-scripts
npm run check
npm run build
cd ..
cargo test --workspace --locked
cargo build --release --locked
bash scripts/brickxinator
```

The launcher opens a local browser service, reuses an existing instance for the
same data directory, and exits after 90 seconds without active app tabs. Crashed
tabs expire after 30 seconds. The default data directory is
`$XDG_DATA_HOME/brickxinator` or `~/.local/share/brickxinator`; override it with
`--data-dir /absolute/local/path`. Keep the checkout and built assets together.

## Alpha workflows

- Save incomplete purchase drafts with seller/type/country/date, order reference,
  informational tax, notes, item notes/source URLs, paid EUR totals and shipping.
  Receive the whole purchase exactly once. Unknown copy counts create no invented
  physical copies; stated box counts never become detailed part stock.
- Manage part/set definitions, manufacturers, colors, conditions and nested storage.
  Protected Unknown values remain stable. Link existing holdings/copies to catalog
  records and move whole holdings or copies without changing quantities.
- Find stock using text/numbers, storage/subtrees, availability, managed attributes,
  seller and purchase dates. Pagination totals cover every match. Archive hides
  daily-list clutter while the dashboard retains ownership totals.
- Identify mixed-lot children without duplicating source stock. Close physically
  empty residue explicitly and reopen without fabricating measured zero.
- Verify set contents with absolute observations, expected checklists, explicit
  line/overall completion and independent assembly state. Reusable definition
  checklists create no stock. Rename copies, correct line identity, retire/restore
  checklist lines, and review fresh recounts after stock adjustments.
- Record Broken, Lost, Donated, Sold and Correction actions on part holdings,
  with independent piece/weight and damaged-subset measurements and audit reasons.
  Preview received-purchase corrections against current residual stock, preserve
  classified children, reject negative outcomes, and reconcile newly discovered copies.
- Upload/paste managed images and PDF documents. Originals remain unchanged;
  generated paths, MIME signatures, byte counts, hashes and target associations
  are validated. Archive preserves attachments. Eligible never-received drafts
  without lines or attachments can be permanently purged, including entered metadata.
- Allocate shipping using Purchase Level, Equal, Price Proportional, Piece Count,
  Weight or Manual. Exact cents reconcile deterministically; incomplete manual
  drafts stay incomplete. Paid prices never change through allocation. View exact
  unit rates with selectable precision and per-definition purchase-price history.
- Export native versioned JSON/assets or human inventory CSV. Native import stages
  and validates an entire collection, initializes a fresh collection, treats an
  identical re-import as a no-op, and rejects implicit merging/conflicting data.

## Persistence and recovery

Rust/Axum serves a Svelte/TypeScript interface with local assets. The API binds
127.0.0.1 and checks Host, Origin and per-process mutation tokens. No login, LAN
service, telemetry or cloud integration is implemented. Audit actors come from
the local service user's environment. Large read integers use decimal strings
where JavaScript would otherwise round them.

SQLite uses WAL, foreign keys, FULL synchronous mode and FTS5. UUIDv7 entities and
immutable typed movement/audit history drive current stock. Opening validates
migration checksums, audit chains, journal replay and relational references.
Migrations 001–012 are immutable; known older prefixes remain readable through
their supported features. Explicit upgrades create validated safety ZIPs first.

Full ZIP snapshots contain a consistent database, referenced assets and optional
configuration, with versioned manifests and hashes. Restore validates before
replacement, creates a safety ZIP for undo, coordinates through OS file locks and
uses a durable rollback journal. Interrupted restore blocks normal opens until
`recover`. Restored machine configuration is placed separately and stays inactive.

Daily protection requires a full validated ZIP **and** successful external Restic
backup/check/retention. Separate monthly ZIPs restore without Restic. Defaults are
`<initial working directory>/backups/daily` and `backups/montly` (requested spelling),
editable in Recovery. Destinations start disabled. External drives/network shares
require exact mount points. Thirty daily recovery points and twelve monthly ZIPs
are retained. The opt-in systemd user timer runs independently of the browser,
checking configurable local schedules and missed-work catch-up. See [INSTALL.md](INSTALL.md).

## Verification and limits

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
npm --prefix frontend run format:check
npm --prefix frontend run check
npm --prefix frontend run build
```

The [evidence checklist](ALPHA_STATUS.md) records actual Rust, Chromium/Firefox,
launcher, migration, recovery and Restic drill results. The real backup drill
restores every retained daily point and every monthly ZIP, including attachments
and inactive configuration review. On the small measured fixture, compressed-ZIP
repository overhead exceeded the retained ZIP inputs; no storage-saving forecast
or large-collection capacity claim is made. Snapshots hold a maintenance gate;
requests may need retrying during maintenance. Review [dependency findings](THIRD_PARTY.md).

Whole-set disposal is explicitly deferred; archiving never removes ownership.
Copy-count corrections cannot delete existing physical copy identities. Partial
receipts/reversal, arbitrary batch splitting, set-content relocation, child-lot
cost allocation, net-price reconstruction, aftercare/returns, reservations, CAD,
MOCs, external market data and online imports remain deferred. Native v1 import
initializes a fresh collection rather than merging into an existing collection.
Image/PDF uploads are limited to 64 MiB; browser archive requests to 512 MiB.
Larger supported archives can use the CLI, within documented extraction limits.

Richard's candidate review on the supported desktop, private security reporting
(explicitly deferred) and community reporting setup remain release gates.
