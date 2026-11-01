# Alpha discovery workbook

Status: rounds 1–15 recorded; functional discovery foundation sufficient to begin alpha implementation. Updated: 2026-10-31.

## Working method

Ask focused rounds, preserve the answers here, assign confirmed decisions in DECISIONS.md, and update affected project documents. Recommendations remain proposals until answered. Preserve the original napkin file as source evidence. Do not infer approval from an unanswered question.

## Round 1 — purpose and alpha boundary

1. What real task should the alpha help you complete? Describe a typical session, what you use today (if anything), and the result you want.
2. Which features must be usable in the first alpha? Proposed boundary for discussion: manual part inventory, search/filter, settings-managed storage, individually tracked set copies, local persistence, backup/restore. Defer CAD, train planning, MOC authoring and online catalog integrations. Accept or adjust this boundary.
3. Which operating system(s) must the alpha run on, and should it be an installed desktop application, local browser application, or something else?
4. What starting data do you have: none, spreadsheets, another application, downloaded catalogs? If importing is essential for alpha, which formats and approximate row counts?
5. How should owned sets affect part quantities? Options to discuss: separate set and loose-part records; set parts included in totals but unavailable while assembled; another rule. Do you need build reservations in alpha?
6. What three concrete checks would convince you the alpha is useful enough to start using? Any target date or firm constraints (hardware, budget, required technologies)?

### Answers received — 2026-10-31

1. Currently uses Obsidian to document purchases, mostly from eBay, copying pictures and manually entering values including part number and built/loose state.
2. Agrees to the proposed alpha scope and deferrals.
3. Kubuntu only. Delivery model not specified.
4. Existing Obsidian Markdown and pictures. Import requirement and scale not specified.
5. Set contents contribute to total parts inventory; assembled parts are unavailable. Reservations deferred.
6. Functionality is the only constraint for this hobby project. No deadline, required technology, hardware or budget constraint except Kubuntu. Three concrete acceptance checks not yet supplied.

Recorded as D-018–D-022. No unanswered subquestion is treated as approved.

## Round 2 — purchases and inventory behavior

1. For a purchase, should one record contain multiple items (sets and/or loose parts), shared seller/date/shipping and individual item prices? Which purchase details do you want?
2. Is importing existing Obsidian notes/pictures required for alpha, or is manual entry acceptable initially? For import: roughly how many notes, is their structure consistent, and how are images linked? A small pasted, anonymized example would help.
3. When you acquire a set whose detailed contents are unknown, may it be saved with only a stated piece count? Proposed behavior: show that count separately from identified parts; do not invent individual part quantities. How should the alpha obtain detailed set contents without online integrations?
4. For alpha, is a whole-set assembled/loose switch sufficient (no partial disassembly)? On disassembly, should parts remain linked to their set copy while becoming available, or join loose stock? How do you want to avoid entering those parts twice?
5. May identical part/color/manufacturer combinations be split across storage locations and purchases, with a combined total in search? Should unknown manufacturer/color/quantity remain blank rather than guessed?
6. Do you prefer an installed desktop application or a local browser application? Which Kubuntu version do you use?

### Answers received — 2026-10-31

1. Proposed purchase structure is sufficient for alpha: multiple items with shared seller/date/shipping and individual prices.
2. Manual entry is acceptable initially; Obsidian import is deferred.
3. Agrees that sets can be saved with stated piece counts separate from identified parts, without inventing individual quantities. Detailed content entry method remains open.
4. Whole-set assembled/loose switch is sufficient. Parts remain linked to their set copy and become available when loose.
5. Identical parts may span purchases and storage locations; search combines quantities. Unknown quantities, colors and manufacturers may remain blank.
6. Local browser application, Kubuntu 26.04 LTS.

Recorded as D-023–D-028.

## Round 3 — entry, identity and storage

