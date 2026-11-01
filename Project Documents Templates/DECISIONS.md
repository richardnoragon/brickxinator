# Brickxinator decision record

## Implementation clarification — 2026-11-01

| ID | Confirmed decision | Consequences |
| --- | --- | --- |
| D-129 | A set invoice line with unknown physical copy count may be received with its count remaining unknown; create no invented copies. | Explicit answer to implementation clarification. Existing copies are not inferred from brick count or price. Adding later-discovered copies requires audited receipt reconciliation, not another receipt. See ADR-012. |

Discovery started: 2026-10-31. Source: [original napkin notes](../brickxinator_napkin.md).

## Recording rules

Confirmed entries below come from explicit answers in the napkin notes or subsequent user answers. Suggestions in the napkin notes are not approved technology choices. Product ambitions do not imply alpha commitments. Later changes will retain the previous decision and record what supersedes it, why, and the affected documents. Unanswered questions remain open.

## Confirmed baseline from the napkin notes

| ID | Decision | Source | Implication / unresolved detail |
| --- | --- | --- | --- |
| D-001 | Priority: inventory tracking, collection management, set management, train layout planning; remaining features follow without a fixed order. | A01 | Alpha boundary still open. |
| D-002 | Initially for personal use; intended to be open source on GitHub. | A02 | License and contribution policy undecided. |
| D-003 | Offline first, with cloud optional. | A03 | Cloud timing and deployment model undecided. |
| D-004 | Offer part number, description, color, owned/available/assigned quantities, storage, condition, purchase price and source; no user-entered field initially required. | A04 | Blank values, internal identity and quantity rules require clarification. |
| D-005 | Physical storage locations are defined in settings to prevent mistyped names. | A05 | Hierarchy, movement and deletion rules open. |
| D-006 | Track LEGO, BlueBrixx, Xingbao, Mould King, CaDA, Cobi, Rebrickable custom parts and generic clones. | A06 | Cross-brand identity and substitution rules open. |
| D-007 | Set information includes number, name, manufacturer, release year, theme, piece count, original/PDF instructions, inventory and build status. | A07 | Alpha subset and attachment handling open. |
| D-008 | Multiple owned copies of a set have separate identities and individual metadata. | A08 | Example repeats 7838_02; exact identifier format is unconfirmed. |
| D-009 | Set decomposition into bags, subassemblies and parts is deferred. | A09 | Relationship between sets and loose inventory still open. |
| D-010 | Long-term MOC sources include Rebrickable, Eurobricks, BrickLink, BlueBrixx community, GitHub and local files. Compare missing/available/alternative parts, cost and alternative colors; support authoring. | A10–A12 | Alpha inclusion and integration feasibility unconfirmed. |
| D-011 | Long-term full CAD and .ldr, .mpd, .dat, Studio .io support; Studio compatibility is an intermediate goal. | A13–A15 | Alpha inclusion and implementation unconfirmed. |
| D-012 | Long-term train planning covers LEGO 9V, RC, Power Functions, Powered Up, BlueBrixx and custom systems. Grid/free placement is selectable; switching reports inconsistencies for user resolution. | A16–A17 | Alpha inclusion unconfirmed. |
| D-013 | Layout ambitions include tracks, buildings, terrain, signals, stations, roads, wiring and electrical blocks; calculations include length, part count, cost, radius and clearance. | A18–A19 | Release allocation remains open. |
| D-014 | Custom parts include 3D prints, stickers and modified track; settings manage additional categories. Support official images, renders and user photos. | A21–A22 | Parts catalog sources (Q20) unanswered. |
| D-015 | Downloads/updates are manually initiated. | A23 | Update content (Q24) unanswered. |
| D-016 | Expected initial collection is fewer than 10,000 parts. Initially one user and one collection, with later expansion possible. | A26–A27 | Meaning of part count and performance targets open. |
| D-017 | Develop requirements through repeated question/answer rounds, record decisions, and establish a sufficient foundation before alpha coding. | User instruction, 2026-10-31 | No application implementation started. |

## Open technology and policy choices

Rust, Tauri, Svelte, SQLite/PostgreSQL, Bevy/Three.js, plugins and the suggested module layout were recommendations in the notes, not accepted decisions. Versioning reference documents exist in ../Versioning; their project-specific adoption and applicability remain to be resolved. License, supported platforms, backup guarantees and release gates remain open.

## Subsequent decisions

### Round 1 — 2026-10-31

Source: user answers to discovery round 1.

| ID | Decision / evidence | Consequences and open points |
| --- | --- | --- |
| D-018 | Current workflow uses Obsidian to record purchases, mostly from eBay, by copying pictures and manually entering values such as part number and built/loose state. Existing material is Markdown and pictures. | Purchase entry and image handling need detailed requirements. Import necessity, structure and volume are unconfirmed. |
| D-019 | Alpha includes manual part inventory, search/filter, settings-managed storage, individually tracked set copies, local persistence and backup/restore. CAD, train planning, MOC authoring and online integrations follow later. | Resolves alpha-boundary questions in D-001, D-010–D-013; broader ambitions remain. Detailed behavior and acceptance criteria still open. |
| D-020 | Kubuntu is the only required alpha operating system. | Desktop versus local-browser delivery and supported Kubuntu versions remain open. No cross-platform alpha commitment. |
| D-021 | Set contents contribute to total owned parts; assembled parts are unavailable. Build reservations are deferred beyond alpha. | Resolves set-accounting direction from D-009. Unknown set contents, disassembly and avoiding duplicate counts require rules. |
| D-022 | This is a hobby project driven by functionality, with no deadline or prescribed technology, hardware or budget constraint beyond Kubuntu. | Specific functional acceptance examples remain unanswered; stack selection is still open. This does not authorize expenditure. |


