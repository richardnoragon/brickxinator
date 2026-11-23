<script lang="ts">
  import { onMount } from 'svelte';
  import {
    api,
    envelope,
    uuid,
    integer,
    scaled,
    money,
    type Data,
  } from './api';
  let { data }: { data: Data } = $props();
  let view = $state<Data | null>(null);
  let error = $state('');
  let busy = $state(false);
  let reason = $state('');
  let projectId = $state('');
  let name = $state('');
  let kind = $state('Moc');
  let versionId = $state('');
  let versionLabel = $state('');
  let designId = $state('');
  let author = $state('');
  let source = $state('');
  let notes = $state('');
  let rows = $state<Data[]>([]);
  let imported = $state<Data[]>([]);
  let snapshotId = $state('');
  let quoteSource = $state('');
  let quoteDate = $state('');
  let quoteCondition = $state('');
  let quoteValue = $state('');
  let part = $state('');
  let color = $state('');
  let quantity = $state('');
  let estimate = $state('');
  let requirement = $state('');
  let holding = $state('');
  let claimQuantity = $state('');
  let missingQuantity = $state('');
  let transferClaim = $state('');
  let transferProject = $state('');
  let transferRequirement = $state('');
  let transferQuantity = $state('');
  let claimKind = $state('Reserved');
  let unitLabel = $state('');
  let unitKind = $state('Bag');
  const parts = $derived(data.catalog.filter((p: Data) => p.kind === 'Part'));
  const colors = $derived(
    data.settings.filter(
      (s: Data) => s.kind.Vocabulary?.vocabulary === 'Color',
    ),
  );
  const project = $derived(view?.state.projects[projectId]);
  const version = $derived(
    project ? view?.state.versions[project.version_id] : null,
  );
  const label = (id: string) =>
    parts.find((p: Data) => p.id === id)?.details.name ??
    parts.find((p: Data) => p.id === id)?.details.number ??
    id;
  const colorName = (id: string) =>
    colors.find((c: Data) => c.id === id)?.name ?? id;
  async function importModel(event: Event) {
    const file = (event.target as HTMLInputElement).files?.[0];
    if (!file) return;
    error = '';
    busy = true;
    try {
      if (file.size > 4 * 1024 * 1024)
        throw new Error('Model text exceeds 4 MiB.');
      const text = await file.text();
      const result = await api('/projects/model-preview', 'POST', {
        source: text,
      });
      imported = result.parts.map((r: Data) => ({
        ...r,
        definition_id:
          parts.find(
            (p: Data) =>
              p.details.number?.toLowerCase() ===
              r.reference.replace(/\.dat$/i, ''),
          )?.id ?? '',
        color_id: '',
      }));
      const hash = await crypto.subtle.digest(
        'SHA-256',
        new TextEncoder().encode(text),
      );
      source =
        file.name +
        ' · SHA256 ' +
        Array.from(new Uint8Array(hash))
          .map((n) => n.toString(16).padStart(2, '0'))
          .join('');
    } catch (e) {
      error = (e as Error).message;
    } finally {
      busy = false;
    }
  }
  function applyMappings() {
    try {
      if (imported.some((r) => !r.definition_id || !r.color_id))
        throw new Error(
          'Map every imported part and color before applying the preview.',
        );
      const grouped = new Map<string, Data>();
      for (const r of imported) {
        const key = r.definition_id + ':' + r.color_id;
        const old = grouped.get(key);
        if (old) old.quantity += r.quantity;
        else
          grouped.set(key, {
            id: uuid(),
            definition_id: r.definition_id,
            color_id: r.color_id,
            quantity: r.quantity,
            unit_estimate_cents: null,
          });
      }
      rows = Array.from(grouped.values());
      imported = [];
      error = '';
    } catch (e) {
      error = (e as Error).message;
    }
  }
  function exportPlan(format: string) {
    if (!view || !project || !version) return;
    const coverage = view.coverage[projectId];
    let text = '';
    if (format === 'csv') {
      const cell = (value: unknown) => {
        let t = String(value ?? '');
        if (/^[\s]*[=+\-@]/.test(t)) t = "'" + t;
        return '"' + t.replace(/"/g, '""') + '"';
      };
      const records = [
        [
          'Part',
          'Color',
          'Required',
          'Reserved',
          'Allocated',
          'Verified',
          'Confirmed missing',
          'Potential gap',
          'Unverified assigned',
          'Estimated gap cost cents',
        ],
        ...coverage.map((c: Data) => {
          const r = version.requirements.find(
            (r: Data) => r.id === c.requirement_id,
          );
          return [
            label(r.definition_id),
            colorName(r.color_id),
            c.required,
            c.reserved,
            c.allocated,
            c.verified,
            c.confirmed_missing,
            c.potential_gap,
            c.unknown_verification,
            c.estimate_cents ?? '',
          ];
        }),
      ];
      text = records.map((row) => row.map(cell).join(',')).join('\r\n');
    } else {
      text =
        '# ' +
        project.name +
        '\n\nBOM version: ' +
        version.label +
        '\n\nProject UUID: ' +
        project.id +
        '\n\nReadiness: ' +
        view.readiness[projectId] +
        '\n\nPlanning gaps are not confirmed missing parts.\n\n';
      for (const c of coverage) {
        const r = version.requirements.find(
          (r: Data) => r.id === c.requirement_id,
        );
        text +=
          '- ' +
          label(r.definition_id) +
          ' / ' +
          colorName(r.color_id) +
          ': required ' +
          c.required +
          ', reserved ' +
          c.reserved +
          ', allocated ' +
          c.allocated +
          ', verified ' +
          c.verified +
          ', confirmed missing ' +
          c.confirmed_missing +
          ', potential gap ' +
          c.potential_gap +
          '\n';
      }
      text += '\n' + project.notes + '\n';
    }
    const url = URL.createObjectURL(
      new Blob([text], {
        type:
          format === 'csv'
            ? 'text/csv;charset=utf-8'
            : 'text/markdown;charset=utf-8',
      }),
    );
    const a = document.createElement('a');
    a.href = url;
    a.download = project.id + (format === 'csv' ? '.csv' : '.md');
    a.click();
    setTimeout(() => URL.revokeObjectURL(url), 1000);
  }
  async function reload() {
    view = await api('/projects');
  }
  onMount(() => {
    reload().catch((e) => (error = e.message));
  });
  async function act(action: Data) {
    busy = true;
    error = '';
    try {
      if (!reason.trim())
        throw new Error('Enter an audit reason before applying a change.');
      await api('/projects', 'POST', {
        ...envelope(),
        reason,
        expected_revision: view?.state.revision ?? 0,
        action,
      });
      await reload();
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    } finally {
      busy = false;
    }
  }
  function addRequirement() {
    try {
      const count = integer(quantity);
      if (!part || !color || !count)
        throw new Error('Choose a part, color and positive quantity.');
      rows = [
        ...rows,
        {
          id: uuid(),
          definition_id: part,
          color_id: color,
          quantity: count,
          unit_estimate_cents: snapshotId
            ? view?.state.price_snapshots[snapshotId].value_cents
            : scaled(estimate, 2),
          price_snapshot_id: snapshotId || null,
        },
      ];
      error = '';
    } catch (e) {
      error = (e as Error).message;
    }
  }
  async function saveVersion() {
    const id = uuid();
    await act({
      AddVersion: {
        id,
        design_id: designId || uuid(),
        label: versionLabel,
        author,
        source,
        notes,
        requirements: rows,
      },
    });
    if (view?.state.versions[id]) {
      versionId = id;
      rows = [];
    }
  }
  async function createProject() {
    const id = uuid();
    await act({
      Create: {
        id,
        name,
        kind,
        version_id: versionId,
        copy_id: null,
        progress: 'Planned',
        notes,
      },
    });
    if (view?.state.projects[id]) projectId = id;
  }
  const claims = $derived(
    Object.values(view?.state.claims ?? {}).filter(
      (c: any) => c.project_id === projectId,
    ) as Data[],
  );
  const units = $derived(
    Object.values(view?.state.units ?? {}).filter(
      (u: any) => u.project_id === projectId,
    ) as Data[],
  );
  const holdingChoices = $derived(
    Object.entries(view?.stock ?? {})
      .filter(([, h]: any) => !h.copy_id && h.definition_id)
      .map(([id, h]) => ({ id, ...(h as Data) }) as Data),
  );
</script>

<p>
  Plan owned or future builds. Reservations and allocations commit existing
  physical stock; requirements never create ownership.
</p>
{#if error}<p role="alert" class="error">{error}</p>{/if}
{#if !view}<p>Loading projects…</p>{:else}
  <fieldset class="workflows" disabled={busy}>
    <label
      >Audit reason <input
        bind:value={reason}
        placeholder="Why are you changing this plan?"
      /></label
    >
    <details class="panel">
      <summary>Recent project audit history</summary>
      <ul>
        {#each view.recent_history as event}<li>
            {event.date} · {Object.keys(event.action)[0].replace(
              /([a-z])([A-Z])/g,
              '$1 $2',
            )} · {event.actor}
            <p>{event.reason}</p>
          </li>{/each}
      </ul>
    </details>
    <section class="panel">
      <h2>Designs and BOM versions</h2>
      <label
        >Import LDraw/MPD requirements<input
          type="file"
          accept=".ldr,.mpd"
          onchange={importModel}
        /></label
      >
      {#if imported.length}
        <h3>Review imported references</h3>
        <p>Choose local identities explicitly. No stock is created.</p>
        {#each imported as item}<div class="form-grid">
            <span
              >{item.reference} · external color {item.color} · {item.quantity}</span
            >
            <label
              >Mapped part<select bind:value={item.definition_id}
                ><option value="">Choose part</option>{#each parts as p}<option
                    value={p.id}>{p.details.name ?? p.details.number}</option
                  >{/each}</select
              ></label
            >
            <label
              >Mapped color<select bind:value={item.color_id}
                ><option value="">Choose color</option
                >{#each colors as c}<option value={c.id}>{c.name}</option
                  >{/each}</select
              ></label
            >
          </div>{/each}
        <button onclick={applyMappings}
          >Apply mapped requirements to draft</button
        >
      {/if}
      <div class="form-grid">
        <label
          >Existing design (leave blank for new)<select bind:value={designId}
            ><option value="">New design</option
            >{#each Object.values(view.state.versions) as v}<option
                value={(v as Data).design_id}>{(v as Data).label}</option
              >{/each}</select
          ></label
        >
        <label
          >Version label<input
            bind:value={versionLabel}
            placeholder="Titanic restoration v1"
          /></label
        >
        <label>Author<input bind:value={author} /></label>
        <label>Source reference<input bind:value={source} /></label>
      </div>
      <label>Notes<textarea bind:value={notes}></textarea></label>
      <div class="form-grid">
        <label
          >Part<select bind:value={part}
            ><option value="">Choose part</option>{#each parts as p}<option
                value={p.id}
                >{p.details.name ?? p.details.number ?? 'Unnamed part'}</option
              >{/each}</select
          ></label
        >
        <label
          >Color<select bind:value={color}
            ><option value="">Choose color</option>{#each colors as c}<option
                value={c.id}>{c.name}</option
              >{/each}</select
          ></label
        >
        <label
          >Required pieces<input
            bind:value={quantity}
            inputmode="numeric"
          /></label
        >
        <label
          >Estimated unit price EUR (optional)<input
            bind:value={estimate}
            inputmode="decimal"
          /></label
        >
      </div>
      <label
        >Use a stored EUR observation for this estimate<select
          bind:value={snapshotId}
          ><option value="">Manual estimate / unknown</option
          >{#each Object.values(view.state.price_snapshots).filter((q: any) => q.definition_id === part && q.currency === 'EUR') as quote}<option
              value={(quote as Data).id}
              >{(quote as Data).source} · {(quote as Data).observed_on} · {(
                quote as Data
              ).condition} · {money((quote as Data).value_cents)}</option
            >{/each}</select
        ></label
      >
      <button onclick={addRequirement}>Add requirement</button>
      <details>
        <summary>Record a market-price observation</summary>
        <p>
          Uses the part selected above. Observations are estimates, not
          acquisition prices or a single true value.
        </p>
        <label>Price source<input bind:value={quoteSource} /></label><label
          >Observation date<input type="date" bind:value={quoteDate} /></label
        ><label>Observed condition<input bind:value={quoteCondition} /></label
        ><label
          >Observed unit price · EUR<input
            bind:value={quoteValue}
            inputmode="decimal"
          /></label
        >
        <button
          onclick={async () => {
            try {
              await act({
                AddPriceSnapshot: {
                  id: uuid(),
                  definition_id: part,
                  source: quoteSource,
                  observed_on: quoteDate,
                  condition: quoteCondition,
                  currency: 'EUR',
                  value_cents: scaled(quoteValue, 2),
                },
              });
            } catch (e) {
              error = (e as Error).message;
            }
          }}>Save price observation</button
        >
        <ul>
          {#each Object.values(view.state.price_snapshots) as quote}<li>
              {label((quote as Data).definition_id)} · {(quote as Data).source} ·
              {(quote as Data).observed_on} · {(quote as Data).condition} · {money(
                (quote as Data).value_cents,
              )}
            </li>{/each}
        </ul>
      </details>
      <ul>
        {#each rows as row, i}<li>
            {label(row.definition_id)} · {colorName(row.color_id)} · {row.quantity}
            <button onclick={() => (rows = rows.filter((_, n) => n !== i))}
              >Remove</button
            >
          </li>{/each}
      </ul>
      <button onclick={saveVersion}>Save BOM version</button>
    </section>
    <section class="panel">
      <h2>Create a project</h2>
      <div class="form-grid">
        <label>Name<input bind:value={name} /></label>
        <label
          >Type<select bind:value={kind}
            ><option value="OfficialSetBuild">Official set build</option><option
              value="Moc">MOC</option
            ><option value="TrainLayout">Train layout</option><option
              value="Diorama">Diorama</option
            ><option value="Custom">Custom project</option></select
          ></label
        >
        <label
          >BOM version<select bind:value={versionId}
            ><option value="">Choose version</option
            >{#each Object.values(view.state.versions) as v}<option
                value={(v as Data).id}>{(v as Data).label}</option
              >{/each}</select
          ></label
        >
      </div>
      <button onclick={createProject}>Create planned project</button>
    </section>
    <section class="panel">
      <h2>All-project procurement coverage</h2>
      <p>
        Confirmed missing, potential gaps and unverified assigned quantities are
        separate evidence categories. They must not be added together as a
        single buying total.
      </p>
      <div class="table-wrap">
        <table>
          <thead
            ><tr
              ><th>Part</th><th>Color</th><th>Confirmed missing</th><th
                >Potential gap</th
              ><th>Assigned but unverified</th><th>Projects</th></tr
            ></thead
          ><tbody>
            {#each view.procurement as need}<tr
                ><td>{label(need.definition_id)}</td><td
                  >{colorName(need.color_id)}</td
                ><td>{need.confirmed_missing}</td><td>{need.potential_gap}</td
                ><td>{need.unknown_verification}</td><td
                  >{need.projects
                    .map((id: string) => view?.state.projects[id].name)
                    .join(', ')}</td
                ></tr
              >{/each}
          </tbody>
        </table>
      </div>
      <h2>Project workspace</h2>
      <label
        >Project<select bind:value={projectId}
          ><option value="">Choose project</option
          >{#each Object.values(view.state.projects) as p}<option
              value={(p as Data).id}>{(p as Data).name}</option
            >{/each}</select
        ></label
      >
      {#if project && version}
        <p>
          {version.label} · Progress: {project.progress} · Readiness: {view
            .readiness[projectId]}
        </p>
        <label
          >Switch BOM version<select bind:value={versionId}
            ><option value="">Choose version</option
            >{#each Object.values(view.state.versions) as v}<option
                value={(v as Data).id}>{(v as Data).label}</option
              >{/each}</select
          ></label
        >
        <button
          disabled={claims.length > 0 ||
            !versionId ||
            versionId === project.version_id}
          onclick={() =>
            act({
              ChangeVersion: { project_id: projectId, version_id: versionId },
            })}>Apply selected version after releasing claims</button
        >
        <p>
          Coverage gaps are planning needs, not confirmed physically missing
          parts. Verification is explicit.
        </p>
        <div class="table-wrap">
          <table>
            <thead
              ><tr
                ><th>Requirement</th><th>Required</th><th>Reserved</th><th
                  >Allocated</th
                ><th>Verified</th><th>Confirmed missing</th><th
                  >Potential gap</th
                ><th>Gap estimate</th></tr
              ></thead
            ><tbody>
              {#each view.coverage[projectId] as row}<tr
                  ><td
                    >{label(
                      version.requirements.find(
                        (r: Data) => r.id === row.requirement_id,
                      ).definition_id,
                    )}</td
                  ><td>{row.required}</td><td>{row.reserved}</td><td
                    >{row.allocated}</td
                  ><td>{row.verified}</td><td>{row.confirmed_missing}</td><td
                    >{row.potential_gap}</td
                  ><td>{money(row.estimate_cents)}</td></tr
                >{/each}
            </tbody>
          </table>
        </div>
        <button onclick={() => exportPlan('csv')}
          >Download procurement CSV</button
        >
        <button onclick={() => exportPlan('md')}>Export project Markdown</button
        >
        <h3>Physical shortage evidence</h3>
        <label
          >Checked requirement<select bind:value={requirement}
            ><option value="">Choose requirement</option
            >{#each version.requirements as r}<option value={r.id}
                >{label(r.definition_id)} · {colorName(r.color_id)}</option
              >{/each}</select
          ></label
        >
        <label
          >Confirmed missing pieces (zero clears)<input
            bind:value={missingQuantity}
            inputmode="numeric"
          /></label
        >
        <button
          onclick={async () => {
            try {
              await act({
                ConfirmMissing: {
                  project_id: projectId,
                  requirement_id: requirement,
                  quantity: integer(missingQuantity),
                },
              });
            } catch (e) {
              error = (e as Error).message;
            }
          }}>Record physically confirmed shortage</button
        >
        <p>
          Changed stock claims reopen shortage evidence for review. Unit
          progress alone does not verify parts.
        </p>
        <h3>Reserve or allocate stock</h3>
        <div class="form-grid">
          <label
            >Requirement<select bind:value={requirement}
              ><option value="">Choose requirement</option
              >{#each version.requirements as r}<option value={r.id}
                  >{label(r.definition_id)} · {colorName(r.color_id)}</option
                >{/each}</select
            ></label
          >
          <label
            >Loose holding<select bind:value={holding}
              ><option value="">Choose holding</option
              >{#each holdingChoices as h}<option value={h.id}
                  >{label(h.definition_id)} · {colorName(h.color_id)} · {view
                    .available[h.id] ?? 'Unknown'} available · {h.id.slice(
                    -8,
                  )}</option
                >{/each}</select
            ></label
          >
          <label
            >Pieces<input
              bind:value={claimQuantity}
              inputmode="numeric"
            /></label
          >
          <label
            >Claim<select bind:value={claimKind}
              ><option value="Reserved">Reserve for future build</option><option
                value="Allocated">Allocate to project</option
              ></select
            ></label
          >
        </div>
        <button
          onclick={async () => {
            try {
              await act({
                Claim: {
                  id: uuid(),
                  project_id: projectId,
                  requirement_id: requirement,
                  holding_id: holding,
                  quantity: integer(claimQuantity),
                  kind: claimKind,
                  verified_revision: null,
                },
              });
            } catch (e) {
              error = (e as Error).message;
            }
          }}>Commit stock claim</button
        >
        <ul>
          {#each claims as c}<li>
              {c.kind} · {c.quantity} · holding {c.holding_id.slice(-8)} · {c.verified_revision ===
              view.stock[c.holding_id]?.revision
                ? 'Physically verified'
                : 'Unverified'}
              <button
                onclick={() =>
                  act({
                    Verify: {
                      claim_id: c.id,
                      holding_revision: view?.stock[c.holding_id].revision,
                    },
                  })}>Confirm physically checked</button
              >
              <button onclick={() => act({ Release: { claim_id: c.id } })}
                >Release claim</button
              >
            </li>{/each}
        </ul>
        <button onclick={() => act({ Start: { project_id: projectId } })}
          >Start build — convert reservations</button
        >
        <button
          onclick={() =>
            act({ Progress: { project_id: projectId, progress: 'Completed' } })}
          >Mark construction completed</button
        >
        <button
          onclick={() =>
            act({
              Progress: { project_id: projectId, progress: 'Dismantled' },
            })}>Mark dismantled — retain allocations</button
        >
        <h3>Transfer a stock claim between projects</h3>
        <label
          >Claim to transfer<select bind:value={transferClaim}
            ><option value="">Choose claim</option>{#each claims as c}<option
                value={c.id}
                >{c.kind} · {c.quantity} pieces · {c.holding_id.slice(
                  -8,
                )}</option
              >{/each}</select
          ></label
        >
        <label
          >Destination project<select bind:value={transferProject}
            ><option value="">Choose project</option
            >{#each Object.values(view.state.projects).filter((p: any) => p.id !== projectId) as p}<option
                value={(p as Data).id}>{(p as Data).name}</option
              >{/each}</select
          ></label
        >
        <label
          >Destination requirement<select bind:value={transferRequirement}
            ><option value="">Choose requirement</option
            >{#each transferProject ? view.state.versions[view.state.projects[transferProject].version_id].requirements : [] as r}<option
                value={r.id}
                >{label(r.definition_id)} · {colorName(r.color_id)}</option
              >{/each}</select
          ></label
        >
        <label
          >Transfer pieces (blank means entire claim)<input
            bind:value={transferQuantity}
            inputmode="numeric"
          /></label
        >
        <button
          onclick={async () => {
            try {
              await act({
                Transfer: {
                  claim_id: transferClaim,
                  project_id: transferProject,
                  requirement_id: transferRequirement,
                  quantity: integer(transferQuantity),
                  new_claim_id: uuid(),
                },
              });
            } catch (e) {
              error = (e as Error).message;
            }
          }}>Transfer reviewed claim quantity</button
        >
        <h3>Bags and subassemblies</h3>
        <label>Label<input bind:value={unitLabel} /></label>
        <label
          >Type<select bind:value={unitKind}
            ><option value="Bag">Bag</option><option value="Subassembly"
              >Subassembly</option
            ></select
          ></label
        >
        <button
          onclick={() =>
            act({
              Unit: {
                id: uuid(),
                project_id: projectId,
                label: unitLabel,
                kind: unitKind,
                progress: unitKind === 'Bag' ? 'Unopened' : 'NotStarted',
              },
            })}>Add build unit</button
        >
        <ul>
          {#each units as u}<li>
              {u.label} · {u.kind} · {u.progress}
              {#each u.kind === 'Bag' ? ['Unopened', 'Opened', 'Completed'] : ['NotStarted', 'InProgress', 'Completed'] as progress}<button
                  onclick={() => act({ Unit: { ...u, progress } })}
                  >{progress}</button
                >{/each}
            </li>{/each}
        </ul>
      {/if}
    </section>
  </fieldset>
{/if}
