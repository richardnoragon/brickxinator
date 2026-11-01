# ADR-013: Managed vocabularies and location trees

Date: 2026-11-01. Status: implemented engineering decision within the agreed
settings/storage scope. Part/set definitions and stock assignment are not yet
implemented by this increment.

## Domain rules

Manufacturers, colors and conditions have permanent UUIDv7 identities, revisions,
display names and archive flags. Each vocabulary seeds one protected Unknown.
Conditions also seed editable New, Used and Mixed. System Unknown cannot be
renamed, archived, moved or duplicated, even with name warnings acknowledged.
No settings deletion command exists; SQL deletion is blocked to retain history.

Locations form a forest. Parents must be locations and selectable when assigned.
Self-parenting and descendant cycles are rejected against the current complete
tree under a serialized transaction. Renaming/reparenting preserves identity;
the audit retains previous/new values. Archiving a parent does not delete or
rewrite its descendants. Selection checks the full ancestor chain, so descendants
are unavailable until the parent is unarchived or they are explicitly reparented
under a selectable location. Existing historical references will remain valid.

Names are retained exactly as entered. Comparison collapses whitespace and
lowercases Unicode characters; this is not full Unicode case folding or catalog
identity matching. Equal names and one-character insertion/deletion/substitution
differences (names at least three characters long) produce structured warnings
within one vocabulary or location sibling group. Archived names also participate.
The caller must explicitly acknowledge warnings; nothing is silently corrected.
There is no claim of comprehensive spelling detection. Initial names are bounded
to 128 characters, nonblank and without control characters.

## Persistence

Migration 003 adds settings records with relational parent references, a unique
protected Unknown per vocabulary, six stable seeded UUIDv7 IDs, and append-only
command audit. Earlier migrations remain unchanged. Seeds retain fixed IDs across
new databases. Each command includes actor/request identity, optional reason,
expected revision for edits and explicit name-warning acknowledgment.

IMMEDIATE transactions validate and commit the record and audit result together.
Stale edits fail. Identical retries return their original change after reopening;
different payloads conflict. Request keys cannot be reused across settings,
purchase or inventory command families. Read snapshots replay the settings audit
from fixed seeds and compare it with stored records. Unrecorded row edits,
inconsistent results, invalid trees or changed protected seeds cause validation
failure. Audit history queries expose UTC timestamps and before/after records.

New databases install schema 3. Known older schema prefixes still open for their
existing functionality, without automatic upgrades. Settings methods return an
explicit schema requirement on older databases. Safety snapshots and coordinated
schema upgrades remain future work.

## Evidence and limits

42 behavioral tests pass with rustfmt and clippy, including protected seeds,
acknowledged names, tree moves/cycles, concurrent opposing moves, archival
selection, persistent retries, failed-audit rollback, tampering detection and
schema-2 compatibility. The synthetic settings demo ran and reopened successfully:
10 retained records, 8 selectable after parent archive, two drawer revisions,
and zero inventory operations.

Settings currently stand alone. Linking definitions and inventory to the managed
IDs, stock location assignment/inheritance, whole-holding relocation, typo-warning
UI and the settings dialog remain future increments. Catalog creation must not
create stock. Settings audit replay is unoptimized; no capacity is claimed.
