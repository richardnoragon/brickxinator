# Project documentation

Current implementation status is in the [root README](../README.md),
[alpha audit](../ALPHA_STATUS.md), [installation guide](../INSTALL.md) and
[dependency inventory](../THIRD_PARTY.md). Historical discovery/design documents
remain below; later ADRs and decisions supersede earlier planning text. The existing folder name is retained; contents are project specifications rather than empty templates.

## Start here

- [README](README.md): purpose, confirmed alpha scope, platform and status.
- [DOMAIN_MODEL](DOMAIN_MODEL.md): entities, relationships, quantities and state invariants.
- [ARCHITECTURE](ARCHITECTURE.md): approved stack, boundaries, transactions and remaining technical selections.
- [Architecture decision records](ADRs/README.md): design and implementation decisions through ADR-023.
- [DATABASE_SCHEMA](DATABASE_SCHEMA.md): implemented migrations 001–018, typed history contracts and historical table sketch.
- [RUST_STRUCTURE](RUST_STRUCTURE.md): implemented three-crate workspace and preserved historical plan, domain modules, REST API and implementation sequence.
- [ROADMAP](ROADMAP.md): implementation sequence and deferred scope.
- [USER_GUIDE](USER_GUIDE.md): daily workflows and current alpha controls.

## Quality and operation

- [TESTING](TESTING.md): acceptance examples and integrity/recovery gates.
- [SECURITY](SECURITY.md): local access, files, backups, recovery and reporting status.
- [RELEASE_PROCESS](RELEASE_PROCESS.md): versioning, release approval and mandatory gates.
- [CONTRIBUTING](CONTRIBUTING.md): branches, reviews and checks.
- [CODE_OF_CONDUCT](CODE_OF_CONDUCT.md): proposed community policy; enforcement setup pending.

## History and reference

- [DECISIONS](DECISIONS.md): dated decision record; later decisions resolve earlier open questions.
- [DISCOVERY](DISCOVERY.md): all 15 rounds, answers and readiness outcome.
- [FAQ](FAQ.md): practical answers and known limits.
- [CHANGELOG](CHANGELOG.md): unreleased source changes; no software release yet.
- [LICENSE](LICENSE.md): GPL-3.0-or-later notice and third-party rights boundary.
- [COPYING](../COPYING): canonical GPLv3 license text.
- [Original napkin notes](../brickxinator_napkin.md): preserved source ideas.
- [Existing versioning policy](../Versioning/Versioning%20%20Standard%20%20Policy.md): repository versioning reference; applicable requirements addressed by release process.

## Readiness

The source candidate is implemented and running locally with schema 18. Latest
feature evidence is 184 Rust tests and 18 browser scenarios. Remaining expanded
acceptance, desktop review/reporting/release gates are in ALPHA_STATUS.md.
Discovery notes, older ADRs and versioning templates retain their actual dates;
ADR-023/current operational documents supersede old feature deferrals. Backups
start disabled and require explicit reviewed activation. No release/tag exists.
