# Changelog

No application version has been released.

## Unreleased

### Expanded alpha implementation — 2026-11-23

- Schema 013–018: project/BOM/claims, physical/source-flow history, fulfilment/
  aftercare/financial history, derived cost history, integration source caching
  and audited color/theme enrichment. Applied migrations remain unchanged.
- Added lifecycle/disposition/safe corrections, stock splits/merges/detachment,
  partial receipt/reversal, supplier return stages, warranties and exact financial
  adjustments separated from settlement/ownership.
- Added Volume shipping/fees, holding/direct-copy shared cost budgets, residual/
  disposed costs, reviewed reallocations and non-duplicating copy/project rollups.
- Added projects/readiness/shortages/progress, attributed prices, Studio-export/
  LDraw/Rebrickable adapters, color/theme/local image enrichment, explicit
  BrickLink mappings/Wanted List XML and Markdown export.
- Updated Linux Docker preview and source installation; snapshots/native exchange
  retain all added history. No binary release/tag or automatic backup activation.
- Validation: 184 Rust tests, 18 Chromium/Firefox scenarios, formatting/lint/type/
  build checks, native/recovery regressions, deployed hashes and Docker doctor.
  Earlier opt-in Restic/process drills retain their separate evidence dates.
- Reconciled operational documentation; ADR-023 supersedes earlier scope models
  while preserving discovery/decision history. See ALPHA_STATUS for limits.

### Alpha source candidate

Implementation: [c1551b5](https://github.com/richardnoragon/brickxinator/commit/c1551b5).
Review: [draft PR #1](https://github.com/richardnoragon/brickxinator/pull/1).

- Rust inventory ledger, purchases, catalog/storage, set verification, lot
  classification, stock adjustments, exact shipping/costing and search.
- Local Axum/Svelte application with managed files, reviewed corrections,
  archive/purge, native JSON/assets exchange and inventory CSV.
- Checked migrations, snapshots, rollback recovery, Restic daily backups,
  monthly ZIP retention and opt-in systemd timer setup.
- Validation and remaining release gates: [ALPHA_STATUS](../ALPHA_STATUS.md).
  This entry describes the `alpha-foundation` review branch, not a release.

### Documentation baseline

- Alpha discovery documentation: 15 answered rounds, decision history, domain model, architecture, user workflows, test requirements, security/backup policy, contribution and release plans.
- GPL-3.0-or-later project licensing decision and canonical GPLv3 text.
- Pre-code technical ADR set, database schema draft and Rust crate/module/API plan; explicit mandatory external daily incremental backup clarification.

A release entry must identify the reviewed commit and approved tag. No release
commit or tag is claimed by this unreleased entry.