1. What should you be able to add manually: a set, a loose-part batch, or an unidentified mixed lot? For set contents, is a table for entering part number, color and quantity sufficient for alpha, with an option to copy the contents from another copy of the same set?
2. Proposed identity rule: keep part definitions separate by manufacturer + part number; distinguish stock by color and condition. Never automatically merge entries with missing identifiers; allow manual linking/correction. Does this fit, and should generic/unknown-brand parts stay separate from branded parts?
3. Proposed totals: show known quantities plus an indication of entries with unknown quantities (for example, “42 known pieces; 2 batches with unknown quantity”). Blank means unknown, never zero. Unidentified stated set counts remain separate. Agree?
4. How should storage work: flat location names or nested locations (room → cabinet → drawer → bin)? Must one set copy or loose-part batch be splittable across locations in alpha? For moves, is selecting a quantity and destination sufficient?
5. For purchase pictures and notes, is this sufficient: attach multiple local images, paste images from clipboard, enter free-text notes, and optionally store the original eBay URL? Should images belong to the purchase, individual items, or both?
6. May prices be optional, use EUR only in alpha, and keep shipping separate without automatically allocating it across items? Should the interface start in English, German, or support both?

### Answers received — 2026-10-31

1. Yes to sets, loose batches, mixed lots, table entry and copying set contents; fields can be edited later.
2. Confirms manufacturer/part-number identity, color/condition distinctions, manual linking of incomplete entries, and separation of unknown-brand stock.
3. Rejects the proposed count-only treatment: inventory may be measured by weight instead of pieces.
4. Nested locations allowed; check misspellings; no splitting across locations.
5. Accepts proposed pictures/notes/source URL capabilities and both purchase/item attachment levels.
6. EUR only for alpha. Shipping must be shared and belong to the purchase. Allocation meaning, optional prices and interface language remain open.

Recorded as D-029–D-034.

## Round 4 — measurements, shipping and storage rules

1. Should each stock entry support piece count, weight, both, or neither? Proposed: optional piece count plus optional weight, entered as g or kg and normalized for totals; mixed lots can remain unidentified. No automatic weight-to-piece conversion. Agree or adjust.
2. Should search show piece and weight totals separately, such as “42 pieces; 1.5 kg,” identify incomplete measurements, and avoid counting the same stock twice when both measurements exist? For mixed lots, should their weight appear only in collection/lot totals until individual parts are identified?
3. By shared shipping, do you mean (A) one purchase-level charge included once in the purchase total, without item allocation; (B) purchase-level charge distributed equally across purchased items; or (C) allocation proportional to item price? Another method is welcome. May unknown prices and shipping remain blank?
4. Proposed storage behavior: choose existing locations from a tree; create names in settings; warn about duplicate/similar sibling names without silently correcting them; one location per set copy or batch; move the whole record. Does this fit? Should a set's linked parts inherit its location?
5. When a set becomes loose, can its linked contents remain together in one location for alpha? Separately relocating its contents would require relaxing the no-splitting rule or a later feature.
6. Which interface language should alpha use: English, German or both?

### Answers received — 2026-10-31

1. Optional count and optional g/kg weight; no automatic conversion.
2. Separate count/weight totals describing the same stock where both exist; mixed-lot weight separate from identified-part totals.
3. Requests flexible shipping: purchase-level default, equal, proportional by item value, and measurement-based allocation (weight/pieces/volume mentioned). Blank prices/shipping allowed; unknown is distinct from zero. Adds purchase tax, currency, order number and allocation method. General principle: never require users to enter unknown information.
4. Accepts tree selection, settings-managed names and duplicate/similarity warnings without silent correction.
5. Confirms shared set/contents location even when loose; explicitly asks that this restriction be highlighted.
6. English interface.

Recorded as D-035–D-042.

**Highlighted alpha limitation: a set copy and all linked contents share one location even when loose. Individual relocation of linked contents is deferred.**

## Round 5 — shipping calculation and accounting

1. For alpha, should all shipping alternatives be implemented now: purchase-level only (default), equal per purchase item row, proportional to item cost, by weight, and by piece count? May volume-based allocation be deferred since volume measurement has not otherwise been required?
2. Is “None” the same as purchase-level only (shipping recorded but no item shares), or does it have a distinct meaning? Should changing the method simply recalculate item shipping shares while preserving original item prices and purchase shipping?
3. If an allocation basis is missing, zero across all items, or shipping itself is unknown, should allocation remain incomplete with a clear explanation rather than inventing values? Should users be able to enter explicit item shares manually as another mode?
4. May EUR calculations use cents, distributing rounding remainder deterministically so item shares sum exactly to purchase shipping? For equal allocation, does “item” mean each purchase row rather than every individual brick?
5. Are entered item prices the total paid for each row (for example, 100 bricks for EUR 5), or unit prices? Is tax already included in prices with an optional informational purchase tax field, or should tax be added separately?
6. Proposed purchase summary: item-price subtotal plus shipping, with unknown amounts clearly marked and totals labeled incomplete. Allocated shipping is a breakdown of the same charge and is never added twice. Does that fit? Should discounts/refunds/returns wait until after alpha?

