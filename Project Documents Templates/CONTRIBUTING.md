# Contributing to Brickxinator

Status: alpha source review candidate. See [INSTALL](../INSTALL.md) for build
and check commands, [ALPHA_STATUS](../ALPHA_STATUS.md) for evidence and limits. Project software is GPL-3.0-or-later.

## Workflow

Use main plus short-lived feature branches and reviewed pull requests (D-116). Conventional Commits are preferred, not mandatory. State the concrete behavior changed, why, and how it was validated. Keep changes scoped; update domain documents and decision records when behavior changes.

Keep supported toolchain versions, source-build steps, launcher and independent
backup setup current in INSTALL.md. Tests must use disposable data.

## Required checks

Rust: rustfmt, clippy and relevant behavioral tests. Frontend: configured formatting and TypeScript checks plus behavior tests appropriate to the change. Stock, cost, verification, receipt, correction and transfer changes require meaningful invariant/regression tests. See TESTING.md for acceptance cases and RELEASE_PROCESS.md for release gates.

## Domain rules contributors must preserve

Unknown is different from zero and invalid. No invented prices, weights or parts. Definition/checklist creation does not create stock. Receipt happens once. Verified quantities are absolute. Preserve source history and audited corrections. Damage affects usability, not ownership. Classification transfers do not duplicate balances. Per-set storage restrictions and deferred scope remain in force.

## Content and reports

Do not add third-party images, instructions or catalogs without establishing redistribution rights and attribution. Do not commit real purchase invoices, personal seller details or private paths as fixtures; use synthetic data.

Public source/ordinary bugs: https://github.com/richardnoragon/brickxinator.
Private security-report destination is explicitly deferred and remains a release gate. Do not publish sensitive data in a public issue. Community policy is drafted in CODE_OF_CONDUCT.md; enforcement contact must be selected before outside participation is opened.

## Technical contracts

Read ADRs/README.md, DATABASE_SCHEMA.md and RUST_STRUCTURE.md before stock/persistence work. Never edit an existing migration. Append audited movement facts instead of writing quantity balances directly. Domain IDs are UUIDv7; FTS and projections must be rebuildable. Source metadata, file publication and snapshots obey the documented consistency protocol. Backup adapters run as external processes; do not embed a backup engine.
