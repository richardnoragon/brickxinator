# ADR-014: Catalog definitions and descriptive inventory links

Date: 2026-11-01. Status: implemented engineering decision within the accepted
catalog/settings/storage scope. This increment does not add ownership implicitly.

## Catalog identity

Part and set definitions have separate namespaces, permanent UUIDv7 IDs,
revisions, archive flags and manufacturer references. Number/name/description
may be missing. Set definitions also accept release year, theme and stated piece
count. Creating any definition adds no inventory or physical set copies.

Known manufacturer plus number uniquely identifies a definition in its namespace,
including archived definitions. Number comparison trims outside whitespace and
lowercases characters; raw supplied numbers are retained. Incomplete identities
(Unknown manufacturer or missing/blank number) remain independent records and
are never auto-merged. Display-name changes never alter manufacturer identity.
The initial bounded text rules allow 512-character number/name/theme fields and
64 KiB descriptions; these are engineering limits, not collection-size claims.

## Inventory metadata and storage

An inventory profile targets one existing holding or physical set copy. It holds
an optional part/set definition link, managed color/condition IDs and optional
location ID. An unlinked definition implies unknown catalog identity; defaults
use the protected Unknown color/condition IDs. Manufacturer identity comes from
the linked definition rather than a duplicate editable inventory field.
Holding targets require Part definitions; SetCopy targets require Set definitions.
Profiles cannot assign existing holdings to set copies or create detailed parts.

Profiles have independent revisions and descriptive audit history. Linking,
relinking, clearing metadata and moving an entire target do not append stock
quantities or change provenance. There is no partial holding/location split.
Set-copy locations describe the whole copy; detailed verified contents and their
inherited effective location still require the future verification aggregate.

New selections must reference active catalog/settings records and selectable
locations (all ancestors active). Retained archived references remain valid
while other metadata changes. Historical replay validates reference type and
existence without retrospectively treating today's archive state as a past error.

## Classification inheritance

If a source holding has a profile, classification creates a child profile in the
same transaction. It inherits location only; part/color/condition are not guessed.
The inherited link remains valid even if the source's location was archived.
Later source moves do not move the child, and later child moves do not move the
source. A related inventory operation ID links the automatic profile audit to its
source/child facts. Replay checks the inherited location against the source profile
at that point in metadata history. Identical inventory retries do not inherit again.
Failure to write the profile or its audit rolls back the entire stock transfer.

## Persistence and compatibility

Migration 004 adds catalog records, catalog audit, inventory profiles and profile
audit. Earlier migrations remain unchanged. Relational FKs constrain targets,
definitions, vocabulary references and locations; application rules enforce their
specific categories. Known identity keys are SQL-unique. Deletion and audit
mutation are blocked. Every command uses IMMEDIATE transaction/revision checks,
shares the command request-key namespace and supports durable original-result
retries. Current records are checked against immutable audit replay on reading.
History queries return UTC timestamps, actor/reason and before/after snapshots.

Only new databases install schema 4. Schema-1/2/3 databases keep their established
capabilities without silent upgrades. Catalog/profile methods explicitly reject
older schemas. General safe upgrades still require maintenance coordination and
a verified safety snapshot; no existing development database is overwritten.

## Validation and remaining work

53 tests pass with rustfmt and clippy. New cases cover no-stock definitions,
known/incomplete identities, namespaces, stale and concurrent edits, persistent
retries, archived references, set-copy linking, independent child relocation,
inherited-profile failure rollback, audit tampering and schema-3 compatibility.
The catalog demo ran and reopened successfully: zero holdings after definition,
then conserved 80/20 pieces, an inherited drawer, and two child profile revisions.

Pre-receipt catalog/storage selections, copy renaming/assembly changes,
verification/checklists, received-purchase reconciliation, attachments, search
indexes/views, shipping allocation, backup/restore, API and UI remain pending.
Queries currently return complete maps and replay history; pagination and growth
performance are not yet implemented or measured.