### Round 2 — 2026-10-31

Source: user answers to discovery round 2.

| ID | Decision | Consequences / open details |
| --- | --- | --- |
| D-023 | An alpha purchase may contain multiple sets and/or loose-part items, with shared seller, purchase date and shipping costs, and individual item prices. This is sufficient for alpha. | Currency, optional values, allocation of shipping and editing behavior still open. |
| D-024 | Manual entry is acceptable for alpha; importing Obsidian notes and pictures is deferred. | Existing source notes need not be migrated automatically. Manual image attachment behavior still needs definition. |
| D-025 | A set may be saved before individual contents are known. Show its stated piece count separately from identified parts; do not invent individual part quantities. | Detailed set-content entry method remains open; manual entry practicality was not explicitly answered. Totals must distinguish known quantities from incomplete information. |
| D-026 | Alpha uses a whole-set assembled/loose switch. Dismantled parts remain linked to their set copy and become available. | Refines D-021; partial assembly and bag/subassembly decomposition are out of alpha scope. Changing state must not create duplicate stock. |
| D-027 | Identical parts may be split across purchases and storage locations, with combined quantities in search. Unknown quantities, colors and manufacturers may remain blank. | Identity matching, treatment of unknowns in sums and per-lot stock movement need definition. Blank quantity is not an approved synonym for zero. |
| D-028 | Alpha is a local browser application on Kubuntu 26.04 LTS. | Resolves delivery model left open in D-020; backend language, browser support, start/stop behavior and network binding remain undecided. |

### Round 3 — 2026-10-31

Source: user answers to discovery round 3.

| ID | Decision | Consequences / open details |
| --- | --- | --- |
| D-029 | Alpha accepts sets, loose-part batches and unidentified mixed lots. Set contents can be entered in a part-number/color/quantity table and copied from another copy of the same set. These fields remain editable later. | Copy must represent another owned copy, not alias the original stock. Exact edit/reconciliation behavior remains open. |
| D-030 | Part identity uses manufacturer and part number; stock is distinguished by color and condition. Entries lacking identifiers remain separate until manually linked. Unknown-brand parts remain separate from branded parts. | Manual linking and correction must preserve inventory and purchase provenance. Color vocabulary and condition choices remain open. |
| D-031 | Inventory may be measured by weight instead of pieces; unknown piece quantities must not be represented solely as unknown-count batches. | Revises the proposed round-3 totals approach, which was rejected. Units, dual measurements, aggregation and availability of weighed stock remain open. Never infer piece counts from weight without an approved rule. |
| D-032 | Nested storage locations are allowed; check for misspellings. No splitting of a set copy or loose-part batch across locations in alpha. | Name checking behavior, hierarchy depth and rename/delete/move rules remain open. |
| D-033 | Support multiple image attachments, clipboard image pasting, free-text notes and optional source/eBay URL at both purchase and individual-item levels. | Interprets “yes, and yes” as accepting the proposed fields and both attachment levels. File management and copy behavior still open. |
| D-034 | Alpha currency is EUR. Shipping must be shared and belong to the purchase. | Purchase ownership confirmed; “shared” allocation rule needs clarification. No allocation formula is approved. Optional-price question was not explicitly answered. |

Unanswered from round 3: interface language and whether prices may be blank. Neither is inferred from the currency answer.

### Round 4 — 2026-10-31

Source: user answers to discovery round 4, including expanded shipping requirements.

| ID | Decision | Consequences / open details |
| --- | --- | --- |
| D-035 | Piece count and weight are independently optional. Accept grams or kilograms; never automatically convert weight into pieces or vice versa. | Refines D-031. Validation, display precision and weight normalization remain to be specified. |
| D-036 | Show piece and weight totals separately. Both measurements on one entry describe the same stock. Keep unidentified mixed-lot weight separate from identified-part totals. | Measurements must not be counted as separate holdings. Partially measured inventory needs completeness indicators. |
| D-037 | Unknown information is acceptable; users must not be required to supply information they do not possess. Prices and shipping may be blank. Unknown shipping is distinct from known free shipping (zero). | Extends D-004 and resolves optional-price questions. Blank costs must not silently become zero in totals or allocation. Internal generated identity is still needed; completion and invalid supplied values are separate concerns. |
| D-038 | Shipping belongs to the purchase, with purchase-level-only treatment as the default. Support equal, proportional-by-item-cost and measurement-based allocations as alternatives. | Supersedes the unresolved allocation choice in D-034. Weight, piece count and volume are mentioned as measurement bases; volume fields, release allocation, None versus Order Level, rounding, missing basis and persistence of derived shares require clarification. Do not infer that volume tracking is already in alpha scope. |
| D-039 | Purchase record includes vendor, date, shipping, tax, currency, order number, notes and allocation method. | EUR-only alpha remains in force (D-034); a currency field does not imply multi-currency support. Tax inclusion, shipping allocation and purchase total calculations remain open. |
| D-040 | Storage uses a selectable tree, names managed in settings, and warnings for duplicate/similar names without silent correction. | Confirms the proposed name-checking approach; exact matching behavior remains an implementation choice to resolve. |
| D-041 | ALPHA RESTRICTION: a set copy and ALL linked contents share ONE storage location, including when the set is loose. Individual relocation and splitting are deferred. User explicitly requests this restriction be highlighted in notes and decisions. | Refines D-032 and D-026. Loose means available, not individually movable; moving the set must preserve a single shared location. |
| D-042 | Alpha interface language is English. | German/localization is not an alpha requirement. |