### Answers received — 2026-10-31

1. All six alpha modes: purchase level, equal, price proportional, piece count, weight and manual. Volume deferred.
2. Merge None/Order Level; allocations preserve source prices and recalculate shipping/effective cost.
3. Missing data prevents calculation with an explanation, never fabricated values. Manual allocation required.
4. Equal means per purchase row; rounding shares must sum exactly to shipping.
5. Row total is stored and unit price derived from quantity. Actual paid price is the source value; tax optional, often included or unknown.
6. Incomplete purchase totals are flagged without blocking records. Shipping counted once. Discounts/refunds/returns and related flows deferred, suggested v1.1. Alpha price history and manual overrides are newly mentioned. Tax addition versus included-tax semantics remain ambiguous.

Recorded as D-043–D-050.

## Round 6 — tax, history and valid calculations

1. For alpha, may all entered prices be final paid amounts, with tax marked unknown, included, or not applicable and optional informational tax amount/rate? Included tax would never be added again. Or must alpha also support tax added on top of entered prices?
2. May users correct purchase values while retaining the old value and timestamp in history? Allocation recalculation would never edit source prices. Does “price history” mean this correction log, comparisons across purchases, external market-price tracking, or some combination?
3. In manual allocation, may incomplete/mismatched shares be saved as a draft with a warning, but remain invalid for effective-cost calculation until all nonnegative shares total exactly the shipping charge? No automatic correction of manual shares is proposed.
4. For automatic allocation, may a missing basis or a zero total basis leave allocation incomplete; known zero bases receive zero shares? Proposed rounding: calculate in cents, assign remainder cents by largest fractional remainder, break ties by stable purchase-row order. Agree?
5. Proposed unit costs: per piece when a known positive count exists; per kg when a known positive weight exists; show both when available, clearly labeled. Unknown/zero denominators yield no unit cost. Keep higher precision for derived unit costs while payable amounts remain in cents. Agree?
6. Missing information should warn without blocking saving. May invalid supplied values (negative counts/weights/prices, malformed numbers) require correction or clearing, and may users save otherwise blank records with generated IDs? This distinguishes unknown data from invalid data.

### Answers received — 2026-10-31

1. Paid prices authoritative; tax informational with Included, Unknown, Not Applicable and Added Separately states; never add twice.
2. Timestamp all corrections and preserve history. Alpha historical purchase prices per item; external market history deferred.
3. Save incomplete manual shares as drafts, show remaining amount; no silent changes; effective allocated costs unavailable until complete.
4. Incomplete automatic allocation for missing/zero-total basis; largest remainder with stable row order.
5. EUR/piece and EUR/kg when possible; higher internal precision and user-selectable display precision.
6. Missing allowed, invalid rejected, blank drafts allowed with generated IDs. Adds seller type and country to purchases, with named seller categories.

Recorded as D-051–D-057.

## Round 7 — daily workflow and stock lifecycle

1. Suggested primary workflow: create purchase draft → enter seller/date/shipping → add set, loose-batch or mixed-lot rows → attach images/notes → set storage → review inventory. Should saving purchase rows immediately make their known quantities owned stock, or should a received/added-to-collection action be required?
2. Should purchase quantity remain historical while current stock can change independently? For alpha, do you need explicit stock adjustments for lost/broken/given-away parts, or only auditable corrections to mistaken entries? Returns/refunds remain deferred.
3. When mixed-lot contents become known, should alpha support transferring quantities/weight into identified child batches while retaining the original purchase link? Or is identifying/correcting the whole lot sufficient initially? Transfers must not count the source and destination stock twice.
4. Suggested set-copy labels: manufacturer + set number + sequence (for example LEGO 7838_01, LEGO 7838_02), backed by immutable internal IDs. May users rename visible labels? Should “set” and “part” numbers use separate namespaces?
5. Which searches/filters must alpha offer: part/set number, name/description, manufacturer, color, condition, seller, purchase date, storage subtree, assembled/loose, availability, unidentified/draft? Which three would you use most?
6. What should the opening screen prioritize: inventory search, recent purchases, or a dashboard? Are these useful acceptance examples: record an eBay purchase with pictures and unknown fields; find stock by part/location; switch a set assembled/loose with correct availability and no duplicate counts?

