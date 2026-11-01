# ADR-009: Immutable ordered SQL migrations

Date: 2026-10-31. Status: Accepted user choice; migration runner is draft. Source decisions: D-126.

## Context

The user requests a durable pre-code foundation for a long-lived offline collection, ordinary-file portability and growth beyond the initial collection size. This ADR refines the existing alpha charter; deferred business features stay deferred.

## Decision

Each committed schema migration is ordered SQL: migration_001.sql, migration_002.sql, and so on. Never edit an existing migration; repair with a new one.

## Design and constraints

Maintain applied version/name/checksum/time. Checksums detect edits even in prerelease development. Startup migrates only after safety backup and exclusive maintenance lock; transactional DDL where supported. Restore is the recovery path for destructive changes; do not promise impossible reverse transformations.

## Consequences

Fresh build and every supported upgrade path must be tested. SQL library/version selection is secondary to this contract. Documentation drafts are not deployed migration files.

## Verification

Implement relevant contract tests from TESTING.md before the first release. No runtime behavior is claimed by this ADR. User-selected architecture is accepted; details marked draft are engineering proposals to validate, not retrospectively attributed user answers.