### Highlighted alpha limitation — storage

**A set copy and all its linked parts stay in one location, even after disassembly. Alpha does not support moving some linked parts to a different location.** Identical loose stock can still exist as separate batches in different locations (D-027); a single batch is not split across locations (D-032).

### Round 5 — 2026-10-31

Source: user-provided specification in response to round 5. Treat the requested recommendations as product requirements; unresolved inconsistencies are listed explicitly.

| ID | Decision | Consequences / open details |
| --- | --- | --- |
| D-043 | Alpha shipping modes: Purchase Level, Equal, Price Proportional, Piece Count, Weight and Manual. Merge None/Order Level into Purchase Level; this is the default, records shipping once in order totals and computes no item share. Volume allocation is deferred. | Resolves release allocation in D-038. |
| D-044 | Recalculating allocation must preserve source purchase prices. Only allocated shipping and effective costs change. | Correction of mistaken source data versus immutability, and price-history scope, require clarification with D-029. |
| D-045 | Missing allocation inputs must produce an explicit incomplete/cannot-calculate status; never invent inputs. Manual shipping allocation is required. | Invalid known inputs, all-zero bases, manual-share validation and incomplete drafts need rules. Missing information alone must not block saving. |
| D-046 | Equal allocation is per purchase row. Allocated shares must sum exactly to purchase shipping, including remainder cents. | Deterministic remainder ordering and proportional rounding algorithm remain open. Manual shares must also reconcile, but mismatch handling remains open. |
| D-047 | Store optional row quantity and total row price; unit price is derived. Source price represents the actual price paid. Tax information is optional, including included-tax information and unknown tax. | Do not invent unit prices when quantity is unavailable. Unit-price precision and weighted-lot unit basis remain open. |
| D-048 | Purchase summary shows item subtotal, shipping, tax information and grand total; incomplete amounts are flagged without blocking record creation. Shipping must not be counted twice. | User also gives “+ Tax (if known)” despite included-tax examples; exclusive-tax support and total formula need clarification before implementation. |
| D-049 | Discounts, coupons, refunds, partial refunds, returns and warranty claims are deferred. User suggests v1.1 as a later target. | No v1.1 release schedule or scope otherwise established. Record historical final paid prices without adding a discount workflow in alpha. |
| D-050 | User lists price history in alpha and requests manual overrides, correct totals and non-blocking warnings for missing information. | Price-history meaning, override boundaries and correction audit behavior remain open. Manual overrides cannot silently contradict required shipping reconciliation. |

### Open consistency questions after round 5

- Included tax must never be added twice; confirm whether separately added tax is needed at all.
- “Original purchase data is never modified” must be reconciled with editable fields: correction history versus immutable records.
- Missing information is permitted; mathematically invalid supplied information requires a separate validation policy.

### Round 6 — 2026-10-31

Source: user answers to discovery round 6.

| ID | Decision | Consequences / open details |
| --- | --- | --- |
| D-051 | Entered prices are authoritative amounts actually paid. Tax is optional informational metadata with Included, Unknown, Not Applicable and Added Separately states. Never add tax again to paid prices. | Resolves D-048: grand total uses paid row totals plus shipping; a separately shown tax amount does not authorize adding it again. Net-price entry/reconstruction is not required. |
| D-052 | All corrections are timestamped and preserve prior values. Alpha provides per-item historical purchase prices across purchases. External market-price history is deferred. | Resolves D-044/D-050: user corrections are permitted and audited; allocation cannot overwrite source prices. Audit coverage, deletion and stock-versus-acquisition history still need detailed rules. |
| D-053 | Manual allocation may be saved as a draft with unbalanced/missing shares; show the remaining amount. Effective allocated costs are unavailable until allocation is complete. Software never silently changes manual shares. | Original known row prices remain usable even when allocated effective costs are unavailable. Overshoot display and validation are implementation details to specify. |
| D-054 | Automatic allocation remains incomplete for missing data or a zero total basis. Use largest-remainder rounding in cents with stable row-order tie breaking. | Refines D-045–D-046. Identical one-third shares of EUR 1 become 0.34, 0.33, 0.33 in stable order. |
| D-055 | Derive EUR/piece and EUR/kg only from available positive measurements; retain more internal precision than displayed. Display precision is user-selectable. | Rounding of payable amounts remains cents. Exact derived precision and source-price versus effective-cost labels remain open. |
| D-056 | Unknown values are allowed; invalid supplied values must be corrected or cleared. Reject negative counts/weights and malformed numeric values. Allow otherwise blank draft records with generated IDs. | Distinguish draft completeness from validity. No fabricated quantity, seller, price or shipping is needed to save. |
| D-057 | Purchases include optional seller type and country. Requested types: Private Seller, Business Seller, eBay Private Seller, eBay Business Seller, Marketplace, Retail Store, Brick Dealer, Unknown. | Preserves requested types despite overlapping concepts. Controlled values/customization and seller reuse remain open. Future country/source/reliability analytics are motivations, not committed alpha reports. |

