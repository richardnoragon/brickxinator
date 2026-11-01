# ADR-007: SQLite FTS5 and structured indexes

Date: 2026-10-31. Status: Accepted user choice; index layout is draft. Source decisions: D-124.

## Context

The user requests a durable pre-code foundation for a long-lived offline collection, ordinary-file portability and growth beyond the initial collection size. This ADR refines the existing alpha charter; deferred business features stay deferred.

## Decision

Use SQLite FTS5 for names, descriptions, notes, sellers and manufacturer text; normal relational indexes for numbers, locations, dates, ownership/availability filters. No external search service.

## Design and constraints

Maintain a rebuildable local search-document/FTS projection with internal integer rowid mapping to UUID entities. FTS internal rowids are not domain IDs. Changes commit consistently with source metadata or are rebuilt before reads are exposed. Parameterize FTS input and apply limits/pagination. Exact identifiers must work independently of tokenization.

## Consequences

Index archived records too; normal views hide archive clutter while explicit search can include it. Compare search rebuild results and bound query work as data grows. FTS is not the source of inventory totals.

## Verification

Implement relevant contract tests from TESTING.md before the first release. No runtime behavior is claimed by this ADR. User-selected architecture is accepted; details marked draft are engineering proposals to validate, not retrospectively attributed user answers.

## Primary reference

[SQLite FTS5 documentation](https://www.sqlite.org/fts5.html).
