# ADR-022: Native v1 records and human CSV

Date: 2026-11-01. Status: implemented engineering contract refining ADR-008.

## Native envelope

The portable directory contains `collection.json` and relative `assets/<UUID.ext>`
files. The downloadable ZIP contains exactly those ordinary files, not a database
blob. The envelope declares `format: brickxinator-native`, version 1 and the exact
schema version. Named record families preserve canonical entities, command audits,
movement order, relational identities and timestamps. A fixed registry and known
field order define each family; imported data never supplies executable SQL/schema.
SQL integer cells use decimal strings; text/null remain explicit tagged cells.
Typed domain JSON payloads preserve their original exact representation.

Assets declare SHA-256 and byte length. The manifest must exactly cover attachment
records, and generated filename, MIME signature, association/audit and file hash
must agree. Machine paths, credentials, runtime files and derived FTS caches are
not native collection records. Snapshot configuration is a separate recovery concern.

## Import

V1 is whole-collection initialization, not implicit merging. A fresh schema with
only untouched initial records is required; an identical existing collection is a
validated no-op. Other existing data, including conflicting UUID content, is rejected.
There is no receive/replay-as-new path that could duplicate stock.

Import stages in a private directory, loads only fixed known fields with bound SQL
values, restores application-owned immutable triggers, checks the database/audits/
FKs, then validates every asset. Imported JSON cannot define triggers. The final
empty/conflict check and installation share the maintenance gate, and installation
uses the same safety ZIP/rollback protocol as restore. Invalid input never replaces
the current collection. Current v1 requires schema 12; future compatibility needs
an explicit format/schema adapter rather than silent guesses.

ZIP input rejects duplicate/symlink/traversing/unlisted paths and bounds expansion.
Native JSON is bounded to 256 MiB; per-file asset limits remain 64 MiB. Browser
archive requests are bounded to 512 MiB, with streaming staging/downloads. CLI
directory exchange supports larger supported collections within validation limits.

## CSV

CSV is a scoped flat current-inventory view, not a relational restore format.
It carries target UUID, kind, label, number, piece count and milligram weight.
Unknown measurements are blank, never zero. Fields are quoted/escaped and formula
prefixes are neutralized. Pagination exports all rows, not just the visible page.

## Evidence

Round trips preserve receipt retries, immutable movement order, unknown values,
provenance, audit snapshots, IDs and PDF bytes. Identical re-import is a no-op;
conflicting collections, altered assets and future versions are rejected without
changing the target. CSV tests cover quoting, formula-like labels and unknown units.
Chromium and Firefox download a native bundle and re-import it without duplicating
stock. Snapshot restore remains the explicit replacement path for nonempty collections.