Tax note: “Added Separately” describes how tax was presented on the source invoice. Actual amount paid remains authoritative; it is not an instruction to add tax twice.

### Round 7 — 2026-10-31

Source: user answers to discovery round 7.

| ID | Decision | Consequences / open details |
| --- | --- | --- |
| D-058 | Purchase statuses: Draft, Ordered, Shipped, Received, Cancelled. Do not add purchase items to owned inventory until explicitly Received. | Receipt must not duplicate stock on repeat action. Partial shipments were cited as a motivation but partial receipt mechanics are not approved yet. Reversal/cancellation after receipt remains open. |
| D-059 | Original purchased quantities are immutable historical facts; current stock changes separately. Alpha adjustment types: Lost, Broken, Donated, Sold, Correction. | Expands alpha beyond entry corrections. Clarify append-only correction of mistakes versus immutable history (D-052); stock effects of Broken and adjustment targets remain open. Sale accounting is not implied. |
| D-060 | Alpha supports gradual mixed-lot decomposition into identified batches, preserving purchase link, source listing, cost, photos and seller. Prevent duplicate stock. | For known piece counts, source remainder plus transferred counts must equal original count, absent separate adjustments. Weight-only lots, child measurements, cost attribution and location behavior require explicit rules; no automatic weight/piece conversion (D-035). Set bag/subassembly decomposition stays deferred. |
| D-061 | Set copies have permanent internal IDs and editable display labels; initial labels such as LEGO 7838_01, LEGO 7838_02 are accepted. | Display-label changes must not change identity. Exact internal ID format is not mandated by the example. |
| D-062 | Search priority: part/set number, storage location, availability; then manufacturer, description, color, assembled/loose, incomplete records, purchase date, seller, condition. | User examples mention Reserved and Missing; build reservations remain deferred (D-021). Do not silently introduce reservation/planned-build functionality. Availability categories need definition. |
| D-063 | Open on a dashboard with globally available search. Suggested contents: set/part totals, recent purchases, awaiting-identification lots, pending deliveries, missing parts and recent activity. | Metric definitions and Missing Parts scope remain open; unknown counts and mixed units must remain explicit. |
| D-064 | Four alpha acceptance workflows: record a purchase with images and unknown fields; find stock by part/location; switch set assembled/loose with correct availability and no duplication; gradually identify a mixed lot without duplicate inventory. | Mixed-lot identification joins the core alpha boundary. Detailed test data and edge cases remain to define. |

### Alpha scope changes explicitly requested in discovery

The original manual-inventory scope now includes six shipping allocation modes, price/correction history, purchase receipt states, stock adjustments and mixed-lot decomposition. This is requested scope, not an implementation estimate. No deadline applies. Set subassembly decomposition, reservations, CAD, MOCs, train planning and online integrations remain deferred.

### Round 8 — 2026-10-31

Source: user answers to discovery round 8.

| ID | Decision | Consequences / open details |
| --- | --- | --- |
| D-065 | Preserve original purchase measurements in history. Identifying unweighed children from a weight-only lot makes residual weight unknown rather than estimated. Never silently estimate. | Distinguish acquisition measurement from valid current residual measurements; historical original weight must not be summed with extracted child stock as current inventory. |
| D-066 | Identified child batches inherit parent location, then may move as whole batches. A single batch cannot be partially split across locations. Set-linked parts remain with their set copy. | Clarifies D-032/D-041 without relaxing the set restriction. Mixed-lot identification is allowed even though arbitrary batch splitting is deferred. |
| D-067 | Original mixed-lot cost stays on the purchase row; identified child costs may be unknown. Internal lot-cost allocation is deferred. | This deferral does NOT remove the six approved purchase shipping allocation modes (D-043). Purchase acquisition cost must not be duplicated across child batches. |
| D-068 | Corrections append revisions, preserve historical values and expose the corrected current value. Whole-purchase receipt only in alpha. Partial receipts, partial shipments, backorders and receipt reversal are deferred. | Refines D-052/D-058/D-059. Correction reconciliation after receipt still needs a safe implementation model. |
| D-069 | Broken/Damaged stock remains owned but unavailable. Sold/Donated stock reduces current ownership. Reject reductions exceeding known stock; unknown stock requires identification/clarification before reduction. | Partially damaged batches require quantity-level usable/damaged accounting; damage does not imply a second storage location or new ownership. Lost remains an approved reduction type from D-059. |
| D-070 | Alpha availability filters: Available, In Assembled Set, Broken/Damaged, Unknown. Reservations and missing-parts analysis are deferred. | Supersedes the earlier three-category proposal with user's final fourth state. Assembly and damage can overlap, so filter precedence versus independent facets remains open. |
| D-071 | Alpha dashboard emphasizes Pending Deliveries, Unidentified Lots, Incomplete Records, Recent Purchases and Inventory Totals. Missing-parts metric is deferred. | Refines D-063; counters must respect receipt state, unknown measurements and no double counting. |

