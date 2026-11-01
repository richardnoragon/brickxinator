# ADR-019: Local application, managed files and recovery coordinator

Date: 2026-11-01. Status: implemented within the accepted alpha scope.

## Decision

The third workspace crate is an Axum application serving built Svelte/TypeScript
assets from the checkout. It binds 127.0.0.1 only. Host must match the advertised
origin; foreign Origin/cross-site requests are rejected, and mutations require
the process token plus matching Origin. The token is obtained through same-origin
bootstrap. CSP, nosniff, no-referrer and no-store headers apply to app responses.
The local service supplies audit actors from USER, with a local-user fallback.
Read integers beyond JavaScript's exact range become decimal strings; editors
reject unsupported large integer inputs rather than rounding them.

App tabs heartbeat every ten seconds. A tab expires after thirty seconds without
a heartbeat; an empty live-tab set receives a ninety-second idle grace. A runtime
instance identifier and OS lock support reusing an existing service for the same
data directory. The launcher invokes xdg-open only when requested.

Each synchronous domain request runs off the async reactor and holds a Store
lease only for that request. Shared/exclusive OS file locks coordinate Store
access and maintenance without expiring leases. No file lock claims to protect
against an external program bypassing the application and editing SQLite directly.

## Files and snapshots

Migration 009 adds immutable attachment records, explicit target associations
and command audit. Managed UUID filenames never use the original filename as a
path. PNG/JPEG/GIF/WebP/PDF signatures are allowed, bounded to 64 MiB; SVG/HTML
uploads are rejected. Originals are copied unchanged. A staged, synced file is
published before its database reference; failure may leave an unreferenced file
but never deliberately publishes a missing reference. Generated filename/type,
size/hash, metadata and association audit are cross-checked during validation.

Snapshots hold exclusive maintenance, use SQLite's backup API, copy referenced
assets and optional JSON configuration, and produce a versioned SHA-256 manifest.
Publication is atomic and non-clobbering after validating the resulting archive.
Validation rejects duplicate/unsafe/symlink/unlisted paths, unsupported versions,
size/hash mismatches and invalid database/audit relationships. Hashing large
database payloads streams through a bounded buffer. Limits are in ALPHA_STATUS.md.

Restore validates first, creates a validated safety ZIP, checkpoints SQLite and
uses a synced rollback journal plus preserved database/assets before installation.
Normal opens refuse a pending restore journal. Recovery rolls back an incomplete
installation or validates a committed installation before cleaning rollback data.
Restored configuration is copied to a separate review file and never activated.
Upgrades use the same gate and pre-upgrade snapshot, then ordered transactional
migrations and validation. Historical migration checksums are never rewritten.

Backup/export/maintenance commands refuse a missing database. Automatic startup
also refuses a nonempty data directory without its database, so an unattended
backup cannot silently protect a newly invented empty collection after data loss.

## Evidence and limits

Tests cover HTTP defenses, exact values, liveness, real-process launcher reuse and
crash restart, file associations, snapshot corruption/traversal/future formats,
interrupted installation stages, safety undo, inactive configuration and migration
failure rollback. Chromium/Firefox run the daily workflows and offline assets.
Snapshot compression/validation currently holds the maintenance gate throughout;
large-collection maintenance latency remains unmeasured. Desktop integration and
native OS clipboard sign-off remain part of Richard's supported-desktop review.
