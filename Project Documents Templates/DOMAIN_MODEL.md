# Brickxinator alpha domain model

Current conceptual specification, consolidated after 15 discovery rounds on 2026-10-31. DECISIONS.md retains rationale/history. This is a design foundation, not an implemented SQL schema.

**Inventory is calculated from immutable inventory movements, not purchase rows.** Current owned/damaged/availability balances are replayable projections; purchase measurements remain provenance. All domain entity IDs are UUIDv7. See [schema draft](DATABASE_SCHEMA.md) and [ADRs](ADRs/README.md).

## Entities and relationships

| Entity | Key information | Relationships / constraints |
| --- | --- | --- |
| Manufacturer, Color, Condition | Permanent ID, display name, archive status, protected-system flag | Settings-managed; one protected Unknown per vocabulary. New/Used/Mixed initial conditions. Historical references survive rename/archive; archived values unavailable for new selection. |
| PartDefinition | ID, manufacturer, part number, description, images as applicable | Exists without ownership. Manufacturer/part number identify known parts; color/condition distinguish stock. Incomplete identities never auto-merge; manual linking audited. |
| SetDefinition | ID, manufacturer, set number, name, year, theme, stated piece count, instructions, reusable expected contents | Separate namespace from part numbers and separate from owned set copies. No definition creates stock. |
| Purchase | ID, status, seller/name/type/country, date, order number, EUR currency, optional shipping, tax status/amount/rate, notes, archive state | Has invoice lines and attachments. Status Draft/Ordered/Shipped/Received/Cancelled. Unknown fields and blank drafts allowed. |
| InvoiceLine | ID, purchase ID, stable row order, description/kind, optional copy quantity, brick count, weight, total paid cents | Price-bearing source record. Copy count and brick count are distinct. Can fund several independent set copies or holdings. No invented per-copy cost. |
| SetCopy | Permanent ID, display label, definition if known, source invoice line, condition, assembly state, shared location | Initially numbered label, freely renameable. Receiving creates copy only. Owns checklist and actual verified contents; not a second part balance. |
| Checklist / ChecklistLine | Set copy, expected part/color/count where known, verified actual count, line-finished flag, history | Copied definitions are expected/unverified only. Count entry absolute. Overall completion explicit after all active lines finished. Count edits reopen affected line and overall verification visibly. |
| Verification / StockCorrection | Set/checklist/actual-stock references, previous/new values, timestamp, local user, mandatory stock-change reason | Newly observed set contents recorded once. Existing-stock reassignment deferred. Expected changes never mutate actual stock. |
| StockBatch | ID, source invoice line, optional parent lot/set copy/part definition, color, condition, location, optional owned count/weight, damaged measures, measurement validity | Current holding separate from acquisition. May be unidentified. Set-linked contents inherit set location; independent batch has one location. |
| IdentificationTransfer | Parent lot, child batch, extracted measures, validity effects, timestamp and provenance | Conserves valid known measurements. Missing extraction measure invalidates affected residual measure, never estimates. Original purchase/photos/seller/cost links retained. |
| ClassificationEvent | Lot, complete/reopen action, confirming local user, timestamp, reason | Known nonzero residue blocks completion. Unknown residue may be physically-empty confirmed without fabricating measured zero. Reopen preserves prior confirmation. |
| StockMovement | Target holding, operation, owned/usability deltas or before/after facts, timestamp/user/reason | Receipt, Lost, Broken, Donated, Sold, Correction, identification transfer. Transfer/damage differ from ownership disposal. Reject impossible balances. |
| Receipt | Purchase, effective receipt record, timestamp, linked stock movements | Whole purchase once. Repeated submissions cannot receive again. Partial receipt/reversal deferred. |
| Location | Permanent ID, parent ID, display name, archive state | Tree without cycles; managed names with duplicate/similarity warnings, no silent correction. Set/contents share one location. |
| Attachment / Association | Managed path, type, original name, integrity data, associated records | Images, pasted images, PDF invoices/instructions. Shared associations preserve provenance without duplicating stock or price. Retained on archive. |
| ShippingAllocation / Share | Purchase method, invoice-line target, share cents, complete/incomplete state | Purchase Level, Equal, Price Proportional, Piece Count, Weight, Manual. Manual draft shares untouched; complete shares reconcile exactly. |
| Revision | Record, previous/new values, timestamp/local user, reason where required | Immutable history with corrected current view. Stock correction reason mandatory; descriptive reason optional. |
| BackupDestination / BackupRun | Destination and reviewed activation, schedule kind, recovery point, dependencies, success/failure/last success | Local/external/network targets. 30 daily plus baseline dependencies; 12 independent full monthly points. Implementation engine pending. |

These are conceptual entities, not a requirement for one table each. DATABASE_SCHEMA.md now drafts columns/indexes/transactions. Immutable ordered migrations must preserve the rules below. Current quantity fields on holdings mean derived projection values, not editable sources independent of the journal.

