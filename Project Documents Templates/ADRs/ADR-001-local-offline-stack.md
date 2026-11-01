# ADR-001: Local offline stack and ownership

Date: 2026-10-31. Status: Accepted user choice. Source decisions: D-086, D-092, D-118.

## Context

The user requests a durable pre-code foundation for a long-lived offline collection, ordinary-file portability and growth beyond the initial collection size. This ADR refines the existing alpha charter; deferred business features stay deferred.

## Decision

Rust/Axum REST/JSON backend, Svelte/TypeScript browser UI, SQLite and ordinary filesystem storage. Offline first, single user first, one active machine first; multi-user/server deployment later. Kubuntu 26.04 LTS, Firefox and Chromium remain the alpha boundary.

## Design and constraints

Core assets are bundled locally. No GraphQL, gRPC, WebSockets-first architecture, Tauri shell, account/tenant system or distributed sync infrastructure in alpha. Multiple tabs still need stale-edit protection. Existing 90-second last-tab shutdown and independent backup scheduling remain.

## Consequences

No network dependency for core workflows; no cross-machine writers. A backup destination on NAS does not mean a live database on NAS.

## Verification

Implement relevant contract tests from TESTING.md before the first release. No runtime behavior is claimed by this ADR. User-selected architecture is accepted; details marked draft are engineering proposals to validate, not retrospectively attributed user answers.
