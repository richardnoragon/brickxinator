# Brickxinator roadmap

Current: 2026-11-23, source candidate `0.1.0-alpha.1`, schema 18. No deadline or
published release. [ALPHA_STATUS](../ALPHA_STATUS.md) records dated evidence.

## Implemented candidate

- Inventory journal, unknown-aware measures, source provenance, damage, storage,
  classification, search/dashboard, managed attachments and catalog/settings.
- Copy lifecycle/disposition/corrections; physical splitting, compatible merging
  and independent detachment/movement with history.
- Versioned projects/BOMs, reservations/allocations, verified readiness, build
  units, shortages, attributed price observations and procurement exports.
- Partial fulfilment, shipments/backorders, receipt reversals, supplier returns,
  independent warranties, gross/discount/payment/refund workflows.
- Seven shipping modes, separate fees, holding/direct-copy cost allocation,
  residual/disposed budgets, reviewed reallocations and cost rollups.
- LDraw/MPD, Studio-export and Rebrickable file adapters, source caching, color/
  theme enrichment, stable theme links, manual images, Wanted List XML and Markdown.
- Native exchange, checked migration/restore, snapshots, Restic and opt-in scheduling.

Latest feature checks: 184 Rust tests and 18 Chromium/Firefox scenarios. Earlier
real process/Restic drills retain separate dates; desktop sign-off is not implied.

## Remaining expanded-alpha acceptance

1. Dedicated quantity-provenance reconciliation after ambiguous mixed-source
   operations; unsafe returns currently block instead of inventing source quantities.
2. Consistent presentation/integration of legacy set verification and newer project
   allocations/completeness, including more cross-workflow acceptance coverage.
3. Audited reassignment of a copy to a different catalog definition.
4. Project priority/footprint refinements and richer per-entity Obsidian packaging.
5. Supported Kubuntu desktop/launcher/clipboard review, final dependency/content
   review and maintainer release approval.
6. Final private security-reporting and community enforcement contacts (explicitly
   deferred until before release/public participation).

## Beyond alpha / deliberately deferred

- Per-piece assembly and instruction-step tracking; bag part checklists.
- CAD/geometry editing, rendering/instruction generation, track editing/simulation
  and authored MOC geometry (external tools remain responsible).
- Native Studio `.io` archives (Studio exports satisfy alpha), Obsidian import/
  plugins, live sync, marketplace login/cart/order placement and real-time pricing.
- Online catalog/image retrieval, compressed/full-bulk streaming adapters and
  automatic collection/profile synchronization.
- Installable Kubuntu packages, other platforms, LAN/cloud/NAS live server mode,
  multi-user and multi-collection operations.

NAS/external backup destinations do not imply support for a live database on NAS.
Syncing the source review branch does not authorize merge, tagging or publication.
Earlier implementation milestones are retained in the changelog, ADRs and
ALPHA_STATUS rather than presented as current missing features.
