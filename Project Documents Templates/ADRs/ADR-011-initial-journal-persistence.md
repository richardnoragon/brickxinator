# ADR-011: Initial standalone journal persistence

Date: 2026-11-01. Status: implemented engineering decision within the accepted
Rust/SQLite/movement architecture. Not an additional user scope decision.

## Implementation

The pure collection ledger composes the per-holding rules. Domain IDs validate
RFC UUIDv7 on construction/deserialization. Operation order is an independent
contiguous local sequence; each holding has a revision. Identical request retries
return their original outcome even after later changes; changed payloads conflict.

Migration 001 introduces standalone holding identities, immutable operation
envelopes, and foreign-key-constrained operation targets. Typed version-1 JSON
payloads record Create/Change/Identify facts, including numeric effects and
source/child identity. This is a minimal implementation of the schema draft,
not its full purchase/set design or the proposed normalized movement table.
There is one authoritative operation history and no editable balance cache.
Future event versions must preserve version-1 replay semantics.

Whole pieces and milligrams are independent u64 measurements in JSON. Global
SQLite sequences use checked i64 encoding. Negative and overflowing deltas fail.
Deserialization validates balance invariants as well as UUID versions. New
standalone observations explicitly create undamaged stock; they are not receipt
commands and must not be used to bypass later purchase receive-once rules.

Identify creates a fresh child, checks source revision and conserves every known
measure supplied. Missing extraction measurements invalidate known nonzero source
residue; unknown does not become measured zero. Currently only loose sources with
known zero damage in both units can be classified. Damage allocation and set-copy
state are not silently inferred. No arbitrary storage-location splitting is added.

The adapter obtains an IMMEDIATE transaction before replay/validation, then writes
the holding identity, operation and targets together. SQL triggers reject update
and deletion of journal/target/holding rows. Commit timestamps are UTC generated
by SQLite. Replay checks envelope/payload equality, targets, identities, versions
and contiguous sequence. Rebuilding on each write is intentionally unoptimized;
rebuildable caches/indexes remain necessary before capacity claims.

## Runtime and migration boundary

Dependencies are pinned in manifests and Cargo.lock. rusqlite 0.41.1 bundles
SQLite 3.54.1 in the tested build. Startup checks version >= 3.51.3, WAL, foreign
keys, synchronous FULL, 5000 ms busy timeout, and an actual temporary FTS5 table.
This follows the [upstream WAL fix guidance](https://sqlite.org/wal.html)
and [rusqlite connection/transaction API](https://docs.rs/rusqlite/0.41.1/rusqlite/).
Rust's declared minimum becomes 1.89 due to rusqlite; tested toolchain is 1.98.1.
The domain crate now depends on serde and uuid, not SQLite or the OS scheduler.

Only fresh creation is implemented: exclusive create-new filesystem reservation,
transactional initial schema and SHA-256 checksum record. Existing databases
are opened and checked without upgrades. Unknown/mismatched migration ledgers
are rejected. Failed initialization may leave the reserved file for diagnosis;
the adapter never overwrites an existing file. There is no general migration
runner, cross-process maintenance coordinator, safety-backup or restore command
yet. Those remain required before adding upgrade paths or using real data.

## Evidence and limits

20 behavioral tests pass, including a failure injected after partial SQL work,
two concurrent writers competing for the same revision, restart/replay, durable
idempotency, immutable rows, checksum mismatch and unmeasured extraction.
rustfmt and clippy pass. The synthetic inventory demo was run successfully and
reopened/replayed its database. Purchase receipt, settings, locations, attachments,
set verification, FTS search, snapshots, restore and browser tests remain pending.
