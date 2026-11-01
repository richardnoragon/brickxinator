# ADR-006: Versioned REST and JSON API

Date: 2026-10-31. Status: Accepted user choice; endpoint/transport details are draft. Source decisions: D-123.

## Context

The user requests a durable pre-code foundation for a long-lived offline collection, ordinary-file portability and growth beyond the initial collection size. This ADR refines the existing alpha charter; deferred business features stay deferred.

## Decision

Axum REST API with JSON under /api/v1. Use resource endpoints and explicit command endpoints for receipt, corrections, verification completion and lot classification.

## Design and constraints

Queries are read-only. Stateful commands use idempotency keys and expected record versions; stale tabs receive a conflict rather than last-write-wins stock corruption. Browser-tab liveness uses ordinary periodic HTTP requests; no WebSocket prerequisite. Loopback, origin/host validation and CSRF protection remain required.

## Consequences

Example resources: parts, purchases, invoice-lines, set-copies, batches. Command endpoint examples in RUST_STRUCTURE.md. DELETE is restricted to eligible empty drafts; archive is explicit for linked records. Responses separate invalid input, stale state, incomplete drafts and infrastructure failure.

## Verification

Implement relevant contract tests from TESTING.md before the first release. No runtime behavior is claimed by this ADR. User-selected architecture is accepted; details marked draft are engineering proposals to validate, not retrospectively attributed user answers.
