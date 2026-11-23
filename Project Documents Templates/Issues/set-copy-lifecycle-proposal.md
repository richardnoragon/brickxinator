# Alpha scope: immutable set-copy identities and audited lifecycle events

> Status reconciliation, 2026-11-23: accepted requirements and implementation-time notes below are preserved. Current schema-18 implementation, evidence and remaining limits are in [ALPHA_STATUS](../../ALPHA_STATUS.md), [ROADMAP](../ROADMAP.md) and [ADR-023](../ADRs/ADR-023-expanded-alpha-ledgers-and-enrichment.md); old “pending/deferred” text is not the current feature checklist.

Status: accepted alpha requirements, clarified on 2026-11-22. Ready for
implementation design; lifecycle feature not implemented. GitHub publication
was blocked by HTTP 401 at the last attempt. D-136 supersedes D-135's deferral.

## Problem and intended behavior

Record the sale, donation, loss or mistaken creation of a physical set copy
without deleting its identity, provenance, attachments or history. Multiple
copies of one definition have independent lifecycles. For example, selling one
of two Titanic copies changes only the selected copy's current ownership state.

## Concepts and existing contracts

- A reusable Set Definition describes the product and creates no ownership.
- A Set Copy identifies an individual physical copy using its permanent UUIDv7;
  human labels such as S-0001 are display labels, not replacement domain IDs.
- Ownership state is derived from append-only lifecycle events. A separate
  ownership table/entity is an implementation choice, not assumed by this issue.
- Internal definition UUIDs are permanent. Business identifiers and metadata,
  including brand and set number, can be corrected through audited changes.
- If a copy was assigned to an entirely different product, preserve both catalog
  definitions and audit the copy's reassignment to the correct definition. Keep
  the physical copy UUID unchanged; do not repurpose the original definition.
- Purchase provenance, condition, verification and assembly already have distinct
  roles. Lifecycle state must not conflate these or imply verified box contents.
- Existing grouped invoice prices must remain authoritative; do not invent
  per-copy acquisition prices or profit/loss figures from them.

## Proposed states and events

ACTIVE, SOLD, DONATED, LOST, STOLEN, DESTROYED, VOID, RETURNED.
RETURNED was added by the purchase-workflow expansion (D-164).

Every transition records permanent event/copy IDs, effective date, recorded UTC
timestamp, local actor, reason and previous/new state. Proposed optional details:
EUR sale proceeds for SOLD; recipient and notes for DONATED; explanatory notes
for loss/theft/destruction; correction reason for VOID. Unknown proceeds differ
from zero. Sale proceeds do not rewrite the acquisition price.

Never physically delete a copy through a normal application command. Inactive
copies remain accessible in history/search with their attachments and references.
Archive remains an independent visibility setting and never changes ownership.
No database-admin deletion operation is proposed.

Purchased is already represented by once-only receipt. Corrected Purchase may
append an explicit VOID event for selected erroneous copy identities rather than
silently deleting identities after a count edit. Selection, effects and stale
revision checks must be reviewed before committing. The original receipt and
prior purchase revisions remain historical facts. Unknown-count receipts still
create no invented copies.

Supplier returns and receipt reversals were subsequently brought into alpha by
D-159–D-168; see [purchase workflows](purchase-fulfilment-finance-and-warranty.md).
RETURNED preserves identity, with ownership removal on physical handover. Duplicate
merging of set-copy identities remains outside this issue; stock-holding logical
merges do not merge physical copy identities. Valuation, insurance reporting,
profit/loss analytics, lending, wishlists and marketplace integration are future
motivations, not implied deliverables.

## Confirmed decisions — 2026-11-22

1. Bring this feature into alpha, superseding D-135's timing decision.
2. Never infer removal of linked parts from the copy's lifecycle state. Require
   explicit disposition on sale/donation/loss/etc. Offer transfer all, keep all,
   or custom selection. “Ask user” is the required interaction, not a persisted
   unresolved stock disposition. No selection is silently pre-applied.
3. A complete sale can transfer all currently linked actual parts. An incomplete
   sale can retain selected parts, and selling a previously parted-out copy must
   not change independent bulk inventory. Lifecycle must never create box-count
   stock or touch holdings no longer linked to the selected copy.
4. Keep definition identity permanent, with versioned/audited metadata corrections.
5. Every terminal lifecycle state can be corrected through an appended correction
   event referring to the prior event and resulting state. An erroneous sale can
   return the copy to ACTIVE. Preserve the original event and its effective date;
   record the correction's own timestamp, actor and reason. Actual returns remain a distinct workflow, now accepted into alpha under
   D-159–D-168.

## Confirmed implementation requirements — further clarification

### Retained parts become independent loose holdings

Detach all retained actual parts when the copy leaves ACTIVE. They become
first-class loose inventory, independently movable, while preserving part/color
identity, measured quantities, damaged subsets, condition, location and ownership
source/provenance. A partial split must link the retained holding to its source
and lifecycle event; retain the historical set-content/checklist relationship for
audit without treating it as current attachment to the inactive copy.

