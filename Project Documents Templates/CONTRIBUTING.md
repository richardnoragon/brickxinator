# Contributing to Brickxinator

The `alpha-foundation` branch is the current source candidate. Work is reviewed
through pull requests; a synchronized branch is not approval to merge/tag/release.
Keep changes scoped and use synthetic fixtures, not private collection data.
Code is GPL-3.0-or-later; do not redistribute third-party catalogs/images/instructions
without establishing the applicable rights and attribution.

Read [ADR-023](ADRs/ADR-023-expanded-alpha-ledgers-and-enrichment.md),
[DOMAIN_MODEL](DOMAIN_MODEL.md), [DATABASE_SCHEMA](DATABASE_SCHEMA.md),
[TESTING](TESTING.md) and [current acceptance work](../ALPHA_STATUS.md).

Preserve immutable UUIDs, source prices/history, unknown-versus-zero, independent
count/weight and exact money. Purchases describe intent; genuine receipts create
stock. Parts/holdings represent ownership; allocations reserve quantities.
Corrections append facts, not historical erasure. Archiving is not disposal.
Direct copy/holding cost shares use one source budget; rollups do not add cost.
Enrichment changes only reviewed fields and never invents owned inventory.

Never edit an applied migration. Add a new checksummed migration and meaningful
upgrade/recovery coverage. Commands must revalidate expected revisions, request
keys and dependencies atomically; UI preview alone is insufficient. Derived
indexes/rollups must remain rebuildable from authoritative history.

Run the standard formatting, lint, type, build and relevant behavioral checks in
TESTING.md. Exercise replay/native restore for persistence changes and both
browsers for affected flows. Record actual evidence and limitations, not assumed
release readiness. Preserve dated discovery/decision/ADR history and explain
superseding behavior explicitly.

Public source/ordinary bugs: https://github.com/richardnoragon/brickxinator.
Private security reporting and community enforcement destinations remain release/
participation gates. Do not disclose private data or sensitive reports publicly.
