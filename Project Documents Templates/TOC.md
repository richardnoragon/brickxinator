# Project documentation

Current implementation status is in the [root README](../README.md),
[alpha audit](../ALPHA_STATUS.md), [installation guide](../INSTALL.md) and
[dependency inventory](../THIRD_PARTY.md). Historical discovery/design documents
remain below; later ADRs and decisions supersede earlier planning text. The existing folder name is retained; contents are project specifications rather than empty templates.

## Start here

- [README](README.md): purpose, confirmed alpha scope, platform and status.
- [DOMAIN_MODEL](DOMAIN_MODEL.md): entities, relationships, quantities and state invariants.
- [ARCHITECTURE](ARCHITECTURE.md): approved stack, boundaries, transactions and remaining technical selections.
- [Architecture decision records](ADRs/README.md): design and implementation decisions through ADR-022.
- [DATABASE_SCHEMA](DATABASE_SCHEMA.md): table/field draft, movement ledger, indexes and exchange/snapshot contracts.
- [RUST_STRUCTURE](RUST_STRUCTURE.md): three-crate workspace plan and current implementation reference, domain modules, REST API and implementation sequence.
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

Functional and technical architecture foundations are documented before coding. Exact dependency pins/executable DDL, external incremental adapter and file limits remain engineering selections. Full ZIP coordinator plus mandatory external daily incrementals is fixed; no embedded backup engine. Reporting destinations and conduct enforcement must be set before public participation/release. No application or scheduled backup has been created during discovery.