This was initially accepted as a lifecycle-disposition exception to D-041/D-066.
The subsequent stock-allocation proposal (D-143–D-145) expands alpha to general
splitting/merging and set allocation/removal/transfer. D-146–D-149 resolve the
shared policies: allocations reserve even Loose stock, allocated locations stay
independent, verification can remain Unknown and logical merges retain source
lots. See [the stock handling issue](stock-holdings-and-set-allocations.md). Lifecycle design must
share that model rather than introducing a second stock balance.

### Transfer all / Keep all / Custom quantity

Require a deliberate mode choice. Custom supports partial quantities, not just
whole holdings. Example: a holding of 300 bricks can retain 47 as loose stock
and transfer 253. For each measured unit, retained plus transferred equals the
pre-event owned quantity, with no negatives or duplicated stock. Preserve damage
as a separately accounted subset. Continue the existing independent piece/weight
rules: never convert between units or invent unknown measurements to make a split
appear complete. Invalid or insufficient measurements need correction/explicit
unknown handling under the existing stock invariants before application.

Example acceptance case: A=100, B=50, C=20; transfer A=100/B=40/C=15 and keep
B=10/C=5. The selected copy becomes SOLD; only B=10 and C=5 remain owned as loose
holdings, with their original provenance. Other copies and unrelated bulk stock
are unchanged. No automatic child cost allocation is introduced.

### Audited reversal with impact analysis

Every terminal state can be corrected through an appended event referencing the
original event. Preview the resulting copy state and reversal of that event's
stock/disposition effects. Check current revisions, quantities, links and relevant
changes to detached holdings. A safe reversal can restore the recorded effects;
it must not infer restoration from acquisition totals or a definition checklist.

If later stock movement, disposal, relocation/reassignment or other relevant
changes make the inverse unsafe, return a blocked reversal preview with concrete
per-holding explanations and require review/conflict resolution. Do not force a
reversal, erase history or silently restore only some effects. The blocked attempt
does not change ownership or the copy's lifecycle state; REVERSAL_BLOCKED is an
operation/preview outcome, not an eighth ownership state. Application of an
approved safe plan must revalidate its dependencies atomically.

Restoring a lifecycle mistake is distinct from a real physical return. Supplier
return workflows are now alpha requirements under D-159–D-168. The reversal's own event must record its reason, local actor, previous/
new state and timestamp. The original event remains visible.

### Metadata correction versus definition reassignment

Correct erroneous brand, set number and other supported business metadata using
existing audited catalog correction semantics; retain catalog UUID and enforce
existing uniqueness/namespace rules. EAN/image examples are motivations, not an
implicit requirement to add new catalog fields in this issue.

If the physical copy is actually a different set (Titanic versus Typewriter),
leave the original definition intact and append an audited definition-link
correction on the copy. Preserve its UUID, acquisition provenance and history.
Reassignment alone does not generate parts, replace verified contents, change
counts or copy the new definition's checklist into owned stock. Any correction
of actual contents uses explicit existing verification/stock workflows.

The non-deletion rule applies to lifecycle history and copy identities. It does
not repeal D-134's approved purge of never-received purchase drafts without lines
or attachments. Lifecycle and shared allocation product policies are resolved.
Technical design must satisfy both issues before implementation is accepted.
A successful lifecycle transition releases/transfers only its affected allocation
quantities; it never clears another set's reservation on a shared holding.

## Integrity requirements and acceptance outline

- No destructive copy deletion, broken references, negative stock or invented
  parts/prices. Expected checklists never become actual stock through lifecycle.
- The selected copy changes state independently of other copies of its definition.
- State, explicitly chosen stock effects and audit records commit atomically and
  replay identically. Keep-all leaves owned quantities unchanged; transfer-all
  removes only current linked actual stock after explicit review.
- A state change without a completed disposition decision fails without mutation.
- Keep/transfer/custom controls apply to all approved lifecycle states as appropriate, including
  VOID; VOID is not a destructive database delete or an implicit receipt reversal.
- Retrying the same request is idempotent; stale revisions/previews conflict.
- Historical purchase counts, attachments, verification and assembly remain
  inspectable; archived ACTIVE copies remain owned as before.
- Dashboards, availability, search, native exchange, snapshots and restore agree
  on current ownership while retaining historical inactive copies.
- VOID corrections preview actual stock effects and retain copy identities; no
  automatic reversal of unrelated part-level adjustments or independent counts.
- Regression tests cover independent copies and all approved terminal states;
  transfer-all, keep-all and the partial-quantity examples above; independent
  loose locations/provenance; damage/unknown units and retired checklist stock;
  idempotent retries, stale previews, transaction rollback, replay and recovery.
- Reversal tests include a safe exact inverse, an erroneous VOID correction,
  conflicting later stock changes and an explanation with no partial mutation.
- Metadata correction and wrong-definition reassignment keep all internal UUIDs
  stable and create no stock. Native exchange/snapshot restore preserve lifecycle
  events, detached provenance, correction references and current ownership.

## References

D-084: archive preserves ownership. D-094: definitions separate from ownership.
D-110: audited stock corrections require reasons. D-120: inventory derives from
immutable movements. D-134: narrow draft-purge exception. D-135 and ADR-021
record the former alpha deferral and current implementation boundaries. D-136
through D-142 record the newly accepted scope; existing copy identities remain
permanent, including those marked VOID.