### Accounting scope clarification

**Deferred lot-cost allocation means dividing one acquired lot's price among newly identified child batches. Purchase shipping allocation remains in alpha.** The two operations must not be conflated.

### Round 9 — 2026-10-31

Source: user answers to discovery round 9.

| ID | Decision | Consequences / open details |
| --- | --- | --- |
| D-072 | Alpha accessible only from this computer through loopback, no login or LAN access. External listing links open only on explicit click. | Local mutation endpoints still require origin/CSRF safeguards; no remote account system implied. |
| D-073 | Support both Firefox and Chromium. Launcher starts local service and opens browser. Service must quit after browser closes. | Multiple tabs, browser crashes, shutdown grace period and backup scheduling while application is closed need explicit behavior. Do not assume browser-process monitoring or a permanent web service is approved. |
| D-074 | Automatic daily incremental backups. Automatic monthly backup on the first of each month, separate from and not replacing daily backups. | Destination, retention, full baseline/chain integrity, monthly full versus incremental, missed-run behavior, background scheduler and restore validation remain open. No unattended job has been created. |
| D-075 | Copy image attachments into application-managed storage, preserving source files. Accept PDF invoices and instructions in alpha. | Backup includes database and managed attachments. PDF/image viewing, file limits and integrity validation remain implementation details to decide. |
| D-076 | Records with stock/history links are archived and recoverable, retaining attachments. Empty never-received drafts may be permanently deleted. | Source: acceptance of proposed deletion behavior. Nonempty unreceived drafts and history of purged empty drafts need clarification if relevant. Archive should not implicitly remove stock; inventory effects need an explicit rule. |
| D-077 | Damage and assembly are independent facts. Damaged parts remain unavailable inside/outside sets; disassembly does not repair stock. | Broken/Damaged filter matches damaged holdings independently of assembly. Avoid counting overlapping unavailability twice. |

### Backup policy emphasis

**Daily incremental backups and first-of-month backups are separate schedules. A monthly run must never substitute for the daily run.** Retention and unattended scheduling are not yet specified; this records requirements only.

### Round 10 — 2026-10-31

Source: user answers to discovery round 10.

| ID | Decision | Consequences / open details |
| --- | --- | --- |
| D-078 | Auto-start local browser service; stop after last Brickxinator tab disconnects with a 60–120 second grace period. | Exact default still open. Refresh/reconnect cancels pending shutdown; browser crash is handled by loss of connected tabs. Always-on server mode is future scope. |
| D-079 | Backups run independently of application lifetime, preferably through OS scheduling. | Scheduler must work with web service stopped and coordinate consistent snapshots when running. No scheduler/tool has been installed or selected. |
| D-080 | Support simultaneous local-folder, external-drive and network-share destinations. Unavailable targets never block inventory use; show persistent warning and last successful backup per target. | Retry cadence and destination-local retention remain to specify. Never silently write into an absent mount's underlying directory. |
| D-081 | Retain 30 daily recovery points and 12 monthly snapshots. Monthly snapshots are full, self-contained and independently restorable. Preserve daily baseline/ancestors needed for retained points. | Daily incremental schedule remains separate from monthly schedule. Yearly retention is future, not alpha. |
| D-082 | Default daily backup time 03:00, user configurable. Run missed work at next opportunity, independently of opening UI. Monthly backup on first; both daily and monthly jobs run that day. | Exact timezone, coalescing missed days and OS catch-up semantics to finalize. Do not fabricate historical recovery points for days when no snapshot existed. |
| D-083 | Restore validates source, creates safety backup, restores database/attachments consistently, and validates result. Safety backup supports undoing an incorrect restore. | Interrupted restore recovery, configuration restore, safety-backup retention and corruption failure behavior need design. |
| D-084 | Archive hides daily-list clutter; record stays searchable, owned and included in inventory totals. Retain data/attachments and support unarchive. | Search UI must provide access to archived records without excluding stock from availability/totals. Archive is not a stock adjustment. |
| D-085 | Store data in ordinary Linux files/directories compatible with standard backup tools; include database, attachments/images, configuration and exports as applicable. Avoid a mandatory opaque custom backup format. | Exact directory layout, database and backup engine remain provisional. The SQLite/Rust diagrams are examples pending explicit stack confirmation. Ordinary file access does not make live database copying inherently consistent; snapshot coordination is required. |

### Readiness checkpoint after round 10

Core workflows, inventory semantics and backup policy are substantially established. Still required: explicit stack/license, detailed persistence/restore consistency, received-record correction effects, UI and schema specification, safe file handling, release gates and contribution/reporting policies. No coding or external publication has started.

### Round 11 — 2026-10-31

Source: user answers to discovery round 11.

