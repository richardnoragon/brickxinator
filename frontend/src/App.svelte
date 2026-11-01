<script lang="ts">
  import { onMount } from 'svelte';
  import {
    api,
    bootstrap,
    envelope,
    uuid,
    integer,
    scaled,
    measure,
    known,
    money,
    amount,
    quantityAmount,
    rate,
    token,
    type Data,
  } from './api';
  import './style.css';
  let data = $state<Data>({
    settings: [],
    catalog: [],
    profiles: [],
    purchases: [],
    attachments: [],
    set_copies: [],
    closed_lots: [],
    dashboard: null,
  });
  let page = $state('Dashboard');
  let error = $state('');
  let notice = $state('');
  let busy = $state(false);
  let inventory = $state<Data>({ rows: [], total_matches: 0, totals: null });
  let includeArchived = $state(false);
  let assemblyFilter = $state('');
  let kindFilter = $state('');
  let incompleteOnly = $state(false);
  let sellerFilter = $state('');
  let purchasedFrom = $state('');
  let purchasedThrough = $state('');
  let query = $state('');
  let storage = $state('');
  let availability = $state('');
  let brandFilter = $state('');
  let colorFilter = $state('');
  let conditionFilter = $state('');
  let offset = $state(0);
  let correcting = $state(false);
  let correctionProposal = $state<Data | null>(null);
  let correctionReason = $state('');
  let purchase = $state<Data | null>(null);
  let purchaseForm = $state<Data>({});
  let costs = $state<Data | null>(null);
  let manual = $state<Record<string, string>>({});
  let ratePlaces = $state(4);
  let selected = $state<Data | null>(null);
  let links = $state<Data>({});
  let operation = $state('Broken');
  let pieces = $state('');
  let grams = $state('');
  let damaged = $state('');
  let damagedGrams = $state('');
  let unknownPieces = $state(false);
  let unknownWeight = $state(false);
  let unknownDamage = $state(false);
  let unknownDamageWeight = $state(false);
  let reason = $state('');
  let identifiedPieces = $state('');
  let identifiedGrams = $state('');
  let identificationReason = $state('');
  let childPart = $state('');
  let physicalEmpty = $state(false);
  let showRetired = $state(false);
  let recountProposal = $state<Data | null>(null);
  let copyLabel = $state('');
  let setId = $state('');
  let setView = $state<Data | null>(null);
  let expected = $state('');
  let part = $state('');
  let lineColor = $state('');
  let copySource = $state('');
  let settingKind = $state('Manufacturer');
  let settingName = $state('');
  let parent = $state('');
  let settingEdit = $state<Data | null>(null);
  let acknowledge = $state(false);
  let templateRows = $state<Data[]>([]);
  let templateSource = $state('');
  let priceHistory = $state<Data[] | null>(null);
  let catalogEdit = $state<Data | null>(null);
  let catalogKind = $state('Part');
  let catalogManufacturer = $state('');
  let number = $state('');
  let title = $state('');
  let description = $state('');
  let year = $state('');
  let theme = $state('');
  let boxPieces = $state('');
  let history = $state<Data[] | null>(null);
  let uploadTarget = $state('');
  let importConfirmed = $state(false);
  let restoreConfirmed = $state(false);
  let purgeConfirmed = $state(false);
  let backupBusy = $state(false);
  let backupConfig = $state<Data | null>(null);
  let backupStatus = $state<Data[]>([]);
  async function loadBackups() {
    const value = await api('/backups');
    backupConfig = value.config;
    backupStatus = value.status;
    data.backup_status = backupStatus.filter((s) =>
      backupConfig!.destinations.some(
        (d: Data) => d.id === s.destination && d.enabled,
      ),
    );
    data.backup_notice = backupConfig!.destinations.some((d: Data) => d.enabled)
      ? null
      : 'Backups are disabled. Configure destinations and the OS timer in Recovery when ready.';
  }
  async function saveBackups() {
    if (!backupConfig) return;
    await api('/backups', 'POST', backupConfig);
    await loadBackups();
  }
  async function startBackups() {
    if (backupBusy) return;
    backupBusy = true;
    error = '';
    try {
      await runBackups();
      notice = 'Backups completed.';
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    } finally {
      backupBusy = false;
    }
  }
  async function runBackups() {
    try {
      await api('/backups/run', 'POST', {});
    } finally {
      await loadBackups();
    }
  }
  function auditLeaves(
    value: unknown,
    path = '',
    result: Record<string, string> = {},
  ): Record<string, string> {
    if (value !== null && typeof value === 'object' && 'Known' in value) {
      result[path] = String((value as Data).Known);
      return result;
    }
    if (value !== null && typeof value === 'object') {
      for (const [key, item] of Object.entries(value)) {
        if (['id', 'copy_id', 'holding_id', 'revision'].includes(key)) continue;
        const name = key
          .replaceAll('_', ' ')
          .replace(/([a-z])([A-Z])/g, '$1 $2');
        auditLeaves(item, path ? `${path} / ${name}` : name, result);
      }
    } else
      result[path] =
        value === null || value === undefined ? 'Unknown' : String(value);
    return result;
  }
  function auditChanges(entry: Data) {
    const before = auditLeaves(entry.before);
    const after = auditLeaves(entry.after);
    return [...new Set([...Object.keys(before), ...Object.keys(after)])]
      .filter((key) => before[key] !== after[key])
      .map((key) => ({
        field: key,
        before: before[key] ?? 'Not recorded',
        after: after[key] ?? 'Not recorded',
      }));
  }
  const pages = [
    'Dashboard',
    'Purchases',
    'Inventory',
    'Sets',
    'Catalog',
    'Settings',
    'Recovery',
  ];
  const vocab = (kind: string) =>
    data.settings.filter((r: Data) => r.kind.Vocabulary?.vocabulary === kind);
  const unknown = (kind: string) =>
    vocab(kind).find((r: Data) => r.kind.Vocabulary.system_unknown)?.id ?? '';
  const active = (records: Data[]) => records.filter((r) => !r.archived);
  function locationPath(id: string): string {
    const r = data.settings.find((r: Data) => r.id === id);
    if (!r) return '';
    return r.kind.Location?.parent_id
      ? `${locationPath(r.kind.Location.parent_id)} / ${r.name}`
      : r.name;
  }
  function selectableLocation(r: Data): boolean {
    return (
      !r.archived &&
      (!r.kind.Location.parent_id ||
        selectableLocation(
          data.settings.find((p: Data) => p.id === r.kind.Location.parent_id),
        ))
    );
  }
  const locations = () =>
    data.settings.filter((r: Data) => r.kind.Location && selectableLocation(r));
  const definitions = (kind: string) =>
    data.catalog.filter((r: Data) => r.kind === kind && !r.archived);
  const label = (r: Data) =>
    [r.details.number, r.details.name].filter(Boolean).join(' · ') ||
    'Incomplete definition';
  const targetId = (row: Data) => Object.values(row.target)[0] as string;
  const totals = (coverage: Data | undefined) =>
    coverage
      ? `${coverage.known_sum} known${coverage.unknown_holdings ? ` · ${coverage.unknown_holdings} unmeasured holding(s)` : ''}`
      : '—';
  const human = (value: string) =>
    (
      ({
        LooseBatch: 'Loose batch',
        MixedLot: 'Mixed lot',
        IdentifiedBatch: 'Identified batch',
        SetContent: 'Verified set contents',
        SetCopy: 'Physical set copy',
        UnquantifiedSetLine: 'Unknown copy count',
        OpeningHolding: 'Opening stock',
        PartiallyAssembled: 'Partially assembled',
        InAssembledSet: 'In assembled set',
        BrokenDamaged: 'Broken/damaged',
        PurchaseLevel: 'Purchase level',
        PriceProportional: 'Proportional to paid prices',
        PieceCount: 'By piece count',
      }) as Record<string, string>
    )[value] ?? value;
  function weightText(milligrams: string): string {
    const kg = BigInt(milligrams) >= 1000000n;
    const value = quantityAmount(milligrams, kg ? 6 : 3).replace(/\.?0+$/, '');
    return `${value} ${kg ? 'kg' : 'g'}`;
  }
  const weightMeasure = (value: unknown) =>
    known(value) ? weightText(known(value)) : 'Unknown';
  const weightTotals = (coverage: Data) =>
    `${weightText(String(coverage.known_sum))} known${coverage.unknown_holdings ? ` · ${coverage.unknown_holdings} unmeasured holding(s)` : ''}`;
  function availableWeight(row: Data): string {
    if (row.assembly !== 'Loose') return '0 g';
    const owned = known(row.holding.weight_mg.owned);
    const damaged = known(row.holding.weight_mg.damaged);
    return owned && damaged
      ? weightText(String(BigInt(owned) - BigInt(damaged)))
      : 'Unknown';
  }
  function external(url: string): string {
    try {
      const u = new URL(url);
      return ['http:', 'https:'].includes(u.protocol) ? u.href : '';
    } catch {
      return '';
    }
  }
  async function search() {
    inventory = await api('/inventory', 'POST', {
      text: query,
      assembly: assemblyFilter || null,
      kind: kindFilter || null,
      incomplete: incompleteOnly ? true : null,
      archived: includeArchived ? null : false,
      seller: sellerFilter || null,
      purchased_from: purchasedFrom || null,
      purchased_through: purchasedThrough || null,
      location_id: storage || null,
      manufacturer_id: brandFilter || null,
      color_id: colorFilter || null,
      condition_id: conditionFilter || null,
      availability: availability || null,
      include_descendants: true,
      offset,
      limit: 50,
    });
  }
  async function refresh() {
    data = await api('/state');
    await search();
    if (setId) setView = await api(`/sets/${setId}`);
  }
  async function run(work: () => Promise<unknown>) {
    if (busy) return;
    busy = true;
    error = '';
    notice = '';
    try {
      await work();
      if (!notice) notice = 'Saved.';
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    } finally {
      busy = false;
    }
  }
  async function send(kind: string, payload: Data) {
    const value = await api(`/commands/${kind}`, 'POST', payload);
    await refresh();
    return value;
  }
  function newPurchase() {
    correcting = false;
    correctionProposal = null;
    correctionReason = '';
    purgeConfirmed = false;
    purchase = { id: uuid(), revision: 0, status: 'Draft' };
    purchaseForm = {
      seller: '',
      seller_type: '',
      seller_country: '',
      purchased_on: '',
      tax_presentation: '',
      tax_amount: '',
      order_number: '',
      source_url: '',
      notes: '',
      shipping: '',
      lines: [],
    };
    costs = null;
    uploadTarget = '';
    page = 'Purchases';
  }
  function openPurchase(p: Data) {
    correcting = false;
    correctionProposal = null;
    correctionReason = '';
    purgeConfirmed = false;
    purchase = p;
    purchaseForm = {
      ...structuredClone($state.snapshot(p.draft)),
      seller_type: p.draft.seller_type ?? '',
      seller_country: p.draft.seller_country ?? '',
      purchased_on: p.draft.purchased_on ?? '',
      tax_presentation: p.draft.tax_presentation ?? '',
      tax_amount: amount(p.draft.tax_cents),
      seller: p.draft.seller ?? '',
      order_number: p.draft.order_number ?? '',
      source_url: p.draft.source_url ?? '',
      shipping: amount(p.draft.shipping_cents),
      lines: p.draft.lines.map((l: Data) => ({
        ...l,
        notes: l.notes ?? '',
        source_url: l.source_url ?? '',
        pieces: known(l.pieces),
        grams: known(l.weight_mg) ? quantityAmount(known(l.weight_mg), 3) : '',
        copies: l.copy_count === null ? '' : String(l.copy_count),
        price: amount(l.paid_total_cents),
      })),
    };
    costs = null;
    uploadTarget = '';
    page = 'Purchases';
  }
  function addPurchaseLine() {
    purchaseForm.lines = [
      ...purchaseForm.lines,
      {
        id: uuid(),
        kind: 'LooseBatch',
        description: '',
        notes: '',
        source_url: '',
        set_number: '',
        copies: '',
        pieces: '',
        grams: '',
        price: '',
        assembly: 'Loose',
      },
    ];
  }
  async function savePurchase() {
    if (!purchase) return;
    const draft = {
      seller: purchaseForm.seller || null,
      seller_type: purchaseForm.seller_type || null,
      seller_country: purchaseForm.seller_country || null,
      purchased_on: purchaseForm.purchased_on || null,
      tax_presentation: purchaseForm.tax_presentation || null,
      tax_cents: scaled(purchaseForm.tax_amount, 2),
      order_number: purchaseForm.order_number || null,
      source_url: purchaseForm.source_url || null,
      notes: purchaseForm.notes,
      shipping_cents: scaled(purchaseForm.shipping, 2),
      lines: purchaseForm.lines.map((l: Data) => ({
        id: l.id,
        kind: l.kind,
        description: l.description || null,
        notes: l.notes ?? '',
        source_url: l.source_url || null,
        set_number: l.kind === 'Set' ? l.set_number || null : null,
        copy_count: l.kind === 'Set' ? integer(l.copies) : null,
        pieces: measure(l.pieces),
        weight_mg: measure(l.grams, 3),
        paid_total_cents: scaled(l.price, 2),
        assembly: l.kind === 'Set' ? l.assembly : 'Loose',
      })),
    };
    if (purchase.status === 'Received') {
      const preview = await api(
        `/purchases/${purchase.id}/correction-preview`,
        'POST',
        { expected_revision: purchase.revision, draft },
      );
      correctionProposal = { draft, preview };
      notice = 'Preview ready; no correction has been applied.';
      return;
    }
    const body = purchase.revision
      ? {
          Save: {
            purchase_id: purchase.id,
            expected_revision: purchase.revision,
            status: purchase.status,
            draft,
          },
        }
      : { Create: { purchase_id: purchase.id, draft } };
    const result = await send('purchases', { ...envelope(), body });
    purchase = result.purchase;
  }
  async function applyCorrection() {
    if (!purchase || !correctionProposal || !correctionReason.trim()) return;
    const result = await send('purchases', {
      ...envelope(),
      reason: correctionReason,
      body: {
        Correct: {
          purchase_id: purchase.id,
          expected_revision: purchase.revision,
          draft: correctionProposal.draft,
          preview: correctionProposal.preview,
        },
      },
    });
    purchase = result.purchase;
    correcting = false;
    correctionProposal = null;
    correctionReason = '';
    costs = null;
  }
  async function receive() {
    if (!purchase) return;
    const result = await send('purchases', {
      ...envelope(),
      body: {
        Receive: {
          purchase_id: purchase.id,
          expected_revision: purchase.revision,
        },
      },
    });
    purchase = result.purchase;
  }
  async function loadCosts() {
    if (!purchase?.revision) return;
    costs = await api(`/costs/${purchase.id}`);
    manual = Object.fromEntries(
      costs!.lines.map((l: Data) => [
        l.invoice_line_id,
        amount(costs!.state.manual_shares[l.invoice_line_id]),
      ]),
    );
  }
  async function shipping(method: string) {
    if (!costs) return;
    await send('shipping', {
      ...envelope(),
      purchase_id: costs.purchase_id,
      expected_revision: costs.state.revision,
      expected_purchase_revision: costs.purchase_revision,
      action: { SetMethod: method },
    });
    await loadCosts();
  }
  async function saveManual() {
    if (!costs) return;
    await send('shipping', {
      ...envelope(),
      purchase_id: costs.purchase_id,
      expected_revision: costs.state.revision,
      expected_purchase_revision: costs.purchase_revision,
      action: {
        SetManualShares: Object.fromEntries(
          Object.entries(manual).map(([id, v]) => [id, scaled(v, 2)]),
        ),
      },
    });
    await loadCosts();
  }
  function selectInventory(row: Data) {
    selected = row;
    pieces = '';
    grams = '';
    damaged = '';
    damagedGrams = '';
    unknownPieces = false;
    unknownWeight = false;
    unknownDamage = false;
    unknownDamageWeight = false;
    reason = '';
    childPart = '';
    physicalEmpty = false;
    identifiedPieces = '';
    identifiedGrams = '';
    identificationReason = '';
    const p = data.profiles.find(
      (p: Data) => JSON.stringify(p.target) === JSON.stringify(row.target),
    );
    links = {
      definition_id: row.definition_id ?? '',
      color_id: row.color.id,
      condition_id: row.condition.id,
      location_id: row.location.at(-1)?.id ?? '',
      archived: p?.links.archived ?? false,
      revision: p?.revision ?? 0,
    };
  }
  async function purgePurchase() {
    if (!purchase || !purgeConfirmed) return;
    await send('purge', {
      id: uuid(),
      request_key: uuid(),
      actor: 'local-user',
      purchase_id: purchase.id,
      expected_revision: purchase.revision,
    });
    purchase = null;
    purgeConfirmed = false;
  }
  async function archivePurchase() {
    if (!purchase) return;
    const result = await send('purchases', {
      ...envelope(),
      body: {
        Archive: {
          purchase_id: purchase.id,
          expected_revision: purchase.revision,
          archived: !purchase.archived,
        },
      },
    });
    purchase = result.purchase;
  }
  async function archiveSelected() {
    if (!selected) return;
    links.archived = !links.archived;
    await saveLinks();
  }
  function manageCopy(copy: Data) {
    const p = data.profiles.find((p: Data) => p.target.SetCopy === copy.id);
    selected = {
      target: { SetCopy: copy.id },
      kind: 'SetCopy',
      label: copy.label,
      holding: null,
    };
    links = {
      definition_id: p?.links.definition_id ?? '',
      color_id: p?.links.color_id ?? unknown('Color'),
      condition_id: p?.links.condition_id ?? unknown('Condition'),
      location_id: p?.links.location_id ?? '',
      archived: p?.links.archived ?? false,
      revision: p?.revision ?? 0,
    };
    page = 'Inventory';
  }
  async function saveLinks() {
    if (!selected) return;
    await send('profiles', {
      ...envelope(),
      target: selected.target,
      expected_revision: links.revision,
      links: {
        archived: links.archived,
        definition_id: links.definition_id || null,
        color_id: links.color_id,
        condition_id: links.condition_id,
        location_id: links.location_id || null,
      },
    });
    selected = null;
  }
  function measurementChange(
    value: string,
    places: number,
    invalidate: boolean,
  ) {
    if (invalidate) return 'Invalidate';
    if (!value.trim()) return 'Unchanged';
    return { SetMeasured: places ? scaled(value, places) : integer(value) };
  }
  async function adjust() {
    if (!selected) return;
    if (!reason.trim()) throw new Error('A stock-change reason is required.');
    const action =
      operation === 'Broken'
        ? {
            Adjust: {
              Broken: { pieces: integer(pieces), weight_mg: scaled(grams, 3) },
            },
          }
        : operation === 'Correction'
          ? {
              Adjust: {
                Correction: {
                  pieces: {
                    owned: measurementChange(pieces, 0, unknownPieces),
                    damaged: measurementChange(damaged, 0, unknownDamage),
                  },
                  weight_mg: {
                    owned: measurementChange(grams, 3, unknownWeight),
                    damaged: measurementChange(
                      damagedGrams,
                      3,
                      unknownDamageWeight,
                    ),
                  },
                },
              },
            }
          : {
              Adjust: {
                Remove: {
                  kind: operation,
                  pieces: integer(pieces),
                  weight_mg: scaled(grams, 3),
                  damaged_pieces: integer(damaged) ?? 0,
                  damaged_weight_mg: scaled(damagedGrams, 3) ?? 0,
                },
              },
            };
    await send('stock', {
      ...envelope(),
      reason,
      holding_id: targetId(selected),
      expected_revision: selected.holding.revision,
      action,
    });
    selected = null;
  }
  async function identify() {
    if (!selected) return;
    const source = selected;
    const child = uuid();
    const chosenPart = childPart;
    await send('inventory', {
      ...envelope(),
      reason: identificationReason || 'Classify mixed-lot contents',
      body: {
        Identify: {
          source_id: targetId(source),
          child_id: child,
          expected_revision: source.holding.revision,
          pieces: integer(identifiedPieces),
          weight_mg: scaled(identifiedGrams, 3),
        },
      },
    });
    selected = null;
    if (chosenPart) {
      const profile = data.profiles.find(
        (p: Data) => p.target.Holding === child,
      );
      try {
        await send('profiles', {
          ...envelope(),
          target: { Holding: child },
          expected_revision: profile?.revision ?? 0,
          links: {
            definition_id: chosenPart,
            color_id: profile?.links.color_id ?? unknown('Color'),
            condition_id: profile?.links.condition_id ?? unknown('Condition'),
            location_id:
              profile?.links.location_id ?? source.location.at(-1)?.id ?? null,
          },
        });
      } catch (e) {
        throw new Error(
          `Stock was transferred successfully. Link the new child’s part definition separately: ${e instanceof Error ? e.message : String(e)}`,
        );
      }
    }
  }
  async function closeLot() {
    if (!selected) return;
    await send('stock', {
      ...envelope(),
      holding_id: targetId(selected),
      expected_revision: selected.holding.revision,
      action: { Complete: { confirm_physical_empty: physicalEmpty } },
    });
    selected = null;
  }
  async function openSet(id: string) {
    recountProposal = null;
    setId = id;
    setView = await api(`/sets/${id}`);
    copyLabel = data.set_copies.find((c: Data) => c.id === id)?.label ?? '';
    page = 'Sets';
  }
  async function verify(action: Data | string, why = '') {
    if (!setView) return;
    await send('verification', {
      ...envelope(),
      copy_id: setId,
      expected_revision: setView.verification.revision,
      reason: why,
      action,
    });
    setView = await api(`/sets/${setId}`);
  }
  async function recordCount(line: Data, total: number | null, why: string) {
    if (total === null) throw new Error('Enter an absolute physical count.');
    const current = setView?.contents.find(
      (content: Data) => content.line_id === line.id,
    );
    if (
      line.verified !== null &&
      current &&
      known(current.holding.pieces.owned) !== String(line.verified)
    ) {
      recountProposal = {
        line_id: line.id,
        total,
        reason: why,
        expected_holding_revision: current.holding.revision,
        expected_owned: structuredClone(
          $state.snapshot(current.holding.pieces.owned),
        ),
      };
      notice =
        'Review the fresh physical recount before changing current stock.';
      return;
    }
    await verify({ Count: { line_id: line.id, total } }, why);
  }
  async function applyRecount() {
    if (!recountProposal) return;
    const proposal = recountProposal;
    await verify(
      {
        ReconcileCount: {
          line_id: proposal.line_id,
          total: proposal.total,
          expected_holding_revision: proposal.expected_holding_revision,
          expected_owned: proposal.expected_owned,
        },
      },
      proposal.reason,
    );
    recountProposal = null;
  }
  async function addExpected() {
    await verify({
      AddLine: {
        line_id: uuid(),
        holding_id: uuid(),
        definition_id: part || null,
        color_id: lineColor || unknown('Color'),
        expected: integer(expected),
      },
    });
    expected = '';
  }
  async function copyDefinitionChecklist() {
    if (!templateSource) return;
    const lines = await api(`/catalog/${templateSource}/checklist-preview`);
    if (!lines.length)
      throw new Error('This definition has no expected checklist.');
    await verify(
      { AddExpected: { lines } },
      'Copy definition expected checklist',
    );
  }
  async function copyChecklist() {
    if (!copySource) return;
    const lines = await api(`/sets/${copySource}/copy-preview`);
    await verify({ CopyExpected: { source_copy_id: copySource, lines } });
  }
  function difference(line: Data) {
    if (!line.finished) return 'Unfinished';
    if (line.expected === null) return 'Expected count unknown';
    const n = BigInt(line.verified) - BigInt(line.expected);
    return n === 0n ? 'Exact' : n > 0n ? `Excess ${n}` : `Short ${-n}`;
  }
  async function saveSetting(accept = false) {
    const action = settingEdit
      ? {
          Rename: {
            id: settingEdit.id,
            expected_revision: settingEdit.revision,
            name: settingName,
          },
        }
      : settingKind === 'Location'
        ? {
            CreateLocation: {
              id: uuid(),
              parent_id: parent || null,
              name: settingName,
            },
          }
        : {
            CreateVocabulary: {
              id: uuid(),
              vocabulary: settingKind,
              name: settingName,
            },
          };
    try {
      await send('settings', {
        ...envelope(),
        accept_name_warnings: accept,
        action,
      });
      settingName = '';
      settingEdit = null;
      acknowledge = false;
    } catch (e) {
      if (e instanceof Error && e.message.includes('acknowledg'))
        acknowledge = true;
      throw e;
    }
  }
  async function archiveSetting(record: Data) {
    await send('settings', {
      ...envelope(),
      accept_name_warnings: false,
      action: {
        Archive: {
          id: record.id,
          expected_revision: record.revision,
          archived: !record.archived,
        },
      },
    });
  }
  function editCatalog(record: Data) {
    templateRows = (record.details.expected_contents ?? []).map(
      (line: Data) => ({
        ...line,
        definition_id: line.definition_id ?? '',
        count: line.expected === null ? '' : String(line.expected),
      }),
    );
    catalogEdit = record;
    catalogKind = record.kind;
    catalogManufacturer = record.details.manufacturer_id;
    number = record.details.number ?? '';
    title = record.details.name ?? '';
    description = record.details.description ?? '';
    year =
      record.details.release_year === null
        ? ''
        : String(record.details.release_year);
    theme = record.details.theme ?? '';
    boxPieces =
      record.details.stated_piece_count === null
        ? ''
        : String(record.details.stated_piece_count);
  }
  async function saveCatalog() {
    const details = {
      expected_contents:
        catalogKind === 'Set'
          ? templateRows.map((line: Data) => ({
              definition_id: line.definition_id || null,
              color_id: line.color_id || unknown('Color'),
              expected: integer(line.count),
            }))
          : [],
      manufacturer_id: catalogManufacturer || unknown('Manufacturer'),
      number: number || null,
      name: title || null,
      description: description || null,
      release_year: catalogKind === 'Set' ? integer(year) : null,
      theme: catalogKind === 'Set' ? theme || null : null,
      stated_piece_count: catalogKind === 'Set' ? integer(boxPieces) : null,
    };
    const action = catalogEdit
      ? {
          Edit: {
            id: catalogEdit.id,
            expected_revision: catalogEdit.revision,
            details,
          },
        }
      : { Create: { id: uuid(), kind: catalogKind, details } };
    await send('catalog', { ...envelope(), action });
    templateRows = [];
    catalogEdit = null;
    number = '';
    title = '';
    description = '';
    year = '';
    theme = '';
    boxPieces = '';
  }
  async function upload(files: FileList | File[]) {
    if (!purchase?.revision)
      throw new Error('Save the purchase before attaching files.');
    for (const file of Array.from(files)) {
      const form = new FormData();
      form.set(
        'meta',
        JSON.stringify({
          ...envelope(),
          target: uploadTarget
            ? { InvoiceLine: uploadTarget }
            : { Purchase: purchase.id },
          original_name: file.name || 'pasted-image.png',
        }),
      );
      form.set('file', file);
      await api('/attachments', 'POST', form);
    }
    await refresh();
  }
  async function exportData(kind: string) {
    const result = await api(`/exchange/${kind}`, 'POST', {});
    const a = document.createElement('a');
    a.href = result.url;
    a.download =
      kind === 'native'
        ? 'brickxinator-native.zip'
        : 'brickxinator-inventory.csv';
    a.click();
  }
  async function importData(file: File | undefined) {
    if (!file || !importConfirmed) return;
    const form = new FormData();
    form.set('file', file);
    await api('/native-import', 'POST', form);
    await refresh();
    importConfirmed = false;
  }
  async function snapshot() {
    const result = await api('/snapshot', 'POST', {});
    notice = 'Snapshot validated. Download it below.';
    const link = document.createElement('a');
    link.href = result.url;
    link.download = 'brickxinator-snapshot.zip';
    link.click();
  }
  async function restore(file: File | undefined) {
    if (!file) return;
    if (!restoreConfirmed)
      throw new Error('Confirm replacement of the current collection first.');
    const form = new FormData();
    form.set('file', file);
    const result = await api('/restore', 'POST', form);
    purchase = null;
    selected = null;
    setId = '';
    setView = null;
    await refresh();
    notice = `Restored. Safety snapshot: ${result.safety_snapshot}`;
  }
  onMount(() => {
    const tab = uuid();
    let timer: ReturnType<typeof setInterval>;
    const beat = () =>
      api('/app-tabs/heartbeat', 'POST', { tab_id: tab }).catch(() => {});
    const leave = () => {
      fetch('/api/v1/app-tabs/disconnect', {
        method: 'POST',
        headers: {
          'content-type': 'application/json',
          'x-brickxinator-token': token,
        },
        body: JSON.stringify({ tab_id: tab }),
        keepalive: true,
      }).catch(() => {});
    };
    bootstrap()
      .then(async () => {
        await refresh();
        await beat();
        timer = setInterval(beat, 10000);
      })
      .catch((e) => (error = String(e)));
    window.addEventListener('pagehide', leave);
    const paste = (event: ClipboardEvent) => {
      if (page !== 'Purchases' || !purchase?.revision || !event.clipboardData)
        return;
      const files = event.clipboardData.files.length
        ? Array.from(event.clipboardData.files)
        : Array.from(event.clipboardData.items)
            .filter((item) => item.kind === 'file')
            .map((item) => item.getAsFile())
            .filter((file): file is File => file !== null);
      if (files.length) {
        event.preventDefault();
        void run(() => upload(files));
      }
    };
    window.addEventListener('paste', paste);
    return () => {
      clearInterval(timer);
      window.removeEventListener('pagehide', leave);
      window.removeEventListener('paste', paste);
      leave();
    };
  });
