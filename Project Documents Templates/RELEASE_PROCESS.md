# Brickxinator release process

Candidate evidence (2026-11-01) is recorded in [ALPHA_STATUS](../ALPHA_STATUS.md).
Source-build/backup commands are in [INSTALL](../INSTALL.md). The source review
branch is not a published alpha release. Headless browser checks and real backup
restores pass; Richard's Kubuntu desktop acceptance/release approval and private
reporting/community contacts remain gates. Public repository creation does not
authorize merging, tagging or publishing a release.

## Historical specification

Status: requirements and release gates; no release has occurred. First target: 0.1.0-alpha.1 (D-115).

## Versioning

Use semantic version identifiers with alpha/beta/rc prerelease suffixes. Apply the applicable requirements of the existing ../Versioning/Versioning  Standard  Policy.md: per-release changelog with commit references, ordered schema migrations with recovery paths, versioned public API if introduced, and identifiable immutable artifacts if produced. Container/API requirements do not imply adding those deliverables to alpha.

The exact runtime/build metadata and CI commands are defined with implementation tooling. Dependency versions and toolchain are pinned/documented for source builds. A database schema version is separate from the application prerelease identifier.

## Release sequence

1. Freeze intended alpha scope, reconcile documents/decisions, identify known limitations and obtain a clean candidate from reviewed main.
2. Run the applicable TESTING.md acceptance/regression suite, formatting/lint/type checks and source-build validation.
3. Verify Firefox and Chromium on Kubuntu 26.04 LTS, launcher shutdown and offline operation.
4. Demonstrate daily external incremental restore and independent monthly ZIP restore with attachments and configuration; verify upgrade migration, safety backup and recovery after failure. An untested restore blocks release.
5. Review dependency/content licenses, reporting destinations, documented installation/backup setup and known data risks. Do not include personal collection data in artifacts.
6. Prepare changelog with real commit references and the actual candidate identifier. Richard reviews evidence and approves the release before tagging/publication. No automatic publication is authorized by discovery.
7. Publish/tag only after approval; document actual validation and limitations. Do not invent a beta/RC trial duration; a trial period can be chosen when candidate evidence exists.

## Mandatory acceptance gates

Record/receive a purchase with images and unknown data; find stock by number/location; gradually identify a mixed lot; verify set contents and change assembly state without duplication. No negative inventory or over-extraction. Absolute counts, audited corrections, partial damage, invoice-line shipping and exact rounding work. Both browsers, backup restore and migration recovery pass.

## Installation and support

Alpha source build is sufficient; provide exact commands once implemented plus launcher and OS backup setup. Kubuntu packages are deferred. Support/report contacts must be set before public release; no unsupported-version promises are invented here.

## Rollback and data safety

Use ordered migrations with an explicit recovery path, tested against disposable copies. For a destructive migration, a validated full pre-migration backup and documented restore path are required; do not assume down-migrations can reconstruct lost data. Restore data/attachments consistently and validate before resuming writes. Release evidence must show the chosen recovery path works.

Technical freeze gates: verify WAL/FKs on every connection and supported FTS5 runtime, movement replay/projection equivalence, immutable migration checksum detection, UUIDv7/JSON identity round-trip, filesystem manifest integrity and REST request idempotency/conflicts. No source release may claim daily backup readiness from ZIP export alone; at least one external incremental integration must be configured and restore-tested.
