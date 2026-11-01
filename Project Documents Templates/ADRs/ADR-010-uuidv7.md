# ADR-010: UUIDv7 entity identifiers

Date: 2026-10-31. Status: Accepted user choice; SQLite encoding is draft. Source decisions: D-127.

## Context

The user requests a durable pre-code foundation for a long-lived offline collection, ordinary-file portability and growth beyond the initial collection size. This ADR refines the existing alpha charter; deferred business features stay deferred.

## Decision

Every domain entity gets an application-generated UUIDv7. Permanent identity is separate from editable human labels.

## Design and constraints

Draft SQLite representation: canonical lowercase 36-character UUID text, matching FK types, validated version/variant. API/JSON use that representation. Technical sequences, FTS rowids and schema version numbers are not entity identities. Seed system values with fixed checked-in UUIDv7 IDs.

## Consequences

UUID time order is not transaction order, authorization or an import conflict resolver. Use an explicit local operation sequence for replay and UTC timestamps for audit. This does not implement synchronization or multi-user support.

## Verification

Implement relevant contract tests from TESTING.md before the first release. No runtime behavior is claimed by this ADR. User-selected architecture is accepted; details marked draft are engineering proposals to validate, not retrospectively attributed user answers.

## Primary reference

[RFC 9562, section 5.7](https://www.rfc-editor.org/rfc/rfc9562.html#section-5.7) defines UUIDv7.
