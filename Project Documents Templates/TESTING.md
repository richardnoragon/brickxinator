# Brickxinator testing and evidence

Updated 2026-11-23. Latest feature evidence: **184 passing Rust tests and 18
Chromium/Firefox browser scenarios**, formatting/clippy/type checks and production
builds. Tests use disposable data; browser requests outside loopback are blocked.
[ALPHA_STATUS](../ALPHA_STATUS.md) retains the exact dated increments, Docker
checks, earlier process/Restic drills and supported-desktop limits.

## Standard checks

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

Browser tests require the debug executable and installed Playwright Chromium/
Firefox browsers. See INSTALL.md. The configuration starts a disposable service;
do not point mutation acceptance tests at a real collection.

## Integrity cases

- Unknown is not zero; no fabricated piece/weight conversion, copy identities or
  expected BOM stock. Damaged stock stays owned but unusable.
- Receipt/verification/classification/split/merge/disposal conserve facts and
  preserve source layers. Multiple quantities remain independently measured.
- Lifecycle and receipt/return corrections retain originals, revalidate dependencies
  under the write transaction and reject stale/unsafe changes atomically.
- Merged returns consume only explicitly selected known acquisition quantities
  and their cost layer, never an implicit FIFO/proportional source choice.
- Reservations/allocations affect availability independently of construction;
  Start converts explicitly and Ready requires verified usable committed supply.
- Confirmed missing, potential gap and unverified supply remain distinguishable.
- Discounts/refunds allocate exact cents without fallback; payments do not reduce
  acquisition cost twice. Original paid amounts remain unchanged.
- Holding/copy shares share one source/component budget; source residual/disposed
  amounts reconcile. Copy cost bases create no stock. Rollups count existing
  shares once, use actual allocated quantities and retain unknown cost coverage.
- Reviewed reallocation handles financial source changes, including zero-target
  correction after a full refund/disposal without fake active inventory.
- Imports preserve local UUIDs/unselected overrides. Color Unknown is protected;
  invalid RGB, duplicate provider IDs, theme cycles/missing parents and stale
  settings/metadata prefixes fail without partial writes.
- Native exchange preserves new histories, theme links, direct copy costs and
  source caches. Managed assets retain bytes/hashes/associations; malformed files,
  traversal and conflicting imports are rejected.

## Browser coverage

Both browsers cover ordinary purchase/receipt/files, archives, exact rates,
verification/recount, mixed lots/corrections, native exchange and responsive views;
projects/reservations/readiness; partial receipts/reversals; financial review,
supplier dispatch and holding costs; copy costs/rollups; reviewed catalog/color/
parent-child theme imports, cached files, local image uploads and Markdown downloads.
Desktop/390-pixel screenshots were inspected for the new workflows.

## Recovery and release evidence

Earlier opt-in real-process/Restic drills are dated separately, not counted as
freshly rerun by every feature change. They exercised process reuse/shutdown,
31 real Restic runs and restoration of all 30 retained daily / 12 monthly points,
including attachments and inactive configuration review. Recovery tests cover
migration failure, tampering, maintenance contention, interrupted restore and undo.

The build host is Ubuntu 26.04.1; real Kubuntu desktop/default-browser/native
clipboard acceptance and maintainer release approval remain open. A clean branch,
green headless tests or a Docker rebuild does not replace desktop sign-off.
Dependency advisories retain their actual review date in THIRD_PARTY.md; refresh
that evidence before release rather than relabeling historical results.
