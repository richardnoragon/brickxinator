# Brickxinator

Brickxinator is an offline-first brick inventory and collection application for LEGO, BlueBrixx and compatible systems. It records facts, preserves history and never invents information. Initial use replaces manually maintained Obsidian purchase notes and pictures, especially eBay and mixed-lot acquisitions.

**Status:** the Rust/SQLite foundation and local Axum/Svelte application are
implemented as an alpha source candidate, including files, reviewed corrections,
exchange and tested backup/recovery. See [current source/build commands](../README.md),
[installation](../INSTALL.md), [completion evidence](../ALPHA_STATUS.md) and
[ADR-019 through ADR-022](ADRs/README.md). Target first prerelease:
`0.1.0-alpha.1`; no release/tag or measured large-collection capacity is claimed.

## Alpha scope

- Purchases and shared invoice lines, pictures/PDFs, notes, seller metadata, EUR prices and explicit whole-purchase receipt.
- Independent set copies with permanent IDs and renameable labels; reusable part/set definitions separate from ownership.
- Nested storage and globally available inventory search; dashboard for deliveries, unidentified lots, incomplete records, recent purchases and totals.
- Gradual mixed-lot identification with provenance and no double counting; counts and g/kg weights tracked independently.
- Expected set checklists, gradual actual-count verification, explicit completion, audited corrections and price history.
- Damage retained as owned but unavailable; Lost/Donated/Sold/Correction stock adjustments.
- Six invoice-line shipping modes: Purchase Level, Equal, Price Proportional, Piece Count, Weight and Manual. Paid prices stay authoritative; tax is informational.
- Full ZIP snapshot export plus mandatory external daily incremental backup and separate first-of-month full ZIPs, multiple destinations and tested restore.

Unknown information is acceptable. Unknown is distinct from zero; invalid supplied values require correction or clearing. Drafts can be otherwise blank. Core operation and locally stored assets require no internet connection.

**Storage limitation:** one set copy and all linked contents share one location, even when loose. A standalone batch moves whole. Identified child batches may move independently, but partial batch splitting is deferred.

**Availability:** Assembled and Partially Assembled make all linked contents unavailable. Loose releases only usable actual contents. Assembly, verification and damage are independent; copied expected contents never create stock.

## Platform and architecture

Kubuntu 26.04 LTS, Firefox and Chromium, English interface. Local browser application: Rust/Axum REST/JSON API, SQLite WAL with foreign keys and FTS5, Svelte/TypeScript and ordinary filesystem attachments. UUIDv7 identifies domain entities; inventory balances derive from immutable movements. Loopback only, no login/LAN access. Launcher starts the app; service exits 90 seconds after the last app tab disconnects. Backups run independently through OS scheduling.

Data defaults to `$XDG_DATA_HOME/brickxinator/`, or `~/.local/share/brickxinator/`, and is configurable. Existing Obsidian imports and installable packages are deferred; alpha installation is from source, using INSTALL.md.

## Deferred

Whole-set disposal, build reservations, per-part assembly, set bags/subassemblies, global missing-parts planning/dashboard, partial receipt/reversal, returns/refunds, internal mixed-lot cost allocation, volume shipping allocation, external market history, CAD, train planning, MOC authoring and online integrations. Other platforms, cloud/NAS server deployment and multi-user access are future directions, not alpha support claims.

## Documentation and license

Start with [the documentation index](TOC.md), [domain model](DOMAIN_MODEL.md), [roadmap](ROADMAP.md) and [decision record](DECISIONS.md). All 15 discovery rounds are retained in [DISCOVERY.md](DISCOVERY.md).

Copyright (C) 2026 Richard. Project code: [GPL-3.0-or-later](LICENSE.md). Third-party content retains its own rights. Contributions follow [CONTRIBUTING.md](CONTRIBUTING.md). Public repository: https://github.com/richardnoragon/brickxinator. Private
security reporting remains explicitly deferred until before release.

Pre-code design: [ADRs](ADRs/README.md), [database schema draft](DATABASE_SCHEMA.md) and [Rust structure](RUST_STRUCTURE.md). JSON is native exchange, CSV human export; schema migrations are ordered and immutable. The executable schema and current source supersede historical layout drafts.
