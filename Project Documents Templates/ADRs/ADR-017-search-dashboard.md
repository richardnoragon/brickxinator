# ADR-017: Rebuildable inventory search and dashboard read models

Date: 2026-11-01. Status: implemented engineering decision within the accepted
FTS5/current-stock/search/dashboard scope. These are library workflows, not a UI.

## Current-stock documents

Search documents resolve current ledger holdings with audited catalog/settings,
source invoice/purchase metadata and inherited set-content attributes. Completed
mixed-lot residue is excluded. Physical copies are separate searchable rows with
no invented part quantities. Received set lines with unknown copy counts are
searchable UnquantifiedSetLine records, not invented physical copies. Expected
checklist lines never introduce part documents until actually counted.

Metadata includes part/set number, label, managed manufacturer/color/condition,
location ancestor path, source seller/description/notes, current assembly, partial
measurement coverage and verification status. Set contents derive condition and
location from their copy's current profile and part/color from the checklist.
Number lookup on a copy may fall back to its invoice set number. Stated per-copy
piece counts come only from a set definition, never a grouped invoice line.

## Queries and totals

Exact normalized-number filters are separate from full text. Text input is split
into at most 32 alphanumeric terms, quoted as literal FTS tokens and joined with
AND; user text cannot supply SQL or FTS operators. Exact whole-query number/UUID
matches are also candidates. Exact numbers rank first, followed by stable
target-kind/UUID order. Structured filters cover manufacturer, color, condition,
location (optionally descendants), record kind, availability, incomplete records
and archived references. Limits are 512 text characters and 1–200 rows per page.
The implementation follows [SQLite's FTS5 documentation](https://www.sqlite.org/fts5.html).

Query totals cover all matches, independent of SQL LIMIT/OFFSET. Piece and
milligram totals remain separate and report known sums plus counts of known and
unknown holdings. A zero known subtotal with unknown coverage is not an exact
zero inventory measurement. Sums use u128 and serialize as exact decimal strings.
Subtotals partition holdings into original mixed-lot residue, catalog-linked
holdings and other holdings; catalog-linked does not promise complete identity.
Copies and unquantified set lines are counted separately from actual parts.

Availability facets overlap: positive available count/weight, an assembly
restriction, positive damage, and missing actual measurements are independent.
Filtering a facet does not add its totals to other overlapping facets. Archived
catalog/settings references remain in current stock and searchable by default;
archived-reference filtering is not inventory-record archiving.

Incomplete is an informational initial heuristic: missing catalog identity,
manufacturer/condition, required stock color or location, no known stock
measurement, or unknown set-copy count. Optional missing weight alone does not
make a count-measured record incomplete. Purchase incompleteness separately
checks seller, costs, empty lines, unknown copy counts or entirely unknown stock
measurements; cancelled purchases are excluded from that counter.

Dashboard totals use every current document once. Pending deliveries count
Ordered/Shipped purchases. Recent purchases are the five most recently created
audit records by purchase-command sequence, with recorded UTC timestamps;
UUID order and absent purchase dates are not used as purchase chronology.

## Cache and concurrency

Migration 007 adds rebuildable document/FTS tables, indexes and a dirty marker.
Internal integer FTS rowids never replace domain UUIDs. Source mutations on the
inventory journal, purchases/invoice lines, definitions, profiles or settings
invalidate the cache transactionally. The next query rebuilds documents and
dashboard from authoritative journal/audit state in one IMMEDIATE transaction.
Unchanged-cache pagination does not replay the journal. Refresh failure rolls
back cache changes and leaves the dirty marker for a later retry; source stock
remains usable and unchanged.

Explicit rebuild recreates the FTS index and refreshes documents. Missing cache
tables/marker rows are recreated from migration 007's idempotent cache DDL.
Rebuild does not modify domain history or entity identities. Snapshot/restore
must treat these tables as derived caches rather than an inventory source.

Queries currently use the SQLite write gate even when clean; filtered totals
scan/decode all matching cached rows. Full refresh still replays unoptimized
history. Incremental projection maintenance, concurrent read scaling and measured
50k/100k growth behavior remain future work. No capacity claim is made.

Fresh databases install schema 7. Earlier schemas retain their existing workflows
without automatic upgrades; search/dashboard explicitly require schema 7. Earlier
migrations remain unchanged. Safe upgrade/restore coordination is still pending.

## Validation and remaining work

87 behavioral tests pass with rustfmt/clippy. New cases cover exact/FTS queries,
typed filters/subtrees, deterministic pagination and all-match totals, literal
operator/SQL input, metadata refresh, archive retention, missing-cache rebuild,
failure rollback, closed/reopened residue, actual set counts, unknown copy lines,
set storage/assembly inheritance, overlapping facets, exact sums above u64,
pending/recent purchase views and schema-6 compatibility.
The demo ran successfully: two matches for 3001 in Drawer 14, one displayed row,
150 pieces across matches, two unknown weights, and unchanged totals after
reopening/rebuilding the index.

Inventory archive, catalog-only/global non-stock search, purchase dates and their
filters, shipping allocation, attachments, received-purchase reconciliation,
backup/restore, safe upgrades, API and browser UI remain pending.
