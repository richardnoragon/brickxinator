# ADR-016: Typed adjustments and mixed-lot lifecycle

Date: 2026-11-01. Status: implemented engineering decision within accepted stock
adjustment and classification-completion scope.

## Stock adjustments

Lost, Donated and Sold record measured removal totals independently in pieces
and milligrams. Damaged subsets are explicit and default to zero; they may not
exceed either removed totals or known damage. The remaining damage must fit the
remaining ownership. Unknown ownership/damage in a selected unit requires
clarification rather than a guessed usable balance. At least one positive removal
measurement is required. Missing residual dimensions become unknown; no units are
converted. Damaged amounts without removal totals in the same unit are rejected.
Purchase prices, original acquisition measurements and child provenance do not
change. Sold does not implement sale-price accounting, refunds or aftercare.

Broken increases damage while retaining owned measurements. Supplied damage
increments require known prior damage/ownership and cannot exceed ownership.
Missing damage measurements become unknown (except units with known zero owned
stock), so usable weight/count is not invented. Independent assembly restrictions
remain in place. Correction uses explicit typed measure actions for observation,
delta or invalidation, requires a reason and preserves previous facts.

Set-owned quantity corrections use verification, not generic edits or typed
ownership corrections that would bypass checklist reopening. Named disposals
remain available, after which recounts require the pending reconciliation flow.
Legacy journal facts continue replaying; schema-6 writes enforce these routes.
All adjustment and reopen commands require actor/request identity and a reason.
Complete classification may supply a system reason when no descriptive reason
is entered.

## Completion and reopening

Classification lifecycle applies only to original received MixedLot holdings,
not loose batches, set contents or classified children. Completion is explicit:
known nonzero ownership or damage in either unit blocks it even with a physical
confirmation. All known zero measures can complete without confirmation; any
unknown residual measure requires explicit physical-empty confirmation.

Completion appends a lifecycle fact, not measured-zero numeric events. It
increments the holding revision and excludes the residue from current inventory.
The historical balance and purchase measurements remain unchanged. Additional
completion, adjustments, generic stock edits and extraction are rejected until
an explicit audited Reopen. Identical request retries still return original results.

Reopen requires prior completion and records its own operation ID/reason/date
while retaining the last completion/confirmation evidence. It changes no measure.
Unknown residue stays unknown; measured-zero residue stays zero. New findings
require explicit observed corrections before extraction above known zero. Such
corrections are separate ownership facts, never another purchase receipt or a
restoration of acquisition weight.

`Ledger::holdings` contains historical replay balances. `current_holdings` and
`Store::current_inventory` exclude completed residual lots and are the current
stock interfaces. History queries expose previous/new measurements, lifecycle
states, UTC commit dates, actors and reasons. UI/search/totals must use current
inventory, not raw historical holdings.

## Persistence and compatibility

The global operation vocabulary adds Stock adjustment/completion/reopen facts.
Migration 006 adds immutable FK-constrained stock-operation references; no
independent editable quantity or closure cache exists. IMMEDIATE transactions
commit facts/references together, with holding revisions and shared request keys.
Replay checks action kinds, targets and classification eligibility against
received provenance. Failed reference writes roll back lifecycle and quantities.

Fresh databases install schema 6; previous schemas retain their existing features
without silent upgrades. Typed stock/lifecycle methods explicitly require schema
6. Older migrations remain unchanged. Safe upgrades and recovery are still pending.

## Validation and remaining work

76 behavioral tests pass with rustfmt/clippy. New cases cover broken/removal
arithmetic, damaged subsets, unknown measurement guards, purchase preservation,
physical-empty confirmation, known-zero closure, blocked closed-state changes,
audited reopening, persistent retries, concurrent removals, injected failures,
route protections for set contents and schema-5 compatibility.
The stock demo ran successfully: closure leaves one active child holding,
reopening leaves source measures unknown, damage/donation yields 95 owned / 2
damaged / 93 available, and the original 5 kg acquisition stays intact.

Received-purchase/recount reconciliation, configurable current-stock views,
search/FTS projections, archive workflows, shipping allocation, attachments,
backup/restore, safe upgrades, API and UI remain future increments. No capacity
or real-collection readiness is claimed by the unoptimized journal queries.