### Answers received — 2026-10-31

1. Require Received before adding to inventory; Draft/Ordered/Shipped/Received/Cancelled states.
2. Purchased quantities immutable; stock separate. Alpha adjustments Lost/Broken/Donated/Sold/Correction.
3. Gradually decompose mixed lots, preserve purchase provenance, and conserve remaining plus identified stock without double counting.
4. Permanent internal IDs, default numbered set-copy labels, freely renameable display labels.
5. Priority: number, location, availability; remaining filters ranked. Reserved/Missing examples need reconciliation with deferred reservations and undefined missing-parts tracking.
6. Dashboard, global search, suggested metrics; accepts three success workflows and adds gradual mixed-lot identification as a fourth.

Recorded as D-058–D-064.

## Round 8 — lot conservation and stock transitions

1. For a weight-only lot (5 kg), when identifying 100 tracks, should users also weigh the extracted tracks to reduce remaining lot weight? Proposed: allow identification without weight, but mark remaining weight unknown/outdated and never convert count to weight. Show original acquisition weight separately from current stock weight. Agree or another rule?
2. Should identified child batches inherit their lot's location initially, then allow each whole child batch to move elsewhere? This would preserve no splitting of a single batch while allowing gradual lot sorting. Set-linked contents would still share their set's location.
3. When decomposing a priced lot, should alpha keep the original cost at the purchase row and leave child-batch costs unknown unless explicitly allocated? Or must it support internal cost allocation to children now? Purchase shipping allocation remains separate from decomposition cost allocation.
4. To reconcile immutable purchase history with corrections: may a correction append a new revision/event while retaining original values? Receipt would happen once per whole purchase in alpha; subsequent adjustments affect stock separately. Are partial receipts and cancellation/reversal after receipt required now or deferred?
5. Does Broken remove stock from owned totals, or keep it owned but unavailable with broken condition? Lost/Donated/Sold would reduce current owned stock. May stock reductions be rejected when they exceed known available stock, with unknown-quantity cases requiring an explicit measurement/correction first?
6. May alpha availability filters be Available, In assembled set, and Unknown, while Reserved waits until later? Does Missing Parts mean deficits against manually entered expected set contents (requiring expected versus actual tracking), or can this dashboard metric wait?

### Answers received — 2026-10-31

1. Preserve original weight permanently; remaining weight unknown after unweighed extraction; never silently estimate.
2. Child batches inherit location and can move whole; no partial batch splitting; set-linked contents remain with set.
3. Original purchase cost retained; child costs can be unknown; internal lot-cost allocation deferred (not purchase shipping allocation).
4. Append correction revisions. Whole-purchase receipt only; partial receipts/shipments, backorders and reversal deferred.
5. Broken remains owned/unavailable; Sold/Donated reduces ownership; reject over-removal and require clarification of unknown stock.
6. Four availability categories including Broken/Damaged. Reservations/missing-parts analysis deferred. Dashboard: pending deliveries, unidentified lots, incomplete records, recent purchases, inventory totals.

Recorded as D-065–D-071.

## Round 9 — operation, backups and data protection

1. Local access proposal: app accessible only on this computer, bound to loopback, no login for alpha, and no remote/LAN access. External listing URLs open only when clicked. Agree?
2. Which browser must work: Firefox (Kubuntu default), Chromium/Chrome, or both? For startup, is a launcher that starts the local service and opens the browser sufficient? Should closing the browser leave the service running until explicitly stopped?
3. Proposed backup: one portable archive containing database, pictures and other attachments; manual backup/restore plus automatic pre-migration/pre-restore safety backups. Do you also want automatic daily backups, and where/how many should be retained? External/off-device copying would remain your choice.
4. May attached/pasted images be copied into app-managed storage, preserving original source files? Should alpha include PDF invoice/instruction attachments too, or images only? No online fetching is proposed.
5. For records with history/stock links, should deletion archive instead of erase, with restore available? May empty, never-received drafts be permanently deleted? How should attached files behave when a record is archived?
6. Damage and assembly can overlap. May these be independent facts, with damaged quantities always unavailable even inside an assembled set, and a Broken/Damaged filter matching them regardless of assembly state? Moving/marking a set loose must not repair damaged stock.

### Answers received — 2026-10-31