| ID | Decision | Consequences / open details |
| --- | --- | --- |
| D-086 | Approved alpha stack: Rust with Axum API, SQLite, Svelte + TypeScript browser UI, filesystem attachments, OS-managed backups. No Tauri or PostgreSQL for alpha. | Supersedes provisional stack status. Dependency versions, migration tooling and backup engine remain unselected. Future platform portability does not expand Kubuntu-only alpha support. |
| D-087 | Daily backups may depend on established backup software. Monthly backups must be human-accessible, self-contained ordinary files/archives, recoverable without that specialized software. | ZIP containing database/attachments/config is an accepted example, not a mandated compression format. |
| D-088 | Shutdown grace is 90 seconds. Schedules use machine local timezone. Missed schedules create one current catch-up per due schedule rather than fictitious historical snapshots. | Both daily and monthly remain separate due schedules. |
| D-089 | Choose GPLv3 for project software. | Exact GPL-3.0-only versus GPL-3.0-or-later designation and copyright attribution remain open. Third-party content retains its own rights. |
| D-090 | Default data directory is $XDG_DATA_HOME/brickxinator, falling back to ~/.local/share/brickxinator. User configurable. Restored machine paths/backup destinations require review before reactivation. | Configurable path does not imply approval for concurrent multi-PC access or live SQLite on a network share; those deployment modes remain future/unresolved. |
| D-091 | Preview received-purchase correction effects and reject impossible/negative stock outcomes. Preserve all revisions. Receive once through an inventory movement; later corrections reconcile rather than receive again. | Validation must conserve source-to-child transfers: child stock is not an additional consumption of the same source. User's illustrative numerical warning is not a finalized accounting formula. |
| D-092 | All core functionality must work offline; online services can enhance but are never required to access/manage the collection, including local images. | Extends D-003. Future train-layout features inherit offline-first policy but remain outside alpha. No CDN/runtime network dependency for core UI. |
| D-093 | Move discovery toward concrete domain entities, relationships and schema rather than more general operational questions. | Domain draft is a proposal until relationship ambiguities are answered. No implementation authorized by a claim of percentage completeness alone. |

### Round 12 — 2026-10-31

Source: user answers to discovery round 12.

| ID | Decision | Consequences / open details |
| --- | --- | --- |
| D-094 | Reusable part/set/color/manufacturer definitions are separate from ownership. Definitions can exist with no owned stock; incomplete ownership can exist with unknown quantity/cost/location. | Confirms proposed catalog/ownership layers. Color remains a managed stock attribute per D-030; user's red-brick example does not by itself change part-number identity. |
| D-095 | Each physical set copy is independently tracked. A combined invoice price for several copies is preserved without inventing per-copy prices; individual prices stay unknown until explicitly allocated. | Requires shared price-bearing invoice line/group distinct from physical set copies. Set-copy-to-purchase-row one-to-one proposal is superseded. Shared-price allocation engine and shipping basis for grouped rows still need boundaries. |
| D-096 | Copying set contents creates an unverified expected checklist, not owned stock. Verification precedes actual ownership. Limited expected-versus-actual tracking is accepted in alpha. | Explicit scope change from earlier deferred auditing. Verify-to-stock mechanics, already-owned linking, checklist edits and partial verification remain open. Missing-parts dashboard/planning remains deferred unless separately approved. |
| D-097 | A batch can contain partial damage (100 owned, 2 damaged, 98 usable when otherwise available). Damage is independent of condition and assembly. Unknown damaged weight means usable weight is unknown where damage affects availability. | Confirms quantity-level damage accounting without storage splitting. |
| D-098 | Lot transfers subtract known extracted measures, invalidate affected residual measures when extraction measurement is missing, and never estimate. Reject extraction exceeding known remainder in the same unit. Classification completion is explicit. | Completion with known nonzero residue needs a rule; explicit completion must not erase measured stock. Unknown can become known only through explicit supplied measurement/verification, not inferred conversion. |
| D-099 | Manufacturer, color and condition lists are settings-managed and extensible. Initial condition values New, Used, Mixed; blank means Unknown. Damage remains separate. | Manufacturer examples include Custom/Unknown; canonical blank versus explicit Unknown representation and vocabulary rename/delete behavior remain to finalize. Online color imports remain later integration work. |

### Alpha scope change — verification checklists

**Expected set contents are not owned inventory.** Creating/copying a checklist adds no owned pieces. User explicitly accepts limited expected-versus-actual verification in alpha (D-096). This does not automatically enable global missing-parts analysis, MOC planning or build reservations.

### Round 13 — 2026-10-31

Source: user-supplied pasted text attachment answering round 13. Substantive answers preserved in DISCOVERY.md; original attachment remains outside the project.

| ID | Decision | Consequences / open details |
| --- | --- | --- |
| D-100 | Set verification is gradual, with Expected/Verified/Unverified. Unverified must never automatically mean Missing. User may explicitly complete verification before a local shortfall is labeled missing, if appropriate. | Limited per-set completion shortfall accepted; global missing-parts dashboard/planning remains deferred. Over-count and line-level completion semantics need precision. |
| D-101 | Alpha verification records actual contents of this set only. Reassignment from existing stock is deferred. | Verification creates/updates this set's actual owned stock but must not affect unrelated inventory. Prevent repeated counts from duplicating holdings; UI must warn against recounting already-recorded independent stock. |
| D-102 | Purchase → invoice line → independent owned copies is approved. One line for three copies can retain quantity 3 and total EUR 300 without per-copy prices. | Refines D-095. Shipping was asked to target invoice lines; answer explicitly confirms hierarchy but does not separately restate all allocation target details. Proposed line-level shipping model remains to confirm. |
| D-103 | Known nonzero lot remainder blocks classification completion. Unknown remainder can be closed by explicit “Confirm physical lot empty”, recording user confirmation and date, without inventing a measured zero. | Physical-empty state excludes residual holding from current stock while preserving unknown historical measurement. Explicit measured zero and confirmed emptiness are distinct facts. |
| D-104 | Receiving a set creates only its owned set copy. Parts remain expected until verified. Changing expected checklist quantity does not change stock; changing verified quantity creates an audited stock correction with timestamp, reason, user, previous and new values. | Refines D-058/D-096. Mandatory reason versus optional metadata remains to confirm. Local user identity does not imply introducing login. |
| D-105 | Manufacturer/color/condition have permanent IDs and renameable display names. Referenced values are archived, remain on historical records and are unavailable for new selection. Unknown is one real system-managed value, consistently displayed, not multiple blank/NULL/Unknown categories. | Supersedes provisional vocabulary storage choice in D-099. Applies to controlled categorical fields; numeric unknowns remain absent measurements rather than text “Unknown”. System Unknown should not be renameable/archivable; confirm as implementation rule. |

