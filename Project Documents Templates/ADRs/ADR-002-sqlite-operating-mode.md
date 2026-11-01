# ADR-002: SQLite operating mode and safe snapshots

Date: 2026-10-31. Status: Accepted user choice; safety details are technical design. Source decisions: D-119.

## Context

The user requests a durable pre-code foundation for a long-lived offline collection, ordinary-file portability and growth beyond the initial collection size. This ADR refines the existing alpha charter; deferred business features stay deferred.

## Decision

Enable WAL and enable/verify foreign keys on EVERY database connection before transactions. Require FTS5 support. SQLite is the only alpha database.

## Design and constraints

One local mutation path serializes domain writes; bounded busy handling still required. Draft defaults: synchronous=FULL and busy_timeout=5000 ms, to validate against hardware. Startup verifies capabilities and pragma results instead of assuming defaults. Pin a SQLite runtime with the WAL-reset fix (3.51.3 or later, or verified upstream fixed backport); exact dependency pin follows tooling setup.

## Consequences

Database is local, with WAL/SHM lifecycle handled by SQLite. Snapshot uses SQLite backup facilities, never an uncoordinated database.sqlite copy. Foreign key checks and integrity checks are restore/migration gates.

## Verification

Implement relevant contract tests from TESTING.md before the first release. No runtime behavior is claimed by this ADR. User-selected architecture is accepted; details marked draft are engineering proposals to validate, not retrospectively attributed user answers.

## Primary references

- [SQLite WAL](https://www.sqlite.org/wal.html): readers/writer concurrency, same-host restriction and fixed WAL-reset versions.
- [Foreign keys](https://www.sqlite.org/foreignkeys.html): enable on every connection.
- [Pragmas](https://www.sqlite.org/pragma.html): durability/busy handling settings.
- [SQLite Backup API](https://www.sqlite.org/backup.html): consistent database copy.
