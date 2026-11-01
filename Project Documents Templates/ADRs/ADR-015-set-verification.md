# ADR-015: Journal-backed set verification and assembly

Date: 2026-11-01. Status: implemented engineering decision within the accepted
set verification, stock integrity and whole-set assembly scope.

## Expected and actual contents

Received physical set copies start with no detailed parts. Verification actions
add expected lines with fresh UUIDv7 line/holding identities, optional part
definition, managed color and optional expected count. A reserved holding ID is
not stock: the actual holding is created only on the first absolute count,
including an explicitly measured zero. New part/color selections are validated.
Initial checklist size is bounded to 10000 lines per copy.

CopyExpected uses a generated preview from another owned copy. It freezes fresh
identities and expected part/color/count facts into the request. Application
checks the source expectations still match before applying the preview. Counts,
finished flags and overall completion are never copied; no stock is introduced.
Preview identities must be retained for identical retries. An empty source or
copying a checklist onto itself is rejected. Reusable definition templates and
line identity edits/removal remain future work.

Counting is absolute, not additive. Reentering 7 stays 7; correcting 7 to 9
appends a +2 ownership effect with reason and preserves previous history.
Changed expected/actual values reopen the affected line and overall verification.
Identical values do not reopen completed work. Finish is explicit after a count;
only finished lines expose Short/Excess/Exact comparisons, and absent expected
values remain Unknown. Overall Complete is separate and requires every line
finished; an empty checklist cannot complete. Status derives NotStarted,
InProgress or Completed without a second mutable source.

If current ownership changed after the last verified total (for example a sale),
recounts are rejected pending explicit reconciliation. They cannot recreate removed
stock. Corrections below existing damage fail balance validation. Count reasons
are mandatory; descriptive/lifecycle commands may use a system default reason.

## Assembly, location and availability

Assembly actions change the whole copy and every counted linked holding atomically.
Both assembled states exclude all linked contents from availability. Loose releases
only usable counts; damage is never repaired or subtracted twice. Unverified sets
can change assembly without inventing parts. Raw per-holding assembly changes and
classification transfers from set contents are rejected.

Set contents retain immutable membership. Their part/color comes from the line,
condition and location from the copy's current profile. Whole-copy moves update
effective content location even while loose. Independent content profile writes,
relocation or existing-stock reassignment are unavailable. `set_contents` returns
the derived view; `inventory_profiles` remains the independent metadata listing.
Receipt copy facts retain initial assembly; `Ledger::assembly` resolves current
assembly from the lifecycle journal.

## Persistence and replay

The operation vocabulary adds typed Verify actions. The immutable inventory
journal records checklists, absolute observations, completion/reopening and assembly
in global order. Per-copy verification revisions reject stale commands. Identical
retries return their original outcome after restart. Verification history rebuilds
before/after copy states with the operation's UTC timestamp, actor and reason.

Migration 005 adds immutable relational line identities, copy/holding membership
and verification operation references. Expected counts and completion flags are
not independently editable SQL balances. Writes commit journal, stock creation,
provenance and references in one IMMEDIATE transaction. SQL targets are validated
against replayed effects, including whole-set assembly changes. Every new source
holding is derived from the operation outcome, not guessed from the action kind.
Failure at any reference/audit write rolls back stock and lifecycle together.

New databases install schema 5. Existing schemas retain their capabilities without
automatic upgrades; verification explicitly requires schema 5. Earlier migrations
are unchanged. Safety snapshots and coordinated upgrades remain unimplemented.

## Evidence and limits

65 behavioral tests pass with rustfmt/clippy. New cases cover absolute counts,
durable retries, explicit finishing/completion, reopened edits, unknown expected
counts, zero observations, damage/assembly overlap, dynamic inherited location,
independent-content protections, concurrent stale counts, disposal reconciliation,
checklist copying, atomic rollback and schema-4 compatibility.
The verification demo ran successfully: zero stock from expected contents,
unfinished 7/10, explicit Short 3 and completion, unchanged stock on reopened
retry, correction to 9 with InProgress status, and 9 available after disassembly.

Catalog template checklists, correcting line identity, line removal/archive,
advanced stock reconciliation, copy label editing, mixed-lot closure/reopening,
search/views, attachments, shipping allocation, backup/restore, API and UI remain
pending. Replay/history queries remain unoptimized; no growth capacity is claimed.