## Purchase versus ownership

Draft/Ordered/Shipped purchases do not increase inventory. Explicit whole-purchase receipt introduces holdings; set receipt introduces only owned SetCopy records, not expected detailed parts. A line with three sets and EUR 300 creates three independent copies with unknown individual price, retaining EUR 300 once at the invoice line.

Acquisition facts are historical. Corrections append revisions and reconcile stock after a preview. They never replay receipt or erase original measurements. Reject corrections below already consumed/transferred facts when no valid reconciliation exists. Child transfers are not an additional consumption of their own destination holdings.

Cancelled purchases do not create stock. Received purchases cannot be receipt-reversed in alpha. Sold/Donated/Lost are separate stock events, not edits to the purchase quantity. Sale/refund accounting is outside scope.

## Set verification and availability

Assembly: Loose, Assembled, Partially Assembled. Verification: Not Started, In Progress, Completed. They are independent; already-built unverified sets are valid. All actual linked parts are unavailable in either assembled state. Loose releases only otherwise usable actual stock. No per-part assembly allocation.

Expected checklist contents are not owned. Verification counts are absolute: 7 entered twice stays 7; 7 → 9 is an audited +2 correction. Verification records contents of this set, not reassignment from other stock.

Line finish is explicit. Expected 10 / counted 7 / unfinished means 3 unverified, never automatically missing. Finish permits Short 3. Counted 12 permits Excess 2. Complete Verification is a separate explicit action after every active line is finished. Expected/verified count edits visibly reopen the line and overall verification; expected edits alone never change actual balances.

Stated box piece counts stay separately labeled; do not add them to actual verified parts. Definitions, set-copy counts and checklist counts are not additive part holdings.

## Damage and measurements

Piece count and g/kg weight are independently optional. Both describe the same stock; no conversion between them. Unknown differs from measured zero. Counts are whole numbers; malformed/negative supplied quantities are invalid.

Damage is separate from condition/assembly: 100 owned, 2 damaged, loose yields 98 available. Assembly and damage overlap without double-subtraction. Unknown damaged weight makes usable weight unknown when applicable. Availability filters: Available, In Assembled Set (including partially assembled), Broken/Damaged, Unknown. Filters can overlap; dashboard totals must not sum them as disjoint ownership buckets.

Search combines known measures for matching stock while exposing missing coverage; unidentified mixed-lot weight stays separate from identified stock. Original acquisition measurements never substitute for current residual measures.

## Lot conservation and location

Source residual plus transferred children conserves every valid known measure, except explicit separately audited ownership adjustments. Reject extraction above known remainder in the same unit. After unweighed extraction from a 5 kg lot, preserve acquisition 5 kg but current residual weight is unknown; never estimate 4.2 kg.

Known nonzero residue prevents classification completion. For unknown residue, explicit physical-empty confirmation closes the current residual holding without rewriting unknown historical measurements into measured zero. Reopen is audited and does not itself invent stock.

Identified child batches inherit location and can then move whole. A single batch cannot be partially split across locations. A set copy and all linked parts always share its location, even loose. Arbitrary batch splitting and independent set-content movement are deferred.

Mixed-lot cost stays on its invoice line; child costs can remain unknown. Internal child-cost allocation is deferred, distinct from purchase shipping allocation.

## Costing

Paid invoice-line total is authoritative EUR money. Optional tax Included/Unknown/Not Applicable/Added Separately is informational, never an extra addition to already-paid prices. Purchase total is paid line totals plus shipping, with incomplete status if unknown inputs remain.

All shipping modes target invoice lines. Equal means per line. Price mode uses known line paid totals, not invented per-copy prices. Piece mode needs known line brick count, not set-copy quantity. Weight uses known line weight. Missing inputs/all-zero basis produce incomplete calculation; known zero basis gets zero share. Largest-remainder cents, stable row-order ties. Manual drafts keep entered shares and show remainder; effective allocated costs unavailable until balanced. Purchase Level computes no line shares.

Unit costs derive EUR/piece and EUR/kg only from valid positive acquisition measures and known costs. Preserve higher precision internally, configurable display precision. Shared prices do not justify invented individual-copy rates. Historical paid-price reporting is separate from external market prices, which are deferred.

## Archive and audit

Archiving changes visibility, never ownership or balances. Retain attachments, searchability and unarchive. Referenced vocabulary values retain IDs and historical links. Empty never-received drafts can be permanently deleted; linked history/stock must not be erased.

## Implementation decisions still to record

Executable SQL/constraint verification, dependency pins, attachment limits and external incremental adapter. Schema encoding/index/API proposals are documented in DATABASE_SCHEMA.md and RUST_STRUCTURE.md; validate them before implementing immutable migrations. Backup architecture is fixed: full ZIP coordinator plus mandatory external daily incrementals and independent monthly ZIPs. Use atomic transactions for stock operations and staging for files. These are design tasks against established functional constraints, not reasons to restart broad discovery.
