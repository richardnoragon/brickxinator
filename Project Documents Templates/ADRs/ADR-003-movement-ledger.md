# ADR-003: Movement-based inventory source of truth

Date: 2026-10-31. Status: Accepted user choice; typed movement design is draft. Source decisions: D-120.

## Context

The user requests a durable pre-code foundation for a long-lived offline collection, ordinary-file portability and growth beyond the initial collection size. This ADR refines the existing alpha charter; deferred business features stay deferred.

## Decision

Inventory is derived from inventory movements, never recalculated from purchase rows. Purchase/verification/adjustment commands append immutable stock facts in one atomic domain operation.

## Design and constraints

A single quantity_change cannot represent unknown weight, damage or assembly correctly. Use typed measure transitions and domain-operation events; projections are rebuildable caches. Original movement rows are never rewritten; corrections append events. Assembly and classification lifecycle facts are in the same ordered inventory operation history. General descriptive metadata can use conventional audited CRUD.

## Consequences

Broken changes damaged holdings, not owned holdings. Transfers have paired source/destination effects. Absolute verified totals become validated changes against current replay state, not repeated additions. Unknown is not a zero opening balance. See DATABASE_SCHEMA.md.

## Verification

Implement relevant contract tests from TESTING.md before the first release. No runtime behavior is claimed by this ADR. User-selected architecture is accepted; details marked draft are engineering proposals to validate, not retrospectively attributed user answers.