</script>

<svelte:head><title>Brickxinator · Collection workshop</title></svelte:head>
<header>
  <div class="brand">
    <span class="brickmark" aria-hidden="true">▦</span>
    <div>
      <strong>Brickxinator</strong><small>Your collection workshop</small>
    </div>
  </div>
  <form
    class="global-search"
    onsubmit={(e) => {
      e.preventDefault();
      offset = 0;
      page = 'Inventory';
      void run(search);
    }}
  >
    <input
      aria-label="Global collection search"
      bind:value={query}
      placeholder="Find parts, sets or storage"
      disabled={busy}
    /><button disabled={busy}>Find stock</button>
  </form>
  <span class="local">● Local &amp; offline</span>
</header>
<div class="layout">
  <nav aria-label="Main navigation">
    {#each pages as item}<button
        disabled={busy}
        class:active={page === item}
        onclick={() => (page = item)}>{item}</button
      >{/each}<small class="version">0.1.0-alpha.1 · Development</small>
  </nav>
  <main aria-busy={busy}>
    <div class="page-heading">
      <div>
        <span class="eyebrow">BRICKXINATOR WORKSHOP</span>
        <h1>{page}</h1>
      </div>
      <button disabled={busy} class="primary" onclick={newPurchase}
        >＋ New purchase</button
      >
    </div>
    {#if error}<div role="alert" class="message error">
        {error}
      </div>{/if}{#if notice}<div role="status" class="message">
        {notice}
      </div>{/if}
    <fieldset class="workflows" disabled={busy}>
      {#if data.backup_notice}<p class="muted" role="status">
          {data.backup_notice}
        </p>{/if}
      {#each data.backup_status ?? [] as status}{#if status.daily_error || status.monthly_error}<div
            role="alert"
            class="message error"
          >
            Backup needs attention. {status.daily_error
              ? `Daily: ${status.daily_error}`
              : ''}
            {status.monthly_error ? `Monthly: ${status.monthly_error}` : ''} Review
            per-destination status in Recovery.
          </div>{/if}{/each}
      {#if !data.dashboard}<p>Opening your collection…</p>
      {:else if page === 'Dashboard'}
        <div class="cards">
          <article>
            <small>Owned pieces</small><strong
              >{data.dashboard.inventory.all.owned_pieces.known_sum}</strong
            ><span
              >{data.dashboard.inventory.all.owned_pieces.unknown_holdings} holding(s)
              with unknown count</span
            >
          </article>
          <article>
            <small>Available pieces</small><strong
              >{data.dashboard.inventory.all.available_pieces.known_sum}</strong
            ><span
              >{data.dashboard.inventory.all.available_pieces.unknown_holdings} holding(s)
              with unknown availability</span
            >
          </article>
          <article>
            <small>Physical set copies</small><strong
              >{data.dashboard.inventory.known_set_copies}</strong
            ><span
              >{data.dashboard.inventory.unquantified_set_lines} invoice line(s) with
              unknown copy count</span
            >
          </article>
          <article>
            <small>Pending deliveries</small><strong
              >{data.dashboard.pending_deliveries}</strong
            ><span>Ordered or shipped</span>
          </article>
        </div>
        <div class="columns">
          <section>
            <h2>Recently recorded purchases</h2>
            {#if !data.dashboard.recent_purchases.length}<div class="empty">
                <h3>Start with what you know</h3>
                <p>
                  A blank draft is welcome. Add measurements, photos and prices
                  as you find them.
                </p>
                <button onclick={newPurchase}>Record a purchase</button>
              </div>{/if}{#each data.dashboard.recent_purchases as p}<button
                class="list-row"
                onclick={() =>
                  openPurchase(data.purchases.find((x: Data) => x.id === p.id))}
                ><strong>{p.seller || 'Unnamed purchase'}</strong><span
                  class="badge">{p.status}</span
                ><span>{money(p.total_cents)}</span></button
              >{/each}
          </section>
          <section>
            <h2>Measurement coverage</h2>
            <p>
              <strong>Current weight:</strong>
              {weightTotals(data.dashboard.inventory.all.owned_weight_mg)}
            </p>
            <p>
              <strong>Unidentified lots:</strong>
              {data.dashboard.inventory.unidentified_lots.holdings}
            </p>
            <p>
              <strong>Incomplete records:</strong>
              {data.dashboard.inventory.incomplete_records}
            </p>
            <p class="muted">
              Counts and weights describe the same stock. Box counts and
              expected checklists are kept separate from verified parts.
            </p>
          </section>
        </div>
      {:else if page === 'Purchases'}
        <section>
          <h2>Purchases</h2>
          <label class="check"
            ><input type="checkbox" bind:checked={includeArchived} /> Include archived
            records</label
          >{#each data.purchases.filter((p: Data) => includeArchived || !p.archived) as p}<button
              class="list-row"
              onclick={() => openPurchase(p)}
              ><strong>{p.draft.seller || 'Unnamed purchase'}</strong><span
                class="badge">{p.status}</span
              ><span>{p.draft.lines.length} invoice line(s)</span></button
            >{/each}
        </section>
        {#if purchase}<section>
            <div class="section-heading">
              <h2>
                {purchase.revision ? 'Purchase details' : 'New purchase draft'}
              </h2>
              <span class="badge">{purchase.status}</span>
            </div>
            <form
              onsubmit={(e) => {
                e.preventDefault();
                void run(savePurchase);
              }}
            >
              <fieldset
                disabled={(purchase.status === 'Received' && !correcting) ||
                  busy ||
                  !!correctionProposal}
              >
                <div class="form-grid">
                  <label
                    >Seller<input
                      bind:value={purchaseForm.seller}
                      placeholder="Unknown"
                    /></label
                  ><label
                    >Seller type<select bind:value={purchaseForm.seller_type}
                      ><option value="">Unknown</option
                      >{#each [['PrivateSeller', 'Private seller'], ['BusinessSeller', 'Business seller'], ['EbayPrivateSeller', 'eBay private seller'], ['EbayBusinessSeller', 'eBay business seller'], ['Marketplace', 'Marketplace'], ['RetailStore', 'Retail store'], ['BrickDealer', 'Brick dealer']] as choice}<option
                          value={choice[0]}>{choice[1]}</option
                        >{/each}</select
                    ></label
                  ><label
                    >Seller country<input
                      bind:value={purchaseForm.seller_country}
                    /></label
                  ><label
                    >Purchase date<input
                      type="date"
                      bind:value={purchaseForm.purchased_on}
                    /></label
                  ><label
                    >Tax presentation<select
                      bind:value={purchaseForm.tax_presentation}
                      ><option value="">Unknown</option><option value="Included"
                        >Included</option
                      ><option value="NotApplicable">Not applicable</option
                      ><option value="AddedSeparately"
                        >Added separately on invoice</option
                      ></select
                    ></label
                  ><label
                    >Informational tax · EUR<input
                      bind:value={purchaseForm.tax_amount}
                      inputmode="decimal"
                    /><small
                      >Already included in paid amounts; never added again.</small
                    ></label
                  ><label
                    >Order reference<input
                      bind:value={purchaseForm.order_number}
                    /></label
                  ><label
                    >Source URL<input
                      bind:value={purchaseForm.source_url}
                      type="url"
                    /></label
                  ><label
                    >Shipping · EUR<input
                      bind:value={purchaseForm.shipping}
                      inputmode="decimal"
                      placeholder="Unknown"
                    /></label
                  ><label
                    >Status<select
                      bind:value={purchase.status}
                      disabled={purchase.status === 'Received'}
                      >{#each ['Draft', 'Ordered', 'Shipped', 'Cancelled'] as status}<option
                          >{status}</option
                        >{/each}{#if purchase.status === 'Received'}<option
                          >Received</option
                        >{/if}</select
                    ></label
                  >
                </div>
                <label
                  >Notes<textarea bind:value={purchaseForm.notes}
                  ></textarea></label
                >
                <h3>Invoice lines</h3>
                <p class="muted">
                  Prices, piece counts and weights apply to the entire invoice
                  line. Physical set copies are counted separately; box counts
                  never create owned parts.
                </p>
                {#each purchaseForm.lines as line, i (line.id)}<div
                    class="invoice-line"
                  >
                    <div class="form-grid">
                      <label
                        >Kind<select bind:value={line.kind}
                          ><option value="LooseBatch">Loose batch</option
                          ><option>MixedLot</option><option>Set</option></select
                        ></label
                      ><label
                        >Description<input
                          bind:value={line.description}
                        /></label
                      ><label
                        >Paid line total · EUR<input
                          bind:value={line.price}
                          inputmode="decimal"
                          placeholder="Unknown"
                        /></label
                      ><label
                        >Brick count<input
                          bind:value={line.pieces}
                          inputmode="numeric"
                          placeholder="Unknown"
                        /></label
                      ><label
                        >Weight · g<input
                          bind:value={line.grams}
                          inputmode="decimal"
                          placeholder="Unknown"
                        /></label
                      ><label
                        >Item source URL<input
                          type="url"
                          bind:value={line.source_url}
                        /></label
                      ><label
                        >Item notes<textarea bind:value={line.notes}
                        ></textarea></label
                      >
                      {#if line.kind === 'Set'}<label
                          >Set number<input
                            bind:value={line.set_number}
                          /></label
                        ><label
                          >Physical copies<input
                            bind:value={line.copies}
                            inputmode="numeric"
                            placeholder="Unknown"
                          /></label
                        ><label
                          >Assembly<select bind:value={line.assembly}
                            ><option>Loose</option><option>Assembled</option
                            ><option value="PartiallyAssembled"
                              >Partially assembled</option
                            ></select
                          ></label
                        >{/if}
                    </div>
                    <button
                      type="button"
                      disabled={purchase.status === 'Received'}
                      class="quiet danger"
                      onclick={() =>
                        (purchaseForm.lines = purchaseForm.lines.filter(
                          (_: Data, index: number) => index !== i,
                        ))}>Remove line</button
                    >
                  </div>{/each}<button
                  type="button"
                  disabled={purchase.status === 'Received'}
                  onclick={addPurchaseLine}>＋ Add invoice line</button
                >
                <div class="actions">
                  <button class="primary" type="submit"
                    >{purchase.status === 'Received'
                      ? 'Preview correction'
                      : 'Save draft / changes'}</button
                  >
                </div>
              </fieldset>
            </form>
            {#if correctionProposal}<section>
                <h3>Review received-purchase correction</h3>
                <p>
                  Only the shown residual changes will be applied. Classified
                  child stock and the original receipt stay preserved.
                </p>
                <div class="table-wrap">
                  <table>
                    <thead
                      ><tr><th>Field</th><th>Previous</th><th>Corrected</th></tr
                      ></thead
                    ><tbody
                      >{#each auditChanges( { before: purchase.draft, after: correctionProposal.draft } ) as change}<tr
                          ><td>{change.field}</td><td>{change.before}</td><td
                            >{change.after}</td
                          ></tr
                        >{/each}</tbody
                    >
                  </table>
                </div>
                <h3>Residual stock effects</h3>
                {#each correctionProposal.preview.effects as effect, i}<p>
                    Residual holding {i + 1}: {known(
                      effect.before.pieces.owned,
                    ) || 'Unknown'} → {known(effect.after.pieces.owned) ||
                      'Unknown'} pieces; {weightMeasure(
                      effect.before.weight_mg.owned,
                    )} → {weightMeasure(effect.after.weight_mg.owned)}.
                  </p>{/each}{#if !correctionProposal.preview.effects.length}<p>
                    No loose or mixed-lot stock measurements change.
                  </p>{/if}{#each correctionProposal.preview.copy_additions as addition}<p
                  >
                    New independent physical copies: {addition.copies
                      .map((copy: Data) => copy.label)
                      .join(', ')}. No part stock is created.
                  </p>{/each}{#each correctionProposal.preview.warnings as warning}<p
                  >
                    {warning}
                  </p>{/each}<label
                  >Correction reason<input
                    bind:value={correctionReason}
                    required
                  /></label
                ><button
                  class="primary"
                  disabled={!correctionReason.trim()}
                  onclick={() => run(applyCorrection)}
                  >Apply reviewed correction</button
                ><button onclick={() => (correctionProposal = null)}
                  >Revise correction</button
                >
              </section>{/if}
            {#if purchase.revision}<div class="actions">
                {#if purchase.status === 'Received' && !correcting}<button
                    onclick={() => (correcting = true)}
                    >Correct received purchase</button
                  >{/if}{#if correcting}<button
                    onclick={() => openPurchase(purchase!)}
                    >Cancel correction</button
                  >{/if}<button onclick={() => run(archivePurchase)}
                  >{purchase.archived ? 'Unarchive' : 'Archive'} purchase</button
                >{#if purchase.status !== 'Received' && purchase.status !== 'Cancelled'}<button
                    class="primary"
                    disabled={busy}
                    onclick={() => run(receive)}>Receive whole purchase</button
                  >{/if}<button onclick={() => run(loadCosts)}
                  >Shipping &amp; costs</button
                ><button
                  onclick={() =>
                    run(
                      async () =>
                        (history = await api(
                          `/history/purchase/${purchase!.id}`,
                        )),
                    )}>View audit history</button
                >
              </div>
              {#if purchase.status !== 'Received' && purchaseForm.lines.length === 0 && !data.attachments.some((a: Data) => a.target.Purchase === purchase!.id)}<div
                  class="actions"
                >
                  <label class="check"
                    ><input type="checkbox" bind:checked={purgeConfirmed} /> Permanently
                    delete this draft and its entered metadata</label
                  ><button
                    class="danger"
                    disabled={!purgeConfirmed || busy}
                    onclick={() => run(purgePurchase)}
                    >Delete draft permanently</button
                  >
                </div>{/if}{#if purchase.status === 'Received'}<p class="muted">
                  Received once. Preview acquisition corrections here; use
                  verification or adjustments for later physical stock changes.
                  Receipt cannot be repeated.
                </p>{/if}
              <h3>Images &amp; documents</h3>
              <p class="muted">
                Upload files or paste an image. Originals remain unchanged.
              </p>
              <label
                >Attach to<select bind:value={uploadTarget}
                  ><option value="">Whole purchase</option
                  >{#each purchaseForm.lines as l, i}<option value={l.id}
                      >Line {i + 1}: {l.description || l.kind}</option
                    >{/each}</select
                ></label
              ><label
                >Choose images or PDFs<input
                  type="file"
                  multiple
                  accept="image/png,image/jpeg,image/gif,image/webp,application/pdf"
                  onchange={(e) => {
                    if (e.currentTarget.files)
                      void run(() => upload(e.currentTarget.files!));
                  }}
                /></label
              >
              <div class="attachments">
                {#each data.attachments.filter((a: Data) => a.target.Purchase === purchase!.id || purchaseForm.lines.some((l: Data) => a.target.InvoiceLine === l.id)) as a}<a
                    href={`/api/v1/attachments/${a.id}`}
                    target="_blank"
                    rel="noopener noreferrer"
                    >{#if a.media_type.startsWith('image/')}<img
                        src={`/api/v1/attachments/${a.id}`}
                        alt={a.original_name}
                      />{/if}<span>{a.original_name}</span></a
                  >{/each}
              </div>{/if}
          </section>{/if}
        {#if costs}<section>
            <h2>Shipping allocation &amp; costs</h2>
            <label
              >Unit-price decimal places<select bind:value={ratePlaces}
                >{#each [0, 1, 2, 3, 4, 5, 6] as places}<option value={places}
                    >{places}</option
                  >{/each}</select
              ></label
            ><label
              >Allocation method<select
                value={costs.state.method}
                onchange={(e) => run(() => shipping(e.currentTarget.value))}
                >{#each ['PurchaseLevel', 'Equal', 'PriceProportional', 'PieceCount', 'Weight', 'Manual'] as method}<option
                    >{method}</option
                  >{/each}</select
              ></label
            >
            <p>
              <span class="badge">{costs.allocation.status}</span> Purchase
              total: <strong>{money(costs.purchase_total_cents)}</strong>
            </p>
            {#if costs.allocation.issues.length}<p class="muted">
                Allocation is incomplete. Check missing bases, manual shares and
                the remainder before using allocated costs.
              </p>{/if}
            <div class="table-wrap">
              <table>
                <thead
                  ><tr
                    ><th>Line</th><th>Paid total</th><th>Shipping share</th><th
                      >Allocated total</th
                    ><th>Paid EUR / piece</th><th>Paid EUR / kg</th><th
                      >Allocated EUR / piece</th
                    ><th>Allocated EUR / kg</th
                    >{#if costs.state.method === 'Manual'}<th
                        >Manual share · EUR</th
                      >{/if}</tr
                  ></thead
                ><tbody
                  >{#each costs.lines as l, i}<tr
                      ><td>{i + 1}</td><td>{money(l.paid_total_cents)}</td><td
                        >{money(l.shipping_share_cents)}</td
                      ><td>{money(l.allocated_total_cents)}</td><td
                        >{rate(
                          l.paid_unit_costs?.eur_per_piece,
                          ratePlaces,
                        )}</td
                      ><td>{rate(l.paid_unit_costs?.eur_per_kg, ratePlaces)}</td
                      ><td
                        >{rate(
                          l.allocated_unit_costs?.eur_per_piece,
                          ratePlaces,
                        )}</td
                      ><td
                        >{rate(
                          l.allocated_unit_costs?.eur_per_kg,
                          ratePlaces,
                        )}</td
                      >{#if costs.state.method === 'Manual'}<td
                          ><input
                            aria-label={`Manual shipping share line ${i + 1}`}
                            bind:value={manual[l.invoice_line_id]}
                            inputmode="decimal"
                          /></td
                        >{/if}</tr
                    >{/each}</tbody
                >
              </table>
            </div>
            {#if costs.state.method === 'Manual'}<p>
                Remainder: {costs.allocation.manual_remainder === 'Balanced'
                  ? 'Balanced'
                  : costs.allocation.manual_remainder?.Unallocated
                    ? `${money(costs.allocation.manual_remainder.Unallocated)} unallocated`
                    : costs.allocation.manual_remainder?.Overallocated
                      ? `${money(costs.allocation.manual_remainder.Overallocated)} overallocated`
                      : 'Shipping unknown'}
              </p>
              <button onclick={() => run(saveManual)}>Save manual shares</button
              >{/if}
          </section>{/if}
      {:else if page === 'Inventory'}
        <section>
          <form
            class="search"
            onsubmit={(e) => {
              e.preventDefault();
              offset = 0;
              void run(search);
            }}
          >
            <label
              >Number or text<input
                bind:value={query}
                placeholder="Part number, description, seller or storage"
              /></label
            ><label
              >Storage subtree<select bind:value={storage}
                ><option value="">All storage</option
                >{#each locations() as r}<option value={r.id}
                    >{locationPath(r.id)}</option
                  >{/each}</select
              ></label
            ><label
              >Availability<select bind:value={availability}
                ><option value="">All stock</option
                >{#each ['Available', 'InAssembledSet', 'BrokenDamaged', 'Unknown'] as f}<option
                    >{f}</option
                  >{/each}</select
              ></label
            ><button class="primary">Search</button>
            <details>
              <summary>More filters</summary>
              <div class="form-grid">
                <label
                  >Assembly state<select bind:value={assemblyFilter}
                    ><option value="">All assembly states</option
                    >{#each ['Loose', 'Assembled', 'PartiallyAssembled'] as value}<option
                        {value}>{human(value)}</option
                      >{/each}</select
                  ></label
                ><label
                  >Record kind<select bind:value={kindFilter}
                    ><option value="">All record kinds</option
                    >{#each ['LooseBatch', 'MixedLot', 'IdentifiedBatch', 'SetContent', 'SetCopy', 'UnquantifiedSetLine', 'OpeningHolding'] as value}<option
                        {value}>{human(value)}</option
                      >{/each}</select
                  ></label
                ><label class="check"
                  ><input type="checkbox" bind:checked={incompleteOnly} /> Incomplete
                  records only</label
                >
              </div>
              <label class="check"
                ><input type="checkbox" bind:checked={includeArchived} /> Include
                archived records</label
              >
              <div class="form-grid">
                <label>Seller contains<input bind:value={sellerFilter} /></label
                ><label
                  >Purchased from<input
                    type="date"
                    bind:value={purchasedFrom}
                  /></label
                ><label
                  >Purchased through<input
                    type="date"
                    bind:value={purchasedThrough}
                  /></label
                >{#each [['Manufacturer', 'brand'], ['Color', 'color'], ['Condition', 'condition']] as pair}<label
                    >{pair[0]}<select
                      value={pair[1] === 'brand'
                        ? brandFilter
                        : pair[1] === 'color'
                          ? colorFilter
                          : conditionFilter}
                      onchange={(e) => {
                        if (pair[1] === 'brand')
                          brandFilter = e.currentTarget.value;
                        else if (pair[1] === 'color')
                          colorFilter = e.currentTarget.value;
                        else conditionFilter = e.currentTarget.value;
                      }}
                      ><option value="">All</option
                      >{#each vocab(pair[0]) as r}<option value={r.id}
                          >{r.name}{r.archived ? ' (archived)' : ''}</option
                        >{/each}</select
                    ></label
                  >{/each}
              </div>
            </details>
          </form>
        </section>
        <section>
          <div class="section-heading">
            <h2>{inventory.total_matches} matching records</h2>
            <span
              >{totals(inventory.totals?.all.owned_pieces)} pieces across all matches</span
            >
          </div>
          <div class="table-wrap">
            <table>
              <thead
                ><tr
                  ><th>Item</th><th>Owned / available</th><th>Storage</th><th
                    >State</th
                  ><th></th></tr
                ></thead
              ><tbody
                >{#each inventory.rows as row}<tr
                    ><td
                      ><strong>{row.number || row.label}</strong><small
                        >{row.label} · {row.manufacturer.name} · {row.color
                          .name}</small
                      ></td
                    ><td
                      >{row.holding
                        ? `${known(row.holding.pieces.owned) || 'Unknown'} owned`
                        : 'Physical copy / source line'}{#if row.holding}<small
                          >{row.assembly !== 'Loose'
                            ? '0 available (assembled)'
                            : known(row.holding.pieces.owned) &&
                                known(row.holding.pieces.damaged)
                              ? `${BigInt(known(row.holding.pieces.owned)) - BigInt(known(row.holding.pieces.damaged))} available`
                              : 'Availability unknown'}</small
                        ><small
                          >Owned weight: {weightMeasure(
                            row.holding.weight_mg.owned,
                          )} · available: {availableWeight(row)}</small
                        >{/if}</td
                    ><td
                      >{row.location.map((r: Data) => r.name).join(' / ') ||
                        'Unknown'}</td
                    ><td
                      >{human(row.kind)}<small
                        >{human(row.assembly)}{row.incomplete
                          ? ' · Incomplete'
                          : ''}</small
                      ></td
                    ><td
                      >{#if row.target.SetCopy}<button
                          onclick={() => run(() => openSet(row.target.SetCopy))}
                          >Verify set</button
                        >{:else if row.set_copy_id}<button
                          onclick={() => run(() => openSet(row.set_copy_id))}
                          >View parent set</button
                        ><button onclick={() => selectInventory(row)}
                          >Adjust</button
                        >{:else if row.target.Holding}<button
                          onclick={() => selectInventory(row)}>Manage</button
                        >{/if}{#if row.purchase_id}<button
                          onclick={() =>
                            openPurchase(
                              data.purchases.find(
                                (p: Data) => p.id === row.purchase_id,
                              ),
                            )}>View source purchase</button
                        >{/if}</td
                    ></tr
                  >{/each}</tbody
              >
            </table>
          </div>
          <div class="actions">
            <button
              disabled={offset === 0}
              onclick={() =>
                run(async () => {
                  offset = Math.max(0, offset - 50);
                  await search();
                })}>Previous</button
            ><span
              >{inventory.total_matches ? offset + 1 : 0}–{Math.min(
                offset + 50,
                inventory.total_matches,
              )}</span
            ><button
              disabled={offset + 50 >= inventory.total_matches}
              onclick={() =>
                run(async () => {
                  offset += 50;
                  await search();
                })}>Next</button
            >
          </div>
        </section>
        {#if selected}<section>
            <div class="section-heading">
              <h2>Manage {selected.number || selected.label}</h2>
              <button onclick={() => (selected = null)}>Close</button>
            </div>
            {#if selected.kind !== 'SetContent'}<h3>Catalog &amp; storage</h3>
              <div class="form-grid">
                <label
                  >{selected.kind === 'SetCopy'
                    ? 'Set definition'
                    : 'Part definition'}<select bind:value={links.definition_id}
                    ><option value="">Unlinked / Unknown</option
                    >{#each definitions(selected.kind === 'SetCopy' ? 'Set' : 'Part') as d}<option
                        value={d.id}>{label(d)}</option
                      >{/each}</select
                  ></label
                ><label
                  >Color<select bind:value={links.color_id}
                    >{#each vocab('Color') as r}<option value={r.id}
                        >{r.name}</option
                      >{/each}</select
                  ></label
                ><label
                  >Condition<select bind:value={links.condition_id}
                    >{#each vocab('Condition') as r}<option value={r.id}
                        >{r.name}</option
                      >{/each}</select
                  ></label
                ><label
                  >Location<select bind:value={links.location_id}
                    ><option value="">Unknown</option
                    >{#each locations() as r}<option value={r.id}
                        >{locationPath(r.id)}</option
                      >{/each}</select
                  ></label
                >
              </div>
              <button onclick={() => run(saveLinks)}
                >Save links / move whole holding</button
              >{/if}
            <button onclick={() => run(archiveSelected)}
              >{links.archived ? 'Unarchive' : 'Archive'} record</button
            >{#if selected.holding}<h3>Stock adjustment</h3>
              <div class="form-grid">
                <label
                  >Action<select bind:value={operation}
                    >{#each ['Broken', 'Lost', 'Donated', 'Sold', 'Correction'] as a}<option
                        >{a}</option
                      >{/each}</select
                  ></label
                ><label
                  >{operation === 'Correction'
                    ? 'Absolute owned pieces'
                    : operation === 'Broken'
                      ? 'Newly broken pieces'
                      : 'Removed pieces'}<input
                    disabled={operation === 'Correction' && unknownPieces}
                    bind:value={pieces}
                    inputmode="numeric"
                    placeholder="Unmeasured"
                  /></label
                ><label
                  >{operation === 'Correction'
                    ? 'Absolute owned weight · g'
                    : operation === 'Broken'
                      ? 'Newly broken weight · g'
                      : 'Removed weight · g'}<input
                    disabled={operation === 'Correction' && unknownWeight}
                    bind:value={grams}
                    inputmode="decimal"
                    placeholder="Unmeasured"
                  /></label
                ><label
                  >{operation === 'Correction'
                    ? 'Absolute damaged pieces'
                    : 'Damaged pieces included in removal'}<input
                    disabled={operation === 'Broken' ||
                      (operation === 'Correction' && unknownDamage)}
                    bind:value={damaged}
                    inputmode="numeric"
                    placeholder="0"
                  /></label
                ><label
                  >{operation === 'Correction'
                    ? 'Absolute damaged weight · g'
                    : 'Damaged weight included in removal · g'}<input
                    disabled={operation === 'Broken' ||
                      (operation === 'Correction' && unknownDamageWeight)}
                    bind:value={damagedGrams}
                    inputmode="decimal"
                    placeholder="0"
                  /></label
                >
              </div>
              {#if operation === 'Correction'}<p class="muted">
                  Blank fields keep their current measurement. Explicitly mark a
                  measurement unknown below.
                </p>
                <div class="form-grid">
                  <label class="check"
                    ><input type="checkbox" bind:checked={unknownPieces} /> Owned
                    piece count unknown</label
                  ><label class="check"
                    ><input type="checkbox" bind:checked={unknownWeight} /> Owned
                    weight unknown</label
                  ><label class="check"
                    ><input type="checkbox" bind:checked={unknownDamage} /> Damaged
                    piece count unknown</label
                  ><label class="check"
                    ><input
                      type="checkbox"
                      bind:checked={unknownDamageWeight}
                    /> Damaged weight unknown</label
                  >
                </div>{:else}<p class="muted">
                  Leave a unit blank when unmeasured; corresponding remaining or
                  damage measurements may become unknown.
                </p>{/if}<label
                >Reason<input
                  bind:value={reason}
                  placeholder="Required for a stock change"
                /></label
              ><button onclick={() => run(adjust)}
                >Record stock adjustment</button
              >
              {#if selected.kind === 'MixedLot'}<h3>Gradual identification</h3>
                <p>
                  Record the units extracted into a separate child. Unmeasured
                  residual dimensions become unknown.
                </p>
                <div class="form-grid">
                  <label
                    >Identified pieces<input
                      bind:value={identifiedPieces}
                      inputmode="numeric"
                      placeholder="Unmeasured"
                    /></label
                  ><label
                    >Identified weight · g<input
                      bind:value={identifiedGrams}
                      inputmode="decimal"
                      placeholder="Unmeasured"
                    /></label
                  ><label
                    >Identification reason<input
                      bind:value={identificationReason}
                    /></label
                  >
                </div>
                <label
                  >Child part definition (optional)<select
                    bind:value={childPart}
                    ><option value="">Link later</option
                    >{#each definitions('Part') as d}<option value={d.id}
                        >{label(d)}</option
                      >{/each}</select
                  ></label
                ><button onclick={() => run(identify)}
                  >Extract identified child</button
                >
                <h3>Complete classification</h3>
                <label class="check"
                  ><input type="checkbox" bind:checked={physicalEmpty} /> I confirm
                  the physical residual lot is empty</label
                ><button onclick={() => run(closeLot)}
                  >Complete classification</button
                >{/if}<button
                class="quiet"
                onclick={() =>
                  run(
                    async () =>
                      (history = await api(
                        `/history/holding/${targetId(selected!)}`,
                      )),
                  )}>View stock history</button
              >{/if}
          </section>{/if}
        <section>
          <h2>Completed mixed lots</h2>
          <p class="muted">
            Closed residual stock is excluded from current inventory. Reopening
            preserves its last measurements.
          </p>
          {#each data.closed_lots as lot}<div class="list-row">
              <strong>{lot.label}</strong><button
                onclick={() =>
                  run(() =>
                    send('stock', {
                      ...envelope(),
                      reason: 'Reopen completed classification',
                      holding_id: lot.id,
                      expected_revision: lot.revision,
                      action: 'Reopen',
                    }),
                  )}>Reopen classification</button
              >
            </div>{/each}
        </section>
      {:else if page === 'Sets'}
        <section>
          <h2>Owned set copies</h2>
          <label class="check"
            ><input type="checkbox" bind:checked={includeArchived} /> Include archived
            records</label
          >{#each data.set_copies.filter((r: Data) => includeArchived || !r.archived) as row}<div
              class="list-row"
            >
              <button onclick={() => run(() => openSet(row.id))}
                ><strong>{row.label}</strong><span class="badge"
                  >{human(row.assembly)}</span
                ></button
              ><button onclick={() => manageCopy(row)}
                >Catalog, storage &amp; archive</button
              >
            </div>{/each}
        </section>
        <!-- Set copy identity survives renaming. -->{#if setView}<section>
            <h2>Set checklist</h2>
            <form
              onsubmit={(e) => {
                e.preventDefault();
                void run(() => verify({ Rename: { label: copyLabel } }));
              }}
            >
              <label
                >Copy label<input
                  bind:value={copyLabel}
                  required
                  maxlength="200"
                /></label
              ><button>Rename copy</button>
            </form>
            <div class="actions">
              <label
                >Whole-set assembly<select
                  value={setView.verification.assembly}
                  onchange={(e) =>
                    run(() =>
                      verify({ Assembly: { state: e.currentTarget.value } }),
                    )}
                  ><option>Loose</option><option>Assembled</option><option
                    >PartiallyAssembled</option
                  ></select
                ></label
              ><button onclick={() => run(() => verify('Complete'))}
                >Complete verification</button
              ><button
                onclick={() =>
                  run(
                    async () => (history = await api(`/history/set/${setId}`)),
                  )}>Audit history</button
              >
            </div>
            <p class="badge">
              {setView.verification.completed ? 'Completed' : 'Not completed'}
            </p>
            <p class="muted">
              Expected counts create no stock. Changing an expected or counted
              total reopens finished work.
            </p>
            {#if recountProposal}<section>
                <h3>Review fresh physical recount</h3>
                <p>
                  The last verified observation differs from current stock after
                  an adjustment. This recount will set current owned pieces from {known(
                    recountProposal.expected_owned,
                  ) || 'Unknown'} to {recountProposal.total}; previous movements
                  remain in history.
                </p>
                <p>Reason: {recountProposal.reason}</p>
                <button class="primary" onclick={() => run(applyRecount)}
                  >Apply reviewed physical recount</button
                ><button onclick={() => (recountProposal = null)}
                  >Cancel recount</button
                >
              </section>{/if}<label class="check"
              ><input type="checkbox" bind:checked={showRetired} /> Show retired checklist
              lines</label
            >
            <p class="muted">
              Retiring a checklist line preserves its verified stock. Assembly
              still applies to those owned contents.
            </p>
            <div class="table-wrap">
              <table>
                <thead
                  ><tr
                    ><th>Part / color</th><th>Expected</th><th
                      >Verified total</th
                    ><th>State</th><th></th></tr
                  ></thead
                ><tbody
                  >{#each Object.values(setView.verification.lines).filter((line: unknown) => showRetired || !(line as Data).retired) as line}<tr
                      ><td
                        ><label
                          >Checklist part<select
                            disabled={(line as Data).retired}
                            value={(line as Data).definition_id ?? ''}
                            onchange={(e) =>
                              run(() =>
                                verify({
                                  Identity: {
                                    line_id: (line as Data).id,
                                    definition_id:
                                      e.currentTarget.value || null,
                                    color_id: (line as Data).color_id,
                                  },
                                }),
                              )}
                            ><option value="">Unknown</option
                            >{#each data.catalog.filter((d: Data) => d.kind === 'Part') as d}<option
                                value={d.id}
                                disabled={d.archived &&
                                  d.id !== (line as Data).definition_id}
                                >{label(d)}</option
                              >{/each}</select
                          ></label
                        ><label
                          >Checklist color<select
                            disabled={(line as Data).retired}
                            value={(line as Data).color_id}
                            onchange={(e) =>
                              run(() =>
                                verify({
                                  Identity: {
                                    line_id: (line as Data).id,
                                    definition_id: (line as Data).definition_id,
                                    color_id: e.currentTarget.value,
                                  },
                                }),
                              )}
                            >{#each vocab('Color') as c}<option
                                value={c.id}
                                disabled={c.archived &&
                                  c.id !== (line as Data).color_id}
                                >{c.name}</option
                              >{/each}</select
                          ></label
                        ></td
                      ><td
                        ><input
                          disabled={(line as Data).retired}
                          aria-label="Expected count"
                          value={(line as Data).expected ?? ''}
                          onchange={(e) =>
                            run(() =>
                              verify({
                                Expected: {
                                  line_id: (line as Data).id,
                                  expected: integer(e.currentTarget.value),
                                },
                              }),
                            )}
                        /></td
                      ><td
                        ><form
                          onsubmit={(e) => {
                            e.preventDefault();
                            const form = new FormData(e.currentTarget);
                            void run(() =>
                              recordCount(
                                line as Data,
                                integer(String(form.get('total'))),
                                String(form.get('reason')),
                              ),
                            );
                          }}
                        >
                          <input
                            disabled={(line as Data).retired}
                            aria-label="Verified total"
                            name="total"
                            value={(line as Data).verified ?? ''}
                            inputmode="numeric"
                            required
                          /><input
                            disabled={(line as Data).retired}
                            aria-label="Count reason"
                            name="reason"
                            placeholder="Count / correction reason"
                            required
                          /><button disabled={(line as Data).retired}
                            >Record count</button
                          >
                        </form></td
                      ><td
                        >{(line as Data).retired
                          ? 'Retired; stock retained'
                          : difference(line as Data)}</td
                      ><td
                        ><button
                          disabled={(line as Data).retired ||
                            (line as Data).verified === null}
                          onclick={() =>
                            run(() =>
                              verify({
                                Finish: { line_id: (line as Data).id },
                              }),
                            )}>Finish line</button
                        ><button
                          onclick={() =>
                            run(() =>
                              verify({
                                Active: {
                                  line_id: (line as Data).id,
                                  active: !!(line as Data).retired,
                                },
                              }),
                            )}
                          >{(line as Data).retired
                            ? 'Restore checklist line'
                            : 'Retire checklist line'}</button
                        ></td
                      ></tr
                    >{/each}</tbody
                >
              </table>
            </div>
            <h3>Use a definition’s expected checklist</h3>
            <label
              >Set definition template<select bind:value={templateSource}
                ><option value="">Choose a definition</option
                >{#each definitions('Set') as d}<option value={d.id}
                    >{label(d)}</option
                  >{/each}</select
              ></label
            ><button
              disabled={!templateSource}
              onclick={() => run(copyDefinitionChecklist)}
              >Add template as unverified expected lines</button
            >
            <h3>Add expected line</h3>
            <div class="form-grid">
              <label
                >Part<select bind:value={part}
                  ><option value="">Unknown</option
                  >{#each definitions('Part') as d}<option value={d.id}
                      >{label(d)}</option
                    >{/each}</select
                ></label
              ><label
                >Color<select bind:value={lineColor}
                  ><option value="">Unknown</option
                  >{#each active(vocab('Color')) as r}<option value={r.id}
                      >{r.name}</option
                    >{/each}</select
                ></label
              ><label
                >Expected pieces<input
                  bind:value={expected}
                  inputmode="numeric"
                  placeholder="Unknown"
                /></label
              >
            </div>
            <button onclick={() => run(addExpected)}>Add expected line</button>
            <h3>Copy another copy’s expected checklist</h3>
            <label
              >Source copy<select bind:value={copySource}
                ><option value="">Choose a copy</option
                >{#each data.set_copies.filter((r: Data) => r.id !== setId) as r}<option
                    value={r.id}>{r.label}</option
                  >{/each}</select
              ></label
            ><button disabled={!copySource} onclick={() => run(copyChecklist)}
              >Copy expected contents only</button
            >
          </section>{/if}
      {:else if page === 'Catalog'}
        <section>
          <h2>{catalogEdit ? 'Edit definition' : 'New definition'}</h2>
          <form
            onsubmit={(e) => {
              e.preventDefault();
              void run(saveCatalog);
            }}
          >
            <div class="form-grid">
              <label
                >Kind<select bind:value={catalogKind} disabled={!!catalogEdit}
                  ><option>Part</option><option>Set</option></select
                ></label
              ><label
                >Manufacturer<select bind:value={catalogManufacturer}
                  ><option value="">Unknown</option
                  >{#each active(vocab('Manufacturer')) as r}<option
                      value={r.id}>{r.name}</option
                    >{/each}</select
                ></label
              ><label>Number<input bind:value={number} /></label><label
                >Name<input bind:value={title} /></label
              >{#if catalogKind === 'Set'}<label
                  >Release year<input bind:value={year} /></label
                ><label>Theme<input bind:value={theme} /></label><label
                  >Stated piece count<input bind:value={boxPieces} /></label
                >{/if}
            </div>
            <label
              >Description<textarea bind:value={description}></textarea></label
            >{#if catalogKind === 'Set'}<h3>Reusable expected checklist</h3>
              <p class="muted">
                Template counts never create owned stock. Copying creates
                independent unverified lines.
              </p>
              {#each templateRows as line, i}<div class="form-grid">
                  <label
                    >Template part<select bind:value={line.definition_id}
                      ><option value="">Unknown</option
                      >{#each definitions('Part') as d}<option value={d.id}
                          >{label(d)}</option
                        >{/each}</select
                    ></label
                  ><label
                    >Template color<select bind:value={line.color_id}
                      ><option value="">Unknown</option
                      >{#each active(vocab('Color')) as c}<option value={c.id}
                          >{c.name}</option
                        >{/each}</select
                    ></label
                  ><label
                    >Template expected count<input
                      bind:value={line.count}
                      inputmode="numeric"
                      placeholder="Unknown"
                    /></label
                  ><button
                    type="button"
                    onclick={() =>
                      (templateRows = templateRows.filter(
                        (_, index) => index !== i,
                      ))}>Remove template line</button
                  >
                </div>{/each}<button
                type="button"
                onclick={() =>
                  (templateRows = [
                    ...templateRows,
                    { definition_id: '', color_id: '', count: '' },
                  ])}>Add template line</button
              >{/if}<button class="primary">Save definition</button><button
              type="button"
              onclick={() => {
                catalogEdit = null;
                templateRows = [];
                number = '';
                title = '';
                description = '';
              }}>New definition</button
            >
          </form>
          <p class="muted">
            Definitions describe items and never create owned stock.
          </p>
        </section>
        <section>
          <h2>Definitions</h2>
          {#each data.catalog as d}<div class="list-row">
              <div>
                <strong>{label(d)}</strong><small
                  >{d.kind}{d.archived ? ' · Archived' : ''}</small
                >
              </div>
              <button onclick={() => editCatalog(d)}>Edit</button><button
                onclick={() =>
                  run(
                    async () => (priceHistory = await api(`/prices/${d.id}`)),
                  )}>Purchase price history</button
              ><button
                onclick={() =>
                  run(() =>
                    send('catalog', {
                      ...envelope(),
                      action: {
                        Archive: {
                          id: d.id,
                          expected_revision: d.revision,
                          archived: !d.archived,
                        },
                      },
                    }),
                  )}>{d.archived ? 'Unarchive' : 'Archive'}</button
              >
            </div>{/each}
        </section>
        {#if priceHistory}<section>
            <div class="section-heading">
              <h2>Historical purchase prices</h2>
              <button onclick={() => (priceHistory = null)}>Close prices</button
              >
            </div>
            <p class="muted">
              Each amount is the entire invoice-line total, including grouped
              copies. Mixed-lot source prices are not attributed to an
              individual part.
            </p>
            <div class="table-wrap">
              <table>
                <thead
                  ><tr
                    ><th>Purchase date / seller</th><th>Paid line total</th><th
                      >Basis</th
                    ><th>Paid EUR / piece</th><th>Paid EUR / kg</th><th
                      >Revision</th
                    ></tr
                  ></thead
                ><tbody
                  >{#each priceHistory as price}<tr
                      ><td
                        >{price.purchased_on ?? 'Unknown date'}<small
                          >{price.seller ?? 'Unknown seller'}</small
                        ></td
                      ><td>{money(price.paid_total_cents)}</td><td
                        >{price.kind === 'Set'
                          ? `${price.copy_count ?? 'Unknown'} copies on invoice line`
                          : price.attributable
                            ? 'Purchased loose batch'
                            : 'Mixed-lot source; unallocated'}</td
                      ><td>{rate(price.paid_unit_costs?.eur_per_piece, 4)}</td
                      ><td>{rate(price.paid_unit_costs?.eur_per_kg, 4)}</td><td
                        >{price.current
                          ? 'Current corrected value'
                          : 'Earlier value'}<small>{price.at}</small></td
                      ></tr
                    >{/each}</tbody
                >
              </table>
            </div>
          </section>{/if}
      {:else if page === 'Settings'}
        <section>
          <h2>Managed names &amp; storage</h2>
          <form
            onsubmit={(e) => {
              e.preventDefault();
              void run(() => saveSetting(false));
            }}
          >
            <div class="form-grid">
              <label
                >Category<select
                  bind:value={settingKind}
                  disabled={!!settingEdit}
                  >{#each ['Manufacturer', 'Color', 'Condition', 'Location'] as kind}<option
                      >{kind}</option
                    >{/each}</select
                ></label
              ><label>Name<input bind:value={settingName} required /></label
              >{#if settingKind === 'Location' && !settingEdit}<label
                  >Parent location<select bind:value={parent}
                    ><option value="">Top level</option
                    >{#each locations() as r}<option value={r.id}
                        >{locationPath(r.id)}</option
                      >{/each}</select
                  ></label
                >{/if}
            </div>
            <button class="primary">Save name</button>{#if acknowledge}<button
                type="button"
                onclick={() => run(() => saveSetting(true))}
                >Keep this spelling despite name warning</button
              >{/if}{#if settingEdit}<button
                type="button"
                onclick={() => {
                  settingEdit = null;
                  settingName = '';
                }}>Cancel edit</button
              >{/if}
          </form>
        </section>
        <section>
          <h2>{settingKind} records</h2>
          {#each data.settings.filter( (r: Data) => (settingKind === 'Location' ? r.kind.Location : r.kind.Vocabulary?.vocabulary === settingKind) ) as r}<div
              class="list-row"
            >
              <div>
                <strong>{r.kind.Location ? locationPath(r.id) : r.name}</strong
                ><small
                  >{r.archived
                    ? 'Archived'
                    : r.kind.Vocabulary?.system_unknown
                      ? 'Protected Unknown'
                      : 'Active'}</small
                >
              </div>
              {#if !r.kind.Vocabulary?.system_unknown}<button
                  onclick={() => {
                    settingEdit = r;
                    settingName = r.name;
                  }}>Rename</button
                ><button onclick={() => run(() => archiveSetting(r))}
                  >{r.archived ? 'Unarchive' : 'Archive'}</button
                >{/if}
            </div>{/each}
        </section>
      {:else if page === 'Recovery'}
        <section>
          <h2>Portable collection exchange</h2>
          <p>
            Native export contains versioned JSON and managed asset files. CSV
            is a readable inventory view; blank measurements mean unknown.
          </p>
          <button onclick={() => run(() => exportData('native'))}
            >Download native JSON bundle</button
          ><button onclick={() => run(() => exportData('csv'))}
            >Download inventory CSV</button
          >
          <h3>Import native bundle into an empty collection</h3>
          <p>
            Existing collections are never merged or overwritten by import. An
            identical re-import changes nothing. Use snapshot restore when you
            intend to replace a collection.
          </p>
          <label class="check"
            ><input type="checkbox" bind:checked={importConfirmed} /> Initialize this
            empty collection from the selected native bundle</label
          ><label
            >Native bundle ZIP<input
              type="file"
              accept=".zip,application/zip"
              disabled={!importConfirmed}
              onchange={(e) =>
                run(() => importData(e.currentTarget.files?.[0]))}
            /></label
          >
        </section>
        <section>
          <h2>Scheduled backups</h2>
          <p>
            Daily protection uses Restic after creating a full ZIP. Monthly ZIPs
            run separately. Install the OS timer to run while this window is
            closed.
          </p>
          <button onclick={() => run(loadBackups)}
            >Load backup settings and status</button
          >{#if backupConfig}<form
              onsubmit={(e) => {
                e.preventDefault();
                void run(saveBackups);
              }}
            >
              <div class="form-grid">
                <label
                  >Daily hour (local time)<input
                    type="number"
                    min="0"
                    max="23"
                    bind:value={backupConfig.hour}
                  /></label
                ><label
                  >Minute<input
                    type="number"
                    min="0"
                    max="59"
                    bind:value={backupConfig.minute}
                  /></label
                ><label
                  >Restic executable<input
                    bind:value={backupConfig.restic}
                  /></label
                >
              </div>
              {#each backupConfig.destinations as d}<fieldset>
                  <legend>Backup destination</legend><label
                    >Destination type<select bind:value={d.kind}
                      ><option value="Local">Local folder</option><option
                        value="ExternalDrive">External drive</option
                      ><option value="NetworkShare">Network share</option
                      ></select
                    ></label
                  ><label class="check"
                    ><input type="checkbox" bind:checked={d.enabled} /> Enable this
                    destination</label
                  ><label
                    >Daily repository folder<input
                      bind:value={d.daily}
                    /></label
                  ><label
                    >Monthly ZIP folder<input bind:value={d.monthly} /></label
                  ><label
                    >Required mount point (required for drives/shares)<input
                      value={d.required_mount ?? ''}
                      oninput={(e) =>
                        (d.required_mount = e.currentTarget.value || null)}
                    /></label
                  >
                </fieldset>{/each}<button
                type="button"
                onclick={() =>
                  (backupConfig!.destinations = [
                    ...backupConfig!.destinations,
                    {
                      id: uuid(),
                      kind: 'Local',
                      daily: '',
                      monthly: '',
                      required_mount: null,
                      enabled: false,
                    },
                  ])}>Add backup destination</button
              >
              <button class="primary">Save backup settings</button><button
                type="button"
                disabled={backupBusy}
                onclick={startBackups}
                >{backupBusy ? 'Backing up…' : 'Run backups now'}</button
              >
            </form>
            {#each backupStatus as s}<article class="audit">
                <h3>
                  {backupConfig?.destinations.find(
                    (d: Data) => d.id === s.destination,
                  )?.daily ?? 'Destination'}
                </h3>
                <p>Last daily success: {s.last_daily_success ?? 'Never'}</p>
                <p>Last monthly success: {s.last_monthly_success ?? 'Never'}</p>
                {#if s.daily_error}<p role="alert">
                    Daily: {s.daily_error}
                  </p>{/if}{#if s.monthly_error}<p role="alert">
                    Monthly: {s.monthly_error}
                  </p>{/if}
              </article>{/each}{/if}
        </section>
        <section>
          <h2>Full collection snapshot</h2>
          <p>
            Export a validated ZIP containing the database and referenced
            images/PDFs. A ZIP alone does not complete daily incremental
            protection.
          </p>
          <button class="primary" onclick={() => run(snapshot)}
            >Create and download full snapshot</button
          >
        </section>
        <section>
          <h2>Restore a snapshot</h2>
          <p>
            A validated safety snapshot is created before replacing the
            collection. Restored machine-specific configuration remains inactive
            for review.
          </p>
          <label class="check"
            ><input type="checkbox" bind:checked={restoreConfirmed} /> Replace the
            current collection with the selected snapshot</label
          ><label
            >Snapshot ZIP<input
              type="file"
              accept=".zip,application/zip"
              disabled={!restoreConfirmed || busy}
              onchange={(e) => run(() => restore(e.currentTarget.files?.[0]))}
            /></label
          >
        </section>
      {/if}
      {#if history}<section>
          <div class="section-heading">
            <h2>Audit history</h2>
            <button onclick={() => (history = null)}>Close history</button>
          </div>
          {#each history as entry}<article class="audit">
              <strong>{entry.at} · {entry.actor}</strong>
              <p>{entry.reason || 'Descriptive change'}</p>
              <details>
                <summary>Previous and new recorded values</summary>
                <div class="table-wrap">
                  <table>
                    <thead
                      ><tr><th>Field</th><th>Previous</th><th>New</th></tr
                      ></thead
                    ><tbody
                      >{#each auditChanges(entry) as change}<tr
                          ><td>{change.field}</td><td>{change.before}</td><td
                            >{change.after}</td
                          ></tr
                        >{/each}</tbody
                    >
                  </table>
                </div>
              </details>
            </article>{/each}
        </section>{/if}
    </fieldset>
  </main>
</div>
