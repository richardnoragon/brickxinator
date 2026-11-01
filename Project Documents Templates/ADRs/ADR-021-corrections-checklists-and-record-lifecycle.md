# ADR-021: Reviewed corrections and record lifecycle

Date: 2026-11-01. Status: implemented within accepted alpha corrections/lifecycle scope.

## Acquisition corrections

Received edits use an explicit preview and correction command, never Save or a
second receipt. Line identity, kind and receipt assembly remain fixed. A preview
freezes purchase revision, inventory sequence, residual before/after measurements
and any newly discovered physical copy IDs. A reason is required. Stale or altered
previews conflict. The purchase revision, immutable movement facts and typed
reconciliation references (migration 011) commit in one transaction.

Known acquisition deltas apply to the residual holding, preserving all classified
children rather than subtracting them twice. Unknown baselines can be reconstructed
only when movement measurements justify it. Negative outcomes and reductions below
known damage are rejected. Independent absolute current-stock observations remain
authoritative: a later acquisition metadata edit does not overwrite that balance,
and the preview states this explicitly. Prior acquisition reconciliation operations
are distinguished from independent stock observations. Original receipt measurements
and every earlier purchase snapshot remain immutable and validated through replay.

Increasing a set line's physical count creates only the additional independent
copy identities. Unknown counts never manufacture copies. Detailed part stock is
unchanged. Copy-count reductions cannot discard existing copy identities. Richard
explicitly deferred whole-set sale/donation/loss disposal; archiving is not disposal.

## Checklists and current counts

Definitions may contain reusable expected lists; copying creates independent
unverified line/holding IDs, never owned parts. Part/color identity edits retain
initial references and append explicit FK-backed revisions (migration 012).
Retiring/restoring a checklist line is reversible and preserves verified stock.
All actual linked contents, including retired checklist lines, follow copy assembly
and storage. Completion considers active lines; an empty active list cannot complete.

Ordinary repeated counts remain absolute and idempotent. After part removal or an
independent owned-stock adjustment, ordinary re-entry cannot recreate stock.
A fresh physical recount instead reviews current ownership and holding revision,
requires a reason, and explicitly sets the measured current count. Stale recount
previews fail. Changes below known damage fail; assembly never repairs damage.

## Archive and purge

Purchase/profile archive flags are audited JSON metadata with backward-compatible
defaults. Daily lists may hide them; searches can include them and dashboard totals
retain their ownership. Current set contents inherit the parent archive context.

Richard allows permanent deletion of any never-received draft with no lines or
attachments, even if metadata was entered. Migration 010 permits this explicit
purge only under those constraints. Purchase/shipping metadata and their old
snapshots are removed. A minimal immutable deletion record and reserved request
keys remain, so retries cannot resurrect the record. It is logical deletion, not
secure disk erasure; prior user backup snapshots retain their historical copy.

## Price history and evidence

Definition price history uses currently linked source invoice lines and preserves
distinct price/acquisition revisions across purchases. Grouped set-copy lines are
shown once; no individual copy price is inferred. Mixed-lot source totals are
explicitly unattributed and do not become child-part unit prices.

Regression tests cover conservation, stale/altered previews, discovered-copy
retry/reopen, audit-failure rollback, superseding physical observations, repeated
known/unknown baseline corrections, identity/retirement, fresh recounts, archives,
purge constraints and conflict/no-resurrection behavior. Browser workflows exercise
the corresponding review and lifecycle controls.