1. Loopback-only, no login/LAN; explicit-click external links accepted.
2. Firefox and Chromium required; launcher opens local browser app; service must quit when browser closes.
3. Automatic daily incremental backups plus separate automatic monthly backups on the first; monthly does not replace daily. Destination, retention, monthly backup type, scheduler and missed runs unanswered.
4. Managed image copies preserve originals; PDF invoices/instructions included.
5. Accepts recoverable archival and permanent deletion of empty never-received drafts; retain archived attachments.
6. Damage and assembly independent.

Recorded as D-072–D-077.

## Round 10 — shutdown and reliable backups

1. For shutdown, may the service stop after the last Brickxinator tab closes (not when unrelated browser tabs close), using a short grace period to tolerate refreshes? Should a browser crash also cause shutdown after a timeout? Multiple Brickxinator tabs would keep it running until the last closes.
2. To back up while the app is closed, may an operating-system scheduled backup job run independently of the web service? It would run only for backup, leaving the browser service stopped. Or should automatic backups run only while the app is open, catching up at next launch?
3. Where should backups be stored: a chosen local folder, external drive, network folder, or several destinations? If unavailable, may the app retry later and show a persistent failure warning without blocking inventory use?
4. Should monthly backups be full, independently restorable snapshots? Daily incrementals need an initial full baseline and intact dependent files. How long should daily and monthly backups be kept? Proposed for discussion: 30 daily recovery points and 12 monthly snapshots, preserving any baseline/ancestors needed by retained daily points.
5. If the computer is off at the scheduled time, should backup run at the next opportunity? May the app use computer-local calendar days/time, run daily at a chosen time, and run both daily and monthly jobs on the first?
6. Proposed restore: choose recovery point, validate archive/checksums and chain, create pre-restore safety backup, stop writes, restore database plus attachments together, and report success/failure. Agree? Should archiving a stock-bearing record only hide it from normal lists while keeping its stock in owned totals?

### Answers received — 2026-10-31

1. Stop after last tab with 60–120 second grace; always-on mode later.
2. Backups independent of application, OS scheduling preferred.
3. Multiple simultaneous local/external/network destinations; persistent warnings and last-success information; do not block use.
4. 30 daily points and 12 full independent monthly snapshots, retaining daily chain dependencies. Yearly later.
5. Catch up at next opportunity; 03:00 configurable default.
6. Validate, safety-backup, consistent restore and post-restore validation. Archived records remain searchable, owned and counted. Strong preference for ordinary files/directories usable by standard Linux backup tools.

Recorded as D-078–D-085. Rust/SQLite diagrams treated as examples until explicit confirmation.

## Round 11 — explicit stack and operating defaults

1. Confirm technology proposal: Rust backend, SQLite database, Svelte + TypeScript browser UI, without Tauri for this local-browser alpha. Or choose another stack. These were previously suggestions rather than approved choices.
2. Backup implementation proposal: systemd user timers plus BorgBackup for daily incrementals/deduplication; separate plain-directory monthly full snapshots for independent restore. Borg daily points require Borg to restore; ordinary live data and monthly snapshots remain directly accessible. Prefer this approach, another existing engine, or plain-directory daily incrementals with managed dependencies? No tool is selected yet; feasibility/version details will be verified before final architecture.
3. May service shutdown default to 90 seconds, and backups use the computer's configured local timezone? Missed runs create one current catch-up recovery point per due schedule rather than pretending to recreate each missed day.
4. Software license: do you prefer MIT (permissive), GPLv3 (distributed derivatives preserve license/source obligations), or another? This covers our code; third-party images/catalogs/instructions keep their own rights. No source content license is assumed.
5. May app data use a standard per-user Linux data directory, with configurable location? May backup settings stay in local configuration and be included in backups, but restore avoid automatically reconnecting old destinations/overwriting machine-specific paths without review?
6. After a received purchase quantity is corrected, should the app preview the resulting stock reconciliation and reject impossible changes (for example, reducing acquisition below already donated/identified stock), retaining revision history? Proposed: corrections never re-receive a purchase or silently duplicate inventory.

### Answers received — 2026-10-31

1. Approves Rust/Axum, SQLite, Svelte/TypeScript, filesystem storage and OS-managed backups; no Tauri/PostgreSQL for alpha.
2. Daily may require existing backup software; monthly ordinary files/archive accessible without it.
3. 90-second shutdown, local timezone, one actual catch-up per due schedule.
4. GPLv3 chosen; exact only/or-later designation open.
5. XDG per-user default, configurable; restored paths/destinations require review.
6. Preview correction impact, reject impossible outcomes, preserve history and never receive twice. Adds explicit offline core requirement and requests domain-model focus.

