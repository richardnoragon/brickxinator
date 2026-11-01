# ADR-008: Versioned JSON, CSV and snapshot manifests

Date: 2026-10-31. Status: Accepted user choice; compatibility contract is draft. Source decisions: D-125.

## Context

The user requests a durable pre-code foundation for a long-lived offline collection, ordinary-file portability and growth beyond the initial collection size. This ADR refines the existing alpha charter; deferred business features stay deferred.

## Decision

JSON is native exchange; CSV is human export; assets stay filesystem files. Future LDraw/Studio/Rebrickable and existing Obsidian imports remain deferred.

## Design and constraints

Publish a versioned JSON envelope, UUID references, UTC timestamps, explicit numeric unknowns and canonical categorical Unknown IDs. CSV is scoped flat views, not a lossless relational restore format. Portable JSON bundle references relative assets by hash/path; snapshot metadata includes app/schema/format versions and integrity inventory.

## Consequences

Native import validates/stages before committing; it must not receive a purchase or replay stock twice. UUID collision with different content is a conflict, not an automatic overwrite. Exact format compatibility/round-trip tests precede native import implementation. See DATABASE_SCHEMA.md.

## Verification

Implement relevant contract tests from TESTING.md before the first release. No runtime behavior is claimed by this ADR. User-selected architecture is accepted; details marked draft are engineering proposals to validate, not retrospectively attributed user answers.
