# Changelog

No application version has been released.

## Unreleased

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