Recorded as D-086–D-093. See DOMAIN_MODEL.md for the proposed model.

## Round 12 — domain model boundaries

1. May part definitions (manufacturer/part number/description) and set definitions (manufacturer/set number/name) be reusable catalog records, separate from owned stock and set copies? They would use separate number namespaces; incomplete stock can exist without a linked definition.
2. Should each owned set copy be its own purchase row, while loose-part and mixed-lot rows may contain many pieces? Three identical purchased sets would create three copy rows under one purchase; their individual prices can be unknown if only the combined invoice amount is known. Do we need a shared invoice-line grouping/combined price for this case in alpha?
3. When copying a set's content list, should alpha create expected/unverified entries that do not yet count as owned stock, or immediately create actual owned contents? Expected-versus-actual set auditing was previously deferred, so this needs an explicit choice. Stated box piece count remains separate from detailed counted inventory.
4. For partial damage, may a stock batch retain owned count/weight plus separately measured damaged count/weight at the same location? Available quantities would exclude damage and assembly without double-subtraction. If damage count is known but damage weight is not, available weight would be unknown rather than estimated.
5. For gradual lot identification, may transfers record extracted count and/or weight, subtract only measurements that remain valid, invalidate unknown residual measures, preserve provenance and leave child costs unknown? Reject extraction above a known source remainder in the same unit; completing classification requires explicit confirmation rather than guessing from weight/count.
6. Should color, manufacturer and condition use settings-managed reusable values, with blank meaning unknown? Suggested initial condition choices: New, Used, Mixed; damage remains a separate stock fact. Are additional conditions needed?

### Answers received — 2026-10-31

1. Separate reusable definitions from ownership, with zero-stock definitions and incomplete owned entries allowed.
2. Independently track physical copies; retain shared invoice price, leave per-copy prices unknown.
3. Copy to unverified checklist, not inventory. Accept limited expected/actual tracking in alpha; exact verification mechanics remain open.
4. Partial damage supported; unknown damaged weight means unknown available weight.
5. Known-measure transfers conserved, unknown remainder not estimated, over-extraction rejected, classification completion explicit.
6. Settings-managed manufacturer/color/condition. New/Used/Mixed and blank unknown; damage separate.

Recorded as D-094–D-099. Domain model updated with approved layers and new verification/grouping concepts.

## Round 13 — verification and invoice grouping

1. Can set checklists be partially verified line by line, entering actual counted quantities even if they differ from expected? Proposed: show Expected, Verified and Unverified; unverified is not automatically called missing. Verification creates actual contents only once and only for a received/owned set copy. Agree?
2. When verifying, should users choose between newly counted contents of this purchased set and linking/transferring stock already recorded elsewhere? Is the second workflow needed in alpha, or may existing-stock-to-set assignment wait? Merely copying a checklist never creates stock.
3. Proposed invoice structure: purchase → invoice lines → owned set copies/batches. One line can represent three sets with one EUR 300 line total and three independently owned copies with unknown individual cost. Shipping Equal means per invoice line; price/weight/count/manual modes also target invoice lines, never invented per-copy prices. Agree?
4. When marking a lot classified, should known nonzero remainder prevent completion until identified or explicitly adjusted? If remainder is unknown, may the user confirm physically empty, recording a timestamped completion fact without inventing its former quantity/weight?
5. Should checklist corrections after verification preserve expected-list history and preview any actual-stock changes separately? Changing expected quantities alone should never create/remove owned parts. Should receiving a used set create only the set copy, leaving actual parts to verification?
6. May managed values be renamed without changing their IDs, and archived rather than deleted while referenced? Unknown would have one canonical representation, displayed as Unknown rather than duplicate blank/Unknown categories. Does that fit?

### Answers received — 2026-10-31

Source: pasted text attachment supplied by user; substantive answers transcribed here for durable project reference.