### Workflow boundary

Four daily workflows: buy, receive, sort/identify, build/disassemble. Proposed set sequence ends with assembly after verification, but whether completion is a prerequisite for assembly is not yet explicit. Do not impose a full-verification prerequisite without confirmation; used sets can arrive already built.

### Round 14 — 2026-10-31

Source: user answers to discovery round 14.

| ID | Decision | Consequences / open details |
| --- | --- | --- |
| D-106 | Assembly and verification are independent. An assembled received set may remain unverified. Verification states Not Started, In Progress, Completed. User also requests assembly labels Assembled, Loose, Partially Assembled. | Partially Assembled extends earlier whole-set binary scope (D-026). Availability effect requires confirmation; no per-part assembly tracking is approved. Detailed quantities count only actual verified stock. |
| D-107 | Checklist lines have explicit Finished flag. Expected 10 / verified 7 / unfinished means 3 unverified, never automatically missing. Finished permits Short 3. Verified 12 against expected 10 permits Excess 2. | Refines D-100. Null expected values and reopening finished lines remain to define. |
| D-108 | Verified entries are absolute totals, never additive. Reentering 7 preserves 7; changing 7 to 9 creates a +2 audited correction. | Repeated submissions must be idempotent, with conflicting edits handled explicitly. |
| D-109 | All shipping allocation modes target invoice lines, not set copies or individual parts. Equal sees a 3-copy set line as one line. Piece-based mode requires an explicitly known line brick count, never inferred from set-copy count. | Resolves D-102 allocation target. Weight basis likewise needs valid line measurement. No per-copy allocation added. |
| D-110 | Stock corrections require a reason and automatically record timestamp/local user/previous/new values. Descriptive edits may have optional reason. System Unknown manufacturer/color/condition cannot be renamed, deleted or archived. | Refines D-104/D-105; no login needed. User's example “damaged” negative adjustment must respect D-069: damage reduces usability, not ownership. |
| D-111 | Explicit physical-empty confirmation excludes residual lot holding from current inventory totals without rewriting historical measurements. Later findings use audited Reopen Classification preserving completion/reopen dates and reason. | Reopening does not itself invent quantities; supplied new measurements need conservation checks against existing history. |

### Remaining alpha state issue

Partially Assembled was introduced explicitly in round 14. Final choice pending: descriptive whole-set state with all contents unavailable, or actual per-part assembly allocation. The latter would expand earlier deferred partial set handling; it must not be inferred.

### Round 15 — 2026-10-31

Source: user answers to discovery round 15.

| ID | Decision | Consequences |
| --- | --- | --- |
| D-112 | Assembly states Loose, Assembled, Partially Assembled. Both assembled states make every linked part unavailable; Loose makes only otherwise usable verified contents available. | Resolves D-106. No per-part assembly allocation or build reservations in alpha. |
| D-113 | Overall verification completes only through explicit Complete Verification after every active line is finished. Expected/verified count changes reopen the affected line and overall verification, preserving history. | Reopening is automatic but visible and audited, never silent. Absolute count/idempotence rules remain in force. |
| D-114 | License GPL-3.0-or-later; copyright notice “Copyright (C) 2026 Richard”. | Resolves D-089. Project license does not cover third-party content automatically. |
| D-115 | Mandatory release gates: core workflows including receipt, inventory integrity/no duplicate or negative stock, audits/corrections, Firefox/Chromium, tested restore and migration safety. First prerelease target 0.1.0-alpha.1. | Target identifier is not a shipped release. Earlier find-inventory acceptance remains required alongside receipt. |
| D-116 | main plus short feature branches and reviewed PRs. rustfmt, clippy, TypeScript checks, behavioral tests and documentation updates. Conventional Commits preferred, not mandatory. | Exact commands added when source tooling exists. |
| D-117 | Source-build installation is sufficient for alpha; provide launcher and backup setup instructions. Installable Kubuntu package deferred. Bug/security report destinations explicitly pending repository selection. | No invented contact or URL; public release preparation must resolve reporting channels. |

### Discovery readiness assessment — 2026-10-31

The functional foundation is sufficient to begin alpha implementation planning and coding: scope, entities, key transitions, integrity rules, deployment, backup requirements, licensing and release gates are established. No further general questionnaire is required before coding. Technical design choices can be recorded as implementation decisions with rationale; they are not silently presented as user decisions.

