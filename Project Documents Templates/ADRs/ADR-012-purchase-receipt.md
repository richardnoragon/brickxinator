# ADR-012: Purchase drafts and atomic whole-purchase receipt

Date: 2026-11-01. Status: implemented engineering decision within existing alpha
scope. Unknown set-copy count behavior explicitly confirmed by the user today.

## Behavior

Purchase drafts accept missing seller, costs, measurements and other available
metadata. Draft, Ordered, Shipped and Cancelled are pre-receipt states and add
no inventory. Only the receipt command can set Received. Empty drafts save but
cannot receive; cancelled purchases must first change status explicitly.

Invoice lines distinguish loose batches, mixed lots and sets. Their paid totals
are authoritative EUR cents and are never allocated automatically to copies.
Shipping is independently optional; incomplete costs stay unknown. Negative or
malformed numeric values fail typed deserialization. Copy counts, part counts
and milligram weights remain separate units.

Receipt introduces exactly one holding for each loose/mixed line with its known
or unknown measurements. A known set copy count creates independent UUIDv7
copies with initial labels and assembly state. No expected/box pieces become
owned part balances. Unknown set copy counts remain unknown and create no
invented copies, as confirmed by the user. There is a 10000-materialized-entity
limit per receipt as an initial resource bound, not a collection capacity claim.

The collection ledger adds an additive Receive fact to the version-1 operation
vocabulary. Existing Create/Change/Identify replay semantics remain unchanged.
Receive facts retain source invoice-line IDs, initial measurements, physical
copy IDs/labels and assembly states. Independent tables preserve foreign keys;
batch provenance follows classification into children. Classification of
purchase stock accepts original mixed lots only; loose batches and identified
children cannot use it for arbitrary splitting.

## Persistence and audit

Migration 002 adds purchases, invoice lines, immutable command audit snapshots,
receipts, stock provenance and set-copy ownership. Migration 001 is unchanged.
An IMMEDIATE SQLite transaction commits the receipt operation, every holding
and copy, provenance, purchase status/revision and audit result together.
The receipt purchase ID and operation ID are unique. A second request key cannot
receive the same purchase. An identical request returns its original result,
including generated identities, after restart; changed payloads conflict.
Revision checks prevent stale tabs from overwriting changes.

Audit records expose before/after purchase snapshots, actor, reason and UTC
timestamp. Descriptive reasons are optional. Receipt supplies a system reason
when none is supplied. Receipt/set-copy/provenance/audit updates and deletes are
blocked by SQL triggers. Replay checks metadata against receipt source facts,
ownership identities and relational provenance. Raw inventory commits cannot
inject a receipt without executing the purchase command.

## Compatibility and incomplete work

Fresh databases install migrations 001 and 002 transactionally. Opening verifies
the checksums of every installed migration and supports either known prefix.
Schema-1 databases keep their inventory functionality; purchase commands return
an explicit schema requirement. Opening never upgrades a database. A tested
safety snapshot and cross-process maintenance coordination are prerequisites for
the future upgrade command. No user data is overwritten to enable this feature.

Received-purchase edits are currently rejected. The required previewed stock
reconciliation is future work, including newly discovering physical copies on
already-received unknown-count lines. This prevents replacing historical facts
or receiving a purchase again as a shortcut. Copy renaming/assembly changes,
verification/checklists, controlled vocabularies, storage, attachments, purchase
dates/tax details, archive/delete, shipping allocation, API and UI remain pending.

## Validation

31 behavioral tests pass, alongside rustfmt/clippy. New cases cover blank drafts,
all pre-receipt states, known/unknown copies, exact totals, persistent retries,
competing receipts, stale revisions, unsupported edits, source provenance,
failure after stock/copy/provenance creation, and schema-1 compatibility.
The synthetic purchase demo ran successfully: zero pre-receipt ownership,
then one holding/three copies, unchanged after reopening and retrying receipt,
with EUR 386.99 recorded once and two audited revisions.