1. Gradual Expected/Verified/Unverified; no automatic missing classification. Only explicit verification completion permits a local missing shortfall if appropriate.
2. No reassignment from existing stock during alpha verification. Record this set's actual count without changing other inventory; existing-stock transfer is a later workflow.
3. Approves Purchase → Invoice Line → physical Set Copies. One 3-copy line retains EUR 300 shared price and unknown copy prices; never invent EUR 100 each.
4. Known nonzero lot residue prevents completion and remains Partially Classified. For unknown residue, explicit Confirm physical lot empty records completion, confirming user and date without fabricating measured 0 kg.
5. Receipt creates owned set copy only; actual parts await verification. Expected edits do not affect inventory. Verified edits create stock correction with timestamp/reason/user/old/new values.
6. Permanent IDs and renameable managed names; archive referenced values, retain historical associations and exclude from new selection. One real system-managed Unknown category. Suggested set workflow proceeds purchase → receipt → copy → definition/checklist → gradual verification → completion → assembled/loose; mixed-lot workflow proceeds receipt → identification → explicit empty confirmation.

Recorded as D-100–D-105.

## Round 14 — final verification mechanics and schema constraints

1. May assembly be set independently of verification, including an already-built but unchecked received set? Only verified parts would contribute detailed part quantities; stated piece count stays separate. Or must verification completion be required before assembly changes?
2. May each checklist line store actual counted quantity plus an explicit checked/finished flag? Before finishing, an expected 10 / counted 7 means 3 unverified; after finishing, 3 short. Over-counts stay as actual owned excess, not rejected. Finishing an unchecked line at zero must be explicit. Agree?
3. When entering verified counts, is the number the current total observed for that line, rather than an increment? Re-entering 7 would leave 7 owned, while changing to 9 creates an audited +2 correction, never 16. Identifying details/condition can be split into actual batches without changing the expected definition.
4. Confirm shipping targets invoice lines in all six modes. A line quantity of 3 sets is a set-copy count, not a brick count; piece-count shipping requires a separate known brick count, and price mode may use its known EUR 300 total without assigning copy costs. Agree?
5. May stock-changing corrections require a reason, with timestamp and local user identity recorded automatically (no login)? Can other descriptive edits keep an optional reason? Unknown system vocabulary entries would be protected against renaming/archiving.
6. For classification completion with a confirmed-empty unknown residue, may current totals exclude that residue while preserving its prior unknown measure and the completion fact? If parts are later found, reopen classification through an audited action rather than editing away the old confirmation.

### Answers received — 2026-10-31

1. Assembly independent from verification; verification states Not Started/In Progress/Completed. Adds assembly Partially Assembled alongside Assembled/Loose; stock semantics pending.
2. Explicit line finish; Short only after finish, Excess permitted.
3. Absolute verified totals, repeated values do not add stock; changes are audited differences.
4. Confirms invoice-line shipping; set-copy count is not brick count.
5. Mandatory stock-correction reasons with timestamp/local user/old/new values; descriptive reasons optional. Protect all system Unknown entries.
6. Explicit empty residual is excluded from current totals; audited reopen preserves completion and new reason/date.

Recorded as D-106–D-111.

## Round 15 — last state and project-policy choices

1. For Partially Assembled, may alpha treat all linked set parts as unavailable, just like Assembled, with the label recording your observation? Per-part assembly allocation would remain deferred. Or keep only Assembled/Loose for alpha?
2. Proposed checklist rules: you can finish/reopen each line explicitly; overall verification completes only by explicit action after every active line is finished. Editing expected quantities or verified counts on a finished line reopens that line and overall verification, preserving history. Is that appropriate?
3. Which exact project license designation: GPL-3.0-only or GPL-3.0-or-later? What copyright holder name/year should appear? Default proposed start year 2026; no name inferred from conversation prose.
4. For alpha release gates, may the four cornerstone workflows, accounting/conservation checks, Firefox/Chromium checks, backup restore drill and upgrade-migration safety checks be mandatory before a tagged prerelease? Proposed version shape 0.1.0-alpha.1, not a release promise.
5. For contributions, may we start with main + short feature branches, reviewed pull requests, Rust formatting/lint checks and frontend formatting/type checks, behavior tests for stock/cost changes, and updated docs/decisions? Commit format preference: conventional commits or plain descriptive messages?
6. Should alpha be source-build installation only, with a launcher and systemd backup setup instructions, or must it ship an installable Kubuntu package? Where should public bugs and private vulnerability reports go once the GitHub repository is chosen? We can leave contact URLs explicitly pending until the repository exists.

### Answers received — 2026-10-31

