# Brickxinator documentation

Current source candidate: `0.1.0-alpha.1`, schema 18, updated 2026-11-23.
The local offline application includes inventory, lifecycle, projects, purchase
fulfilment/aftercare, financial adjustments, derived costs/rollups and reviewed
file/catalog enrichment. No release/tag is published.

Start with the [root overview](../README.md), [installation](../INSTALL.md),
[user guide](USER_GUIDE.md) and [current evidence/limits](../ALPHA_STATUS.md).
[ADR-023](ADRs/ADR-023-expanded-alpha-ledgers-and-enrichment.md) reconciles the
expanded implementation with earlier decisions. [DECISIONS](DECISIONS.md)
retains the user's dated policies, through D-195.

Physical holdings represent ownership; copy/project allocations reserve stock.
Assembly, storage and physical verification stay separate. Financial adjustments,
settlement, derived acquisition cost and market observations are separate flows.
All source histories and UUIDs survive audited corrections.

The [documentation index](TOC.md) identifies operational documents and preserved
history. Discovery, original napkin notes, generic versioning templates and dated
ADRs retain their original context; they are not current feature-status lists.
The executable schema is [migrations](../migrations), not the historical table
sketch. The folder's original name is retained for stable links.

Platform: Kubuntu 26.04, Firefox/Chromium, one local user. Source installation and
Linux local Docker preview are documented; native packages, LAN/cloud serving
and multi-user operation remain deferred. Backups start disabled and require
reviewed activation. Reporting/release gates remain in ALPHA_STATUS.md.

Code: [GPL-3.0-or-later](LICENSE.md). Third-party content retains its own rights.
