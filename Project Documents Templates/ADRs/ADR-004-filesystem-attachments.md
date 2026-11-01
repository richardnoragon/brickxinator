# ADR-004: Filesystem attachments with database metadata

Date: 2026-10-31. Status: Accepted user choice; crash protocol is technical design. Source decisions: D-075, D-121.

## Context

The user requests a durable pre-code foundation for a long-lived offline collection, ordinary-file portability and growth beyond the initial collection size. This ADR refines the existing alpha charter; deferred business features stay deferred.

## Decision

Keep image/PDF bytes outside SQLite. Store UUID, relative managed path, hash, byte size, media type and associations in SQLite. Preserve originals; archives retain files.

## Design and constraints

Stage files, validate/hash, atomically publish managed bytes, then commit references; crashes can leave reclaimable orphan files, never deliberately commit a missing file reference. Cleanup must check all associations and snapshot pins. SHA-256 is the proposed integrity hash. Do not execute active attachment content or shell commands from files.

## Consequences

Data layout remains ordinary files. Restore/snapshot manifest covers referenced managed assets. Filesystem paths are relative under approved roots and cannot traverse out.

## Verification

Implement relevant contract tests from TESTING.md before the first release. No runtime behavior is claimed by this ADR. User-selected architecture is accepted; details marked draft are engineering proposals to validate, not retrospectively attributed user answers.