Remaining nonblocking design work: choose dependency versions and backup engine, finalize SQL constraints/indexes and exact measurement precision, attachment limits/viewer, UI details and performance targets. Backup consistency/restore correctness are mandatory implementation and release gates. Reporting destinations must be resolved before public release. Long-term 100,000+ parts is a growth goal, not a measured alpha capacity claim. No application has been built, tested, released or scheduled during discovery.

### Pre-code technical architecture freeze — 2026-10-31

Source: user technical-foundation request and explicit follow-up on daily incrementals. Supersedes earlier statements leaving these choices unselected. Accepted user choices are distinguished from proposed engineering details inside the ADRs/schema.

| ID | Decision | Consequences / references |
| --- | --- | --- |
| D-118 | Offline-first, single-user-first, one active machine for alpha. Rust/Axum, Svelte/TypeScript, local browser and GPL-3.0-or-later remain fixed. | No speculative account/tenant/sync subsystem. Multiple tabs still need transactional safety. ADR-001. |
| D-119 | SQLite WAL mode, foreign keys enabled, FTS5 supported. | Verify foreign keys on each connection; use local live database. Snapshot consistency required. Runtime dependency must include upstream WAL-reset fix; engineering defaults documented separately. ADR-002. |
| D-120 | Inventory derives from immutable inventory movements, never purchase totals. Every stock-changing command appends movements/lifecycle facts. | Current holding balances become replayable projections; purchases remain provenance. Unknown measures, damage and assembly require typed effects, not one undifferentiated quantity_change. ADR-003 and DATABASE_SCHEMA.md. |
| D-121 | Images/PDF bytes stored on filesystem; SQLite stores attachment identity/path/hash/size and associations. | No attachment BLOBs; coherent file/database snapshots required. ADR-004. |
| D-122 | Brickxinator provides full ZIP snapshot export/coordinator plus external backup hook; do not embed backup engines. Monthly full ZIP self-contained. | Metadata manifest, SQLite consistent copy, referenced files and config. ADR-005. |
| D-123 | Axum REST/JSON for alpha, versioned API. | No GraphQL/gRPC/WebSockets-first. Draft command endpoints, idempotency and local-tab liveness documented. ADR-006. |
| D-124 | SQLite FTS5 for text search, alongside structured database indexes. | No Elasticsearch. FTS/projections rebuildable; UUID domain IDs distinct from internal FTS rowids. ADR-007. |
| D-125 | Native JSON exchange, human CSV export, images/filesystem; future LDraw/Studio/Rebrickable imports deferred. | Versioned format, explicit unknowns, UUID references and asset manifests. Obsidian import remains deferred. ADR-008. |
| D-126 | Ordered migration_001.sql etc.; never alter existing migrations. | Checksummed migration ledger and new corrective migrations. Draft schema document is not an installed migration. ADR-009. |
| D-127 | UUIDv7 for all domain entities. | Draft canonical text encoding; explicit operation sequence separate from IDs. No distributed-system feature commitment. ADR-010. |
| D-128 | Explicit follow-up: daily incremental EXTERNAL backup remains mandatory, in addition to the daily full application ZIP snapshot. | Supersedes ADR-005's temporary pending policy. Daily recovery requires configured external tool/adapter with incremental capability; no embedded engine. ZIP-only success is not successful daily backup. Monthly ZIP still independent. 30 daily external recovery points plus required dependencies and 12 monthly ZIPs; staging is not another unbounded retention store. |

### Technical freeze outcome

Created ADR-001 through ADR-010, DATABASE_SCHEMA.md and RUST_STRUCTURE.md before source implementation. Detailed table encodings/module splits/connection tuning are marked engineering drafts. The source-of-truth ledger and mandatory external incremental policy are accepted decisions. Exact external executable, dependency pins and format fixtures must be selected/tested during implementation; a successful daily chain restore is a release gate, not an assumption. No Rust, SQL migration, installed timer or backup hook has been created.


## Alpha completion clarifications — 2026-11-01

These answers supplement the historical discovery record.

| ID | Decision | Consequences |
| --- | --- | --- |
| D-130 | Richard selected Restic for mandatory external daily incrementals. | Full application ZIP still precedes external protection; independent monthly ZIP remains. ADR-020. |
| D-131 | Defaults are working-directory `backups/daily` and `backups/montly`, editable after installation. | Preserve literal spelling. Defaults are local folders without a required mount; external-drive/share types require one. Source development activates no real user schedule. |
| D-132 | Create public open-source repository `richardnoragon/brickxinator`. Private security reporting is deferred. | Repository visibility is public; this does not approve an alpha release or waive review/reporting gates. |
| D-133 | No other source repository exists; continue in this workspace with the Rust inventory foundation and finish alpha implementation. | Existing accepted scope and integrity/recovery gates continue to apply. |
| D-134 | Any never-received draft without invoice lines or attachments may be permanently deleted, including entered metadata. | Implement explicit purge; minimal deletion/request identities remain to prevent accidental resurrection, while entered metadata/audit snapshots are removed. ADR-021. |
| D-135 | Whole-set sale/donation/loss disposal is deferred beyond alpha. | Part-holding adjustments remain in alpha. Archive never removes ownership; copy-count corrections cannot discard existing copy identities. |

Implementation details and evidence are in ADR-019 through ADR-022 and the root
ALPHA_STATUS.md. No merge/tag/release or real backup timer is implied by this entry.
