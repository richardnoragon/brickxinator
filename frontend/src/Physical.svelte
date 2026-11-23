<script lang="ts">
  import { onMount } from 'svelte';
  import {
    api,
    envelope,
    uuid,
    integer,
    scaled,
    known,
    quantityAmount,
    type Data,
  } from './api';
  let view = $state<Data | null>(null),
    error = $state(''),
    busy = $state(false),
    proposal = $state<Data | null>(null);
  let action = $state('Split'),
    source = $state(''),
    chosen = $state<string[]>([]),
    reason = $state('');
  let pieces = $state(''),
    grams = $state(''),
    damage = $state(''),
    damageGrams = $state('');
  let copyId = $state(''),
    status = $state('Sold'),
    date = $state(''),
    proceeds = $state(''),
    recipient = $state(''),
    notes = $state('');
  let dispositions = $state<Data[]>([]);
  const holdings = $derived(
    Object.values(view?.holdings ?? {}).filter(
      (h: any) => !h.links.archived,
    ) as Data[],
  );
  const copy = $derived(view?.copies.find((c: Data) => c.id === copyId));
  function portion(p: string, w: string, d: string, dw: string) {
    return {
      pieces: integer(p),
      weight_mg: scaled(w, 3),
      damaged_pieces: integer(d),
      damaged_weight_mg: scaled(dw, 3),
    };
  }
  function count(m: unknown) {
    return known(m) || 'Unknown';
  }
  function describe(h: Data | null | undefined) {
    return h
      ? `${count(h.pieces.owned)} pieces, ${known(h.weight_mg.owned) ? quantityAmount(known(h.weight_mg.owned), 3) + ' g' : 'unmeasured weight'}`
      : 'New holding';
  }
  async function reload() {
    view = await api('/physical');
  }
  onMount(() => {
    reload().catch((e) => (error = e.message));
  });
  function chooseCopy(event: Event) {
    copyId = (event.target as HTMLSelectElement).value;
    const c = view?.copies.find((c: Data) => c.id === copyId);
    dispositions = (c?.linked ?? []).map((id: string) => ({
      id,
      mode: '',
      pieces: '',
      grams: '',
      damage: '',
      damageGrams: '',
    }));
    proposal = null;
  }
  async function preview() {
    busy = true;
    error = '';
    try {
      if (!reason.trim()) throw new Error('Enter an audit reason.');
      let body: Data;
      if (action === 'Split') {
        const h = view?.holdings[source];
        if (!h) throw new Error('Choose source stock.');
        body = {
          Split: {
            source_id: source,
            child_id: uuid(),
            expected_revision: h.holding.revision,
            portion: portion(pieces, grams, damage, damageGrams),
          },
        };
      } else if (action === 'Detach') {
        const h = view?.holdings[source];
        if (!h) throw new Error('Choose assigned stock.');
        body = {
          Detach: { holding_id: source, expected_revision: h.holding.revision },
        };
      } else if (action === 'Merge') {
        body = {
          Merge: {
            sources: chosen.map((id) => ({
              holding_id: id,
              expected_revision: view?.holdings[id].holding.revision,
            })),
            target_id: uuid(),
          },
        };
      } else {
        if (!copy) throw new Error('Choose a physical copy.');
        const correcting = action === 'CorrectLifecycle';
        if (correcting && copy.lifecycle.status === 'Active')
          throw new Error('This copy is already Active.');
        const parts = correcting
          ? []
          : dispositions.map((d) => {
              if (!d.mode)
                throw new Error(
                  'Choose an explicit disposition for every linked holding.',
                );
              return {
                holding_id: d.id,
                expected_revision: view?.holdings[d.id].holding.revision,
                handling:
                  d.mode === 'Custom'
                    ? {
                        Custom: {
                          child_id: uuid(),
                          keep: portion(
                            d.pieces,
                            d.grams,
                            d.damage,
                            d.damageGrams,
                          ),
                        },
                      }
                    : d.mode,
              };
            });
        body = {
          Lifecycle: {
            copy_id: copyId,
            expected_revision: copy.lifecycle.revision,
            status: correcting ? 'Active' : status,
            effective_on: date,
            proceeds_cents:
              !correcting && status === 'Sold' ? scaled(proceeds, 2) : null,
            recipient: recipient || null,
            notes,
            parts,
            corrects: correcting ? copy.lifecycle.last_event : null,
          },
        };
      }
      proposal = await api('/physical/preview', 'POST', {
        ...envelope(),
        reason,
        body,
      });
    } catch (e) {
      error = (e as Error).message;
    } finally {
      busy = false;
    }
  }
  async function apply() {
    if (!proposal) return;
    busy = true;
    error = '';
    try {
      await api('/physical', 'POST', {
        operation: proposal.operation,
        expected_profile_sequence: proposal.expected_profile_sequence,
      });
      proposal = null;
      await reload();
    } catch (e) {
      error = (e as Error).message;
    } finally {
      busy = false;
    }
  }
