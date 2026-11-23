<script lang="ts">
  import { onMount } from 'svelte';
  import { api, envelope, uuid, integer, money, known, type Data } from './api';
  const values = (v: unknown): Data[] => Object.values(v ?? {}) as Data[];
  let { data }: { data: Data } = $props();
  let format = $state('RebrickableParts'),
    source = $state<Data | null>(null),
    parsed = $state<Data | null>(null),
    error = $state(''),
    busy = $state(false),
    reason = $state(''),
    cache = $state<Data[]>([]),
    catalogRows = $state<Data[]>([]),
    parts = $state<Data[]>([]),
    proposal = $state<Data | null>(null),
    command = $state<Data | null>(null),
    name = $state(''),
    versionLabel = $state('Imported BOM'),
    design = $state(''),
    projects = $state<Data | null>(null),
    enrichment = $state<Data>({
      records: {},
      settings_sequence: 0,
      history: [],
    }),
    enrichmentRows = $state<Data[]>([]),
    enrichmentProposal = $state<Data | null>(null),
    enrichmentCommand = $state<Data | null>(null),
    acceptWarnings = $state(false),
    imageTarget = $state(''),
    managedImages = $state<Data[]>([]);
  let partColumn = $state(''),
    colorColumn = $state(''),
    qtyColumn = $state(''),
    inventoryColumn = $state(''),
    inventoryId = $state(''),
    spareColumn = $state(''),
    includeSpares = $state(false),
    delimiter = $state(','),
    manufacturer = $state('');
  let currentCatalog = $state<Data[]>([]);
  let currentSettings = $state<Data[]>([]);
  const defs = (kind: string) =>
    (currentCatalog.length ? currentCatalog : data.catalog).filter(
      (d: Data) => d.kind === kind && !d.archived,
    );
  const vocabulary = (kind: string) =>
    (currentSettings.length ? currentSettings : data.settings).filter(
      (r: Data) => r.kind.Vocabulary?.vocabulary === kind && !r.archived,
    );
  const download = (filename: string, text: string, type = 'text/plain') => {
    const url = URL.createObjectURL(new Blob([text], { type }));
    const a = document.createElement('a');
    a.href = url;
    a.download = filename;
    a.click();
    setTimeout(() => URL.revokeObjectURL(url), 1000);
  };
  async function reload() {
    cache = await api('/integrations');
    const state = await api('/state');
    currentCatalog = state.catalog;
    currentSettings = state.settings;
    managedImages = state.attachments;
    enrichment = await api('/enrichments');
    projects = await api('/projects');
  }
  onMount(() => {
    reload().catch((e) => (error = e.message));
  });
  async function read(event: Event) {
    const f = (event.target as HTMLInputElement).files?.[0];
    if (!f) return;
    busy = true;
    error = '';
    proposal = null;
    enrichmentProposal = null;
    enrichmentRows = [];
    try {
      if (f.size > 4 * 1024 * 1024)
        throw new Error(
          'Use a scoped file up to 4 MiB. Decompress Rebrickable downloads first.',
        );
      source = {
        name: f.name,
        format,
        text: await f.text(),
        delimiter: delimiter === ',' ? ',' : '\t',
        columns: null,
      };
      await parse();
    } catch (e) {
      error = (e as Error).message;
    } finally {
      busy = false;
    }
  }
  async function parse() {
    if (!source) return;
    error = '';
    proposal = null;
    try {
      source = {
        ...source,
        format,
        delimiter: delimiter === ',' ? ',' : '\t',
        columns:
          partColumn && colorColumn && qtyColumn
            ? {
                part: partColumn,
                color: colorColumn,
                quantity: qtyColumn,
                inventory: inventoryColumn || null,
                inventory_id: inventoryId || null,
                spare: spareColumn || null,
                include_spares: includeSpares,
              }
            : null,
      };
      parsed = await api('/integrations/parse', 'POST', source);
      enrichmentRows = (parsed?.enrichment ?? []).map((row: Data) => ({
        ...row,
        selected: false,
        target: '',
        local_name: '',
        fields:
          format === 'RebrickableColors'
            ? ['name', 'external_id', 'rgb', 'is_trans']
            : ['name', 'external_id', 'parent'],
      }));
      catalogRows = (parsed?.catalog ?? []).map((r: Data) => ({
        ...r,
        selected: false,
        target: '',
        theme_id: '',
        fields: [
          'number',
          'name',
          ...(r.release_year === null ? [] : ['release_year']),
          ...(r.stated_piece_count === null ? [] : ['stated_piece_count']),
        ],
      }));
      parts = (parsed?.parts ?? []).map((r: Data) => ({
        ...r,
        definition_id: '',
        color_id: '',
      }));
    } catch (e) {
      error = (e as Error).message;
    }
  }
  function target(row: Data) {
    if (!row.target) return;
    row.fields = [];
  }
  function enrichmentTarget(row: Data) {
    row.fields = [];
    row.local_name = '';
  }
  async function reviewEnrichment() {
    if (!source) return;
    try {
      if (!reason.trim()) throw new Error('Enter an audit reason.');
      enrichmentCommand = {
        ...envelope(),
        reason,
        source,
        expected_settings_sequence: enrichment.settings_sequence,
        accept_name_warnings: acceptWarnings,
        mappings: enrichmentRows
          .filter((r) => r.selected)
          .map((r) => {
            const old = enrichment.records[r.target];
            const setting = currentSettings.find((s) => s.id === r.target);
            return {
              row: r.row,
              target_id: r.target || uuid(),
              expected_revision: old?.revision ?? 0,
              expected_settings_revision: setting?.revision ?? 0,
              fields: r.fields,
              local_name: r.local_name || null,
            };
          }),
      };
      enrichmentProposal = await api(
        '/enrichments/preview',
        'POST',
        enrichmentCommand,
      );
    } catch (e) {
      error = (e as Error).message;
    }
  }
  async function applyEnrichment() {
    if (!enrichmentCommand) return;
    busy = true;
    try {
      await api('/enrichments', 'POST', enrichmentCommand);
      await reload();
      enrichmentRows = [];
      enrichmentProposal = null;
      source = null;
      parsed = null;
    } catch (e) {
      error = (e as Error).message;
    } finally {
      busy = false;
    }
  }
  async function attachImage(event: Event) {
    try {
      if (!imageTarget) throw new Error('Choose a catalog definition first.');
      const files = (event.target as HTMLInputElement).files;
      if (!files) return;
      busy = true;
      for (const file of Array.from(files)) {
        const form = new FormData();
        form.set(
          'meta',
          JSON.stringify({
            ...envelope(),
            target: { Definition: imageTarget },
            original_name: file.name,
          }),
        );
        form.set('file', file);
        await api('/attachments', 'POST', form);
      }
      await reload();
    } catch (e) {
      error = (e as Error).message;
    } finally {
      busy = false;
    }
  }
  async function review() {
    if (!source || !parsed) return;
    error = '';
    try {
      if (!reason.trim()) throw new Error('Enter an audit reason.');
      const mappings = catalogRows
        .filter((r) => r.selected)
        .map((r) => {
          const d = currentCatalog.find((d: Data) => d.id === r.target);
          return {
            row: r.row,
            target_id: d?.id ?? uuid(),
            expected_revision: d?.revision ?? 0,
            manufacturer_id: d?.details.manufacturer_id ?? manufacturer,
            fields: r.fields,
            theme_id: r.theme_id || null,
          };
        });
      command = { ...envelope(), reason, source, mappings };
      proposal = await api('/integrations/preview', 'POST', command);
    } catch (e) {
      error = (e as Error).message;
    }
  }
  async function apply() {
    if (!command) return;
    error = '';
    try {
      await api('/integrations', 'POST', command);
      await reload();
      proposal = null;
      parsed = null;
      source = null;
      catalogRows = [];
      parts = [];
    } catch (e) {
      error = (e as Error).message;
    }
  }
  async function saveBom() {
    if (!source || !parsed) return;
    error = '';
    try {
      if (
        !name.trim() ||
        !reason.trim() ||
        parts.some((p) => !p.definition_id || !p.color_id)
      )
        throw new Error(
          'Enter a design name, reason and explicit local mappings for every part/color.',
        );
      const c = { ...envelope(), reason, source, mappings: [] };
      await api('/integrations', 'POST', c);
      const state = await api('/projects');
      const grouped = new Map<string, Data>();
      for (const p of parts) {
        const key = p.definition_id + ':' + p.color_id;
        const existing = grouped.get(key);
        if (existing) {
          existing.quantity += p.quantity;
          if (!Number.isSafeInteger(existing.quantity))
            throw new Error('Quantity exceeds exact editor range.');
        } else
          grouped.set(key, {
            id: uuid(),
            definition_id: p.definition_id,
            color_id: p.color_id,
            quantity: p.quantity,
            unit_estimate_cents: null,
            price_snapshot_id: null,
          });
      }
      await api('/projects', 'POST', {
        ...envelope(),
        reason,
        expected_revision: state.state.revision,
        action: {
          AddVersion: {
            id: uuid(),
            design_id: design || uuid(),
            label: versionLabel,
            author: '',
            notes: name,
            source: `cached-import:${c.id} sha256:${parsed.sha256}`,
            requirements: [...grouped.values()],
          },
        },
      });
      await reload();
      parts = [];
    } catch (e) {
      error = (e as Error).message;
    }
  }
  let wanted = $state<Data[]>([]),
    projectId = $state('');
  function wantedProject() {
    const p = projects?.state.projects[projectId];
    const v = p ? projects?.state.versions[p.version_id] : null;
    wanted = (v?.requirements ?? []).map((r: Data) => {
      const coverage = projects?.coverage[projectId]?.find(
        (c: Data) => c.requirement_id === r.id,
      );
      return {
        label:
          data.catalog.find((d: Data) => d.id === r.definition_id)?.details
            .name ?? 'Part',
        color:
          vocabulary('Color').find((c: Data) => c.id === r.color_id)?.name ??
          'Unknown',
        definition_id: r.definition_id,
        local_color_id: r.color_id,
        part_id:
          projects?.state.external_mappings?.[
            r.definition_id + ':' + r.color_id
          ]?.bricklink_part_id ?? '',
        color_id: String(
          projects?.state.external_mappings?.[
            r.definition_id + ':' + r.color_id
          ]?.bricklink_color_id ?? '',
        ),
        quantity: String(
          coverage
            ? coverage.confirmed_missing + coverage.potential_gap
            : r.quantity,
        ),
      };
    });
  }
  async function saveMappings() {
    try {
      if (!reason.trim())
        throw new Error('Enter a reason for confirming mappings.');
      for (const r of wanted) {
        if (!r.part_id || r.color_id === '')
          throw new Error('Confirm each external part and color ID.');
        const state = await api('/projects');
        await api('/projects', 'POST', {
          ...envelope(),
          reason,
          expected_revision: state.state.revision,
          action: {
            ConfirmExternalMapping: {
              definition_id: r.definition_id,
              color_id: r.local_color_id,
              bricklink_part_id: r.part_id,
              bricklink_color_id: integer(r.color_id),
              notes: 'Reviewed in offline Wanted List export',
            },
          },
        });
      }
      await reload();
    } catch (e) {
      error = (e as Error).message;
    }
  }
  async function wantedDownload() {
    try {
      const selected = wanted
        .map((r) => {
          const quantity = integer(r.quantity);
          if (quantity === null)
            throw new Error(
              'Enter a known wanted quantity, or zero to omit that item.',
            );
          return { part_id: r.part_id, color_id: r.color_id, quantity };
        })
        .filter((r) => r.quantity > 0);
      if (!selected.length)
        throw new Error('Choose at least one positive wanted quantity.');
      if (selected.some((r) => !r.part_id || r.color_id === ''))
        throw new Error(
          'Confirm BrickLink part and color IDs for each wanted item.',
        );
      const result = await api(
        '/integrations/wanted-xml',
        'POST',
        selected.map((r) => ({
          part_id: r.part_id,
          color_id: integer(r.color_id),
          quantity: r.quantity,
        })),
      );
      download('brickxinator-wanted.xml', result.xml, 'application/xml');
    } catch (e) {
      error = (e as Error).message;
    }
  }
  const md = (value: unknown) =>
    String(value ?? 'Unknown')
      .replaceAll('\\', '\\\\')
      .replaceAll('[', '\\[')
      .replaceAll(']', '\\]')
      .replaceAll('<', '&lt;')
      .replaceAll('>', '&gt;')
      .replaceAll('\n', ' ');
  async function markdown() {
    const physical = await api('/physical');
    const pages: string[] = [
      '# Brickxinator collection',
      '',
      ...data.set_copies.map(
        (c: Data) =>
          `## ${md(c.label)}\n\nbrickxinator_id: ${c.id}\n\nLifecycle: ${md(c.lifecycle?.status ?? 'Active')} · Assembly: ${md(c.assembly)}`,
      ),
      '# Purchase history',
      '',
    ];
    for (const p of data.purchases)
      pages.push(
        `## ${md(p.draft.order_number ?? p.draft.seller ?? 'Purchase')}\n\nbrickxinator_id: ${p.id}\n\nDate: ${md(p.draft.purchased_on)} · Status: ${p.status}\n\n${p.draft.lines.map((l: Data) => `- ${md(l.description ?? l.set_number)}: ${l.paid_total_cents === null ? 'Unknown' : money(l.paid_total_cents)} recorded paid`).join('\n')}\n\n${md(p.draft.notes)}`,
      );
    pages.push(
      '# Physical holdings',
      ...values(physical.holdings).map(
        (h) =>
          `## ${md(h.label)}\n\nbrickxinator_id: ${h.id}\n\nOwned pieces: ${known(h.holding.pieces.owned) || 'Unknown'} · Owned weight (mg): ${known(h.holding.weight_mg.owned) || 'Unknown'}\n\nColor: ${md(h.color)} · Location: ${md(h.location)}`,
      ),
    );
    pages.push('# Build notes and missing-part planning', '');
    for (const p of values(projects?.state.projects) as Data[])
      pages.push(
        `## ${md(p.name)}\n\nbrickxinator_id: ${p.id}\n\nStatus: ${md(p.progress)}\n\n${md(p.notes)}\n\n${(projects?.coverage[p.id] ?? []).map((r: Data) => `- Requirement ${r.requirement_id}: confirmed missing ${r.confirmed_missing ?? 'Unknown'}, potential gap ${r.potential_gap ?? 'Unknown'}`).join('\n')}`,
      );
    download('brickxinator-collection.md', pages.join('\n\n'), 'text/markdown');
  }