1. Loose/Assembled/Partially Assembled; both assembled labels make all linked parts unavailable. No per-part assembly tracking.
2. Explicit overall completion after finished lines; count changes reopen affected line and overall process visibly, preserving history.
3. GPL-3.0-or-later; Copyright (C) 2026 Richard.
4. Mandatory workflow, integrity, browser, restore and migration gates; first target 0.1.0-alpha.1.
5. main/feature branches/reviewed PRs, formatting/lint/type/test/doc checks. Conventional Commits preferred rather than enforced.
6. Source build sufficient; Kubuntu package later. Reporting destinations pending repository selection.

Recorded as D-112–D-117.

## Discovery outcome

A sufficient functional foundation is established to begin alpha implementation. General questioning can stop. New domain ambiguity discovered during implementation should be raised with a concrete example, recorded here and in DECISIONS.md, then reflected in current specifications.

The current specifications are README, DOMAIN_MODEL, ARCHITECTURE, ROADMAP, USER_GUIDE, TESTING, SECURITY, RELEASE_PROCESS and CONTRIBUTING. This workbook and DECISIONS preserve historical questions/answers, including earlier open points superseded by later rounds. Historical “pending” text is not the current specification.

Nonblocking implementation selections: backup engine, dependency versions, migration library, field precision and schema/index details, attachment limits, UI details and performance measurements. Public repository/reporting contacts and conduct enforcement need resolution before opening the project to external participation. No implementation or release has occurred.


## Later rounds

- Inventory identity, optional fields, quantities, colors, manufacturers, duplicates and storage rules.
- Set-copy lifecycle, collection boundaries, reservations and imports/exports.
- UI workflows, search, accessibility, language and expected scale.
- Architecture and technology selection, persistence, attachments and migration strategy.
- Data protection, backup/restore, dependency and catalog licensing.
- Acceptance criteria, testing, release/versioning policy and distribution.
- Open-source license, contributions, conduct and support.

## Documentation coverage

| Document | Information to establish |
| --- | --- |
| README.md | Purpose, confirmed alpha features, platforms, installation and first-use workflow |
| ARCHITECTURE.md | Approved stack, boundaries, data relationships, adapters and extensibility |
| ROADMAP.md | Alpha scope, later milestones, dependencies and deferred ambitions |
| DECISIONS.md | Dated decisions, source, rationale, consequences and revisions |
| TESTING.md | Behavioral acceptance examples, integrity checks, regression and release checks |
| SECURITY.md | Local data, attachments/imports, backups, supported versions and reporting |
| RELEASE_PROCESS.md | Versioning adoption, release gates, packaging and approval |
| CONTRIBUTING.md | Setup, coding standards, branches, commits, PRs and checks |
| CODE_OF_CONDUCT.md | Community expectations and enforcement contact |
| LICENSE.md | User-selected software license; separate treatment of third-party content |
| CHANGELOG.md | Actual changes; no invented release history |
| FAQ.md | Answers established during discovery |
| USER_GUIDE.md | User workflows; listed in original TOC but template absent |
| TOC.md | Accurate navigation of completed and pending documents |

## Ready-to-code gate — proposed, awaiting agreement

- Alpha scope and exclusions are explicit.
- Main user workflows and representative acceptance examples are agreed.
- Inventory/set accounting, identity and storage behavior are unambiguous.
- Platforms, stack and persistence strategy are selected.
- Import/export and backup/recovery expectations are defined.
- Test and release gates, licensing and important data risks are addressed.
- Remaining open questions are explicitly deferred or identified as blockers.

The remaining templates will be populated as answers supply evidence; placeholder recommendations are not treated as established policy.

## Pre-code technical freeze follow-up — 2026-10-31

User requests formal technical selections before coding: SQLite WAL/foreign keys/FTS5, movement-based inventory, filesystem attachments, ZIP snapshot coordinator/external hooks, Axum REST/JSON, JSON/CSV formats, immutable migrations, UUIDv7 and single-user offline scope. Produce ADR set, database schema draft and Rust module/crate plan.

Clarification asked: may daily external incremental become optional because application ZIPs are full? **User answer: “Daily incremental external backup remains mandatory.”** Therefore retain full ZIP application snapshot plus mandatory external incremental recovery, separately reported and tested; monthly full ZIP remains independent. No claim that ZIP itself is incremental.

Recorded D-118–D-128. Current technical artifacts are ADRs/README.md, DATABASE_SCHEMA.md and RUST_STRUCTURE.md. Source implementation has not begun.