</script>

<p>
  Move physical stock without erasing its source history. Copy lifecycle and
  part ownership are separate; every disposition is explicit.
</p>
{#if error}<p role="alert" class="message error">{error}</p>{/if}
{#if view}<fieldset class="workflows" disabled={busy}>
    <section
      oninput={() => (proposal = null)}
      onchange={() => (proposal = null)}
    >
      <h2>Prepare a reviewed movement</h2>
      <label
        >Operation<select bind:value={action}
          ><option value="Split">Split stock into a new loose holding</option
          ><option value="Merge">Merge compatible source holdings</option
          ><option value="Detach">Detach all of a set's holding</option><option
            value="Lifecycle"
            >Sell / donate / lose / void / return a copy</option
          ><option value="CorrectLifecycle"
            >Correct a lifecycle event back to Active</option
          ></select
        ></label
      >
      <label>Reason<input bind:value={reason} /></label>
      {#if action === 'Split' || action === 'Detach'}
        <label
          >Source holding<select bind:value={source}
            ><option value="">Choose stock</option
            >{#each holdings.filter((h) => action !== 'Detach' || h.copy_id) as h}<option
                value={h.id}
                >{h.label} · {h.color} · {count(h.holding.pieces.owned)} pieces ·
                {h.location ?? 'Unknown location'} · {h.id.slice(-8)}</option
              >{/each}</select
          ></label
        >
        {#if action === 'Split'}
          <p>
            The child is loose stock at the source location. Unmeasured
            transferred dimensions remain Unknown; source measurements may
            become Unknown. Resolve active claims before moving claimed stock.
          </p>
          <div class="form-grid">
            <label
              >Transferred pieces<input
                bind:value={pieces}
                inputmode="numeric"
              /></label
            ><label
              >Transferred weight · g<input
                bind:value={grams}
                inputmode="decimal"
              /></label
            ><label
              >Transferred damaged pieces<input
                bind:value={damage}
                inputmode="numeric"
              /></label
            ><label
              >Transferred damaged weight · g<input
                bind:value={damageGrams}
                inputmode="decimal"
              /></label
            >
          </div>
        {:else}<p>
            Ownership stays unchanged. The holding becomes independently movable
            loose stock; historical set verification is retained.
          </p>{/if}
      {:else if action === 'Merge'}
        <p>
          Select matching part, color, condition and location. Source identities
          and acquisition layers remain in history; no average price is
          invented. Release claims and detach set assignments first.
        </p>
        {#each holdings.filter((h) => !h.copy_id) as h}<label class="check"
            ><input type="checkbox" bind:group={chosen} value={h.id} />{h.label} ·
            {h.color} · {count(h.holding.pieces.owned)} pieces · {h.condition} · {h.location ??
              'Unknown location'} · {h.id.slice(-8)}</label
          >{/each}
      {:else}
        <label
          >Physical copy<select value={copyId} onchange={chooseCopy}
            ><option value="">Choose copy</option
            >{#each view.copies as c}<option value={c.id}
                >{c.label} · {c.lifecycle.status}</option
              >{/each}</select
          ></label
        >
        <div class="form-grid">
          <label>Effective date<input type="date" bind:value={date} /></label
          >{#if action === 'Lifecycle'}<label
              >Lifecycle outcome<select bind:value={status}
                >{#each ['Sold', 'Donated', 'Lost', 'Stolen', 'Destroyed', 'Void', 'Returned'] as state}<option
                    value={state}>{state}</option
                  >{/each}</select
              ></label
            >{#if status === 'Sold'}<label
                >Sale proceeds · EUR (optional)<input
                  bind:value={proceeds}
                  inputmode="decimal"
                /></label
              >{/if}<label
              >Recipient (optional)<input bind:value={recipient} /></label
            >{/if}
        </div>
        <label>Notes<textarea bind:value={notes}></textarea></label>
        {#if action === 'Lifecycle'}
          <p>
            No hidden part removal. Choose Transfer all, Keep all, or custom
            retained quantities for every current linked holding. Kept parts
            become independent loose stock. Returns mean confirmed physical
            handover; no refund is created automatically.
          </p>
          {#each dispositions as d}<div class="panel">
              <h3>
                {view.holdings[d.id].label} · {describe(
                  view.holdings[d.id].holding,
                )}
              </h3>
              <label
                >Part handling<select bind:value={d.mode}
                  ><option value="">Choose disposition</option><option
                    value="TransferAll">Transfer all ownership</option
                  ><option value="KeepAll">Keep all as loose stock</option
                  ><option value="Custom"
                    >Keep a custom quantity; transfer the rest</option
                  ></select
                ></label
              >
              {#if d.mode === 'Custom'}<div class="form-grid">
                  <label
                    >Kept pieces<input
                      bind:value={d.pieces}
                      inputmode="numeric"
                    /></label
                  ><label
                    >Kept weight · g<input
                      bind:value={d.grams}
                      inputmode="decimal"
                    /></label
                  ><label
                    >Kept damaged pieces<input
                      bind:value={d.damage}
                      inputmode="numeric"
                    /></label
                  ><label
                    >Kept damaged weight · g<input
                      bind:value={d.damageGrams}
                      inputmode="decimal"
                    /></label
                  >
                </div>{/if}
            </div>{/each}
        {:else}<p>
            A correction restores only the original event's recorded stock
            effects. Later stock, claim or metadata changes can block it for
            review. Original events and copy identities remain.
          </p>{/if}
      {/if}
      <button onclick={preview}>Review physical effects</button>
    </section>
    {#if proposal}<section>
        <h2>Review before applying</h2>
        <p>{proposal.operation.reason}</p>
        {#each proposal.effects as effect}<p>
            {view.holdings[effect.id]?.label ?? 'New loose holding'}: {describe(
              effect.before,
            )} → {describe(effect.after)}
          </p>{/each}{#if !proposal.effects.length}<p>
            Copy state changes only; no actual part stock is linked.
          </p>{/if}<button class="primary" onclick={apply}
          >Apply reviewed physical event</button
        ><button onclick={() => (proposal = null)}>Cancel</button>
      </section>{/if}
    <section>
      <h2>Recent physical history</h2>
      {#each view.history as op}<div class="audit">
          <strong>{op.body.Lifecycle?.status ?? Object.keys(op.body)[0]}</strong
          >
          · {op.actor}
          <p>{op.reason}</p>
          {#if op.body.Lifecycle}<small
              >{op.body.Lifecycle.effective_on} · {view.copies.find(
                (c: Data) => c.id === op.body.Lifecycle.copy_id,
              )?.label}</small
            >{/if}
        </div>{/each}
    </section>
    <section>
      <h2>Source-layer links</h2>
      <p>
        Every merge and partial transfer retains its source identities. Original
        purchases remain accessible in inventory history.
      </p>
      {#each view.flow as flow}<p>
          {view.holdings[flow.source]?.label ?? 'Source stock'} ({flow.source.slice(
            -8,
          )}) → {view.holdings[flow.target]?.label ?? 'Derived stock'} ({flow.target.slice(
            -8,
          )})
        </p>{/each}
    </section>
  </fieldset>{:else}<p>Loading physical stock…</p>{/if}