</script>

{#if error}<p role="alert" class="error">{error}</p>{/if}
<fieldset class="workflows" disabled={busy}>
  <section>
    <h2>Offline imports and exports</h2>
    <p>
      External files enrich catalog and planning data. Imports never create
      ownership or change paid prices. Studio exports (LDraw/MPD, CSV/TSV) are
      supported; native .io archives are deferred.
    </p>
    <label
      >Import format<select
        bind:value={format}
        onchange={() => {
          source = null;
          parsed = null;
          proposal = null;
          enrichmentRows = [];
          enrichmentProposal = null;
          catalogRows = [];
          parts = [];
        }}
        >{#each ['RebrickableParts', 'RebrickableSets', 'RebrickableColors', 'RebrickableThemes', 'RebrickableInventory', 'StudioParts', 'LDraw'] as f}<option
            >{f}</option
          >{/each}</select
      ></label
    ><label
      >Delimiter<select bind:value={delimiter}
        ><option value=",">Comma CSV</option><option value="tab">Tab TSV</option
        ></select
      ></label
    ><label
      >Local source file<input
        type="file"
        accept=".csv,.tsv,.ldr,.mpd"
        onchange={read}
      /></label
    ><label
      >Import audit reason<input
        bind:value={reason}
        oninput={() => (proposal = null)}
      /></label
    >
  </section>
  {#if parsed?.table}<section>
      <h2>Choose source columns</h2>
      <p>
        Use the provider's part/color IDs and actual quantity column. Bulk
        Rebrickable inventory files require a selected inventory ID; spare parts
        are optional.
      </p>
      {#each [['Part identifier', partColumn], ['Color identifier', colorColumn], ['Quantity', qtyColumn], ['Inventory column', inventoryColumn], ['Spare flag column', spareColumn]] as [label, value], i}<label
          >{label}<select
            {value}
            onchange={(e) => {
              const v = e.currentTarget.value;
              if (i === 0) partColumn = v;
              else if (i === 1) colorColumn = v;
              else if (i === 2) qtyColumn = v;
              else if (i === 3) inventoryColumn = v;
              else spareColumn = v;
              proposal = null;
            }}
            ><option value="">Not selected</option
            >{#each parsed.table.headers as h}<option>{h}</option
              >{/each}</select
          ></label
        >{/each}<label
        >Selected inventory ID<input bind:value={inventoryId} /></label
      ><label
        ><input type="checkbox" bind:checked={includeSpares} />Include spare
        parts</label
      ><button onclick={parse}>Review selected BOM</button>
    </section>{/if}
  {#if enrichmentRows.length}<section
      oninput={() => (enrichmentProposal = null)}
      onchange={() => (enrichmentProposal = null)}
    >
      <h2>Review colors and themes</h2>
      <p>
        Choose each row and the fields to change. Unselected names,
        RGB/transparency and parent relationships stay local. Theme parents must
        be imported or mapped; cycles and duplicate provider identities are
        rejected.
      </p>
      <label
        ><input type="checkbox" bind:checked={acceptWarnings} />Accept reviewed
        duplicate/ambiguous color-name warnings</label
      >{#each enrichmentRows as row}<div class="audit">
          <label
            ><input
              type="checkbox"
              bind:checked={row.selected}
            />{row.external_id} · {row.name}</label
          >{#if row.selected}<label
              >Local enrichment target<select
                bind:value={row.target}
                onchange={() => enrichmentTarget(row)}
                ><option value=""
                  >Create local {format === 'RebrickableColors'
                    ? 'color'
                    : 'theme'}</option
                >{#if format === 'RebrickableColors'}{#each vocabulary('Color').filter((r: Data) => !r.kind.Vocabulary.system_unknown) as color}<option
                      value={color.id}
                      >{color.name} · {color.id.slice(-8)}</option
                    >{/each}{:else}{#each values(enrichment.records).filter((r) => r.kind === 'Theme') as theme}<option
                      value={theme.id}
                      >{theme.name} · source {theme.external_id}</option
                    >{/each}{/if}</select
              ></label
            >{#each format === 'RebrickableColors' ? ['name', 'external_id', 'rgb', 'is_trans'] : ['name', 'external_id', 'parent'] as field}<label
                ><input
                  type="checkbox"
                  value={field}
                  bind:group={row.fields}
                />Apply {field}: {field === 'parent'
                  ? (row.parent_external_id ?? 'Root')
                  : (row[field] ?? 'Unknown')}</label
              >{/each}<label
              >Local display name override<input
                bind:value={row.local_name}
                placeholder="Blank uses source name when selected"
              /></label
            >{/if}
        </div>{/each}<button onclick={reviewEnrichment}
        >Review selected color/theme changes</button
      >
    </section>{/if}
  {#if enrichmentProposal}<section>
      <h2>Review enrichment before applying</h2>
      {#each enrichmentProposal.records as row}<p>
          {row.kind}: {row.name} · source ID {row.external_id} · revision {row.revision}{#if row.rgb}
            · RGB {row.rgb}{/if}
        </p>{/each}
      <p>Stock and paid-price changes: none.</p>
      <button class="primary" onclick={applyEnrichment}
        >Apply reviewed color/theme import</button
      >
    </section>{/if}
  {#if catalogRows.length}<section
      oninput={() => (proposal = null)}
      onchange={() => (proposal = null)}
    >
      <h2>Review catalog enrichment</h2>
      <label
        >Manufacturer for new definitions<select bind:value={manufacturer}
          ><option value="">Choose manufacturer</option
          >{#each vocabulary('Manufacturer') as m}<option value={m.id}
              >{m.name}</option
            >{/each}</select
        ></label
      >
      <p>
        Choose rows explicitly. Existing records retain local fields unless
        selected for replacement; UUIDs stay unchanged.
      </p>
      {#each catalogRows as row}<div class="audit">
          <label
            ><input type="checkbox" bind:checked={row.selected} />{row.number} · {row.name}</label
          >{#if row.selected}<label
              >Catalog target<select
                bind:value={row.target}
                onchange={() => target(row)}
                ><option value="">Create new definition</option
                >{#each defs(format === 'RebrickableParts' ? 'Part' : 'Set') as d}<option
                    value={d.id}>{d.details.number} · {d.details.name}</option
                  >{/each}</select
              ></label
            >{#if format === 'RebrickableSets'}<label
                >Imported catalog theme<select bind:value={row.theme_id}
                  ><option value="">No linked theme / clear link</option
                  >{#each values(enrichment.records).filter((r) => r.kind === 'Theme') as theme}<option
                      value={theme.id}
                      >{theme.name} · source {theme.external_id}</option
                    >{/each}</select
                ></label
              >{/if}{#each format === 'RebrickableSets' ? ['number', 'name', 'release_year', 'stated_piece_count', 'theme'] : ['number', 'name'] as field}<label
                ><input
                  type="checkbox"
                  value={field}
                  bind:group={row.fields}
                />Apply {field}: {row[field] ?? 'Unknown'}</label
              >{/each}{/if}
        </div>{/each}<button onclick={review}
        >Review selected catalog changes</button
      >
    </section>{/if}
  {#if parts.length}<section>
      <h2>Review local BOM mappings</h2>
      {#each parts as p}<div class="audit">
          <strong
            >{p.reference} · source color {p.color} · {p.quantity} pieces</strong
          ><label
            >Local part<select bind:value={p.definition_id}
              ><option value="">Choose part</option
              >{#each defs('Part') as d}<option value={d.id}
                  >{d.details.number} · {d.details.name}</option
                >{/each}</select
            ></label
          ><label
            >Local color<select bind:value={p.color_id}
              ><option value="">Choose color</option
              >{#each vocabulary('Color') as c}<option value={c.id}
                  >{c.name}</option
                >{/each}</select
            ></label
          >
        </div>{/each}<label
        >Design name or notes<input bind:value={name} /></label
      ><label>BOM version label<input bind:value={versionLabel} /></label><label
        >Design to version<select bind:value={design}
          ><option value="">New design</option
          >{#each values(projects?.state.versions) as v}<option
              value={v.design_id}>{v.label}</option
            >{/each}</select
        ></label
      ><button onclick={saveBom}
        >Cache source and save reviewed BOM version</button
      >
      <p>
        Create or update the build project in Projects using the saved version.
        Existing allocations and versions remain unchanged.
      </p>
    </section>{/if}
  {#if proposal}<section>
      <h2>Review import before applying</h2>
      <p>
        Source: {source?.name}. Catalog changes: {proposal.changes.length}.
        Stock changes: none.
      </p>
      {#each proposal.changes as c}<p>
          {Object.keys(c.action)[0]}: {c.action.Create?.details.name ??
            c.action.Edit?.details.name ??
            'Catalog definition'}
        </p>{/each}<button class="primary" onclick={apply}
        >Apply reviewed import</button
      >
    </section>{/if}
  <section>
    <h2>Catalog colors and themes</h2>
    {#each values(enrichment.records) as row}<div class="audit">
        <strong
          >{row.kind === 'Color'
            ? (currentSettings.find((s) => s.id === row.id)?.name ?? row.name)
            : row.name}</strong
        >
        · {row.kind} · source {row.external_id}{#if row.rgb}<span
            style:background-color={'#' + row.rgb}
            style:display="inline-block"
            style:width="1rem"
            style:height="1rem"
            aria-label={'RGB ' + row.rgb}
          ></span>
          RGB {row.rgb}{/if}{#if row.is_trans !== null}
          · {row.is_trans ? 'Transparent' : 'Opaque'}{/if}{#if row.parent_id}<p>
            Parent: {enrichment.records[row.parent_id]?.name ?? 'Unknown'}
          </p>{/if}
      </div>{/each}
  </section>
  <section>
    <h2>Catalog images</h2>
    <p>
      Attach selected local files. Images are stored by content hash and remain
      available offline; the app does not download external URLs or replace
      existing images.
    </p>
    <label
      >Image catalog definition<select bind:value={imageTarget}
        ><option value="">Choose definition</option
        >{#each currentCatalog as definition}<option value={definition.id}
            >{definition.details.number} · {definition.details.name}</option
          >{/each}</select
      ></label
    ><label
      >Attach catalog image<input
        type="file"
        accept="image/png,image/jpeg,image/webp"
        multiple
        onchange={attachImage}
      /></label
    >{#each managedImages.filter((a) => a.target.Definition === imageTarget && a.media_type.startsWith('image/')) as image}<a
        href={'/api/v1/attachments/' + image.id}
        target="_blank"
        rel="noreferrer"
        ><img
          src={'/api/v1/attachments/' + image.id}
          alt={image.original_name}
          style:max-width="160px"
        />{image.original_name}</a
      >{/each}
  </section>
  <section>
    <h2>Color/theme import history</h2>
    {#each enrichment.history as entry}<div class="audit">
        <strong>{entry.command.source.name}</strong> · {entry.at}
        <p>{entry.command.reason}</p>
        <button
          onclick={() =>
            download(entry.command.source.name, entry.command.source.text)}
          >Download cached color/theme source</button
        >
      </div>{/each}
  </section>
  <section>
    <h2>Source cache and manual refresh</h2>
    <p>
      Choose a new file to refresh; each reviewed import preserves the previous
      source. Cached files remain available offline.
    </p>
    {#each cache as c}<div class="audit">
        <strong>{c.command.source.name}</strong> · {c.command.source.format} · {c.at}
        <p>{c.command.reason}</p>
        <button
          onclick={() => download(c.command.source.name, c.command.source.text)}
          >Download cached source</button
        >
      </div>{/each}
  </section>
  <section>
    <h2>BrickLink Wanted List</h2>
    <p>
      Download only. Confirm BrickLink identifiers explicitly; local UUIDs and
      Rebrickable/LDraw IDs are not substituted.
    </p>
    <label
      >Build project<select bind:value={projectId} onchange={wantedProject}
        ><option value="">Choose project</option
        >{#each values(projects?.state.projects) as p}<option value={p.id}
            >{p.name}</option
          >{/each}</select
      ></label
    >{#each wanted as r}<div class="audit">
        <strong>{r.label} · {r.color}</strong><label
          >Confirmed BrickLink part ID<input bind:value={r.part_id} /></label
        ><label
          >Confirmed BrickLink color ID<input
            bind:value={r.color_id}
            inputmode="numeric"
          /></label
        ><label
          >Wanted quantity<input
            bind:value={r.quantity}
            inputmode="numeric"
          /></label
        >
      </div>{/each}<button onclick={saveMappings}
      >Save confirmed BrickLink mappings</button
    >
    <button onclick={wantedDownload}>Download reviewed Wanted List XML</button>
  </section>
  <section>
    <h2>Obsidian / Markdown</h2>
    <p>
      Export collection, purchase history, project notes and evidence-aware
      missing-part planning to a local Markdown document.
    </p>
    <button onclick={markdown}>Download collection Markdown</button>
  </section>
</fieldset>
