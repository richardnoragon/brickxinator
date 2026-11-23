<script lang="ts">
  import { onMount } from 'svelte';
  import {
    api,
    envelope,
    scaled,
    known,
    integer,
    money as formatMoney,
    type Data,
  } from './api';
  const money = (n: unknown) =>
    formatMoney(n as string | number | null | undefined);
  let { data }: { data: Data } = $props();
  let purchase = $state(''),
    view = $state<Data | null>(null),
    physical = $state<Data>({ holdings: {} }),
    error = $state(''),
    busy = $state(false),
    proposal = $state<Data | null>(null),
    command = $state<Data | null>(null);
  let component = $state('Item'),
    line = $state(''),
    method = $state('Manual'),
    amount = $state(''),
    reviewedDisposed = $state(''),
    reason = $state(''),
    date = $state(''),
    rows = $state<Data[]>([]),
    copyRows = $state<Data[]>([]),
    rollups = $state<Data | null>(null);
  const values = (v: unknown): Data[] =>
    Object.entries(v ?? {}).map(([id, row]) => ({ id, ...(row as Data) }));
  async function loadRollups() {
    rollups = await api('/cost-rollups');
  }
  onMount(() => {
    loadRollups().catch((e) => (error = e.message));
  });
  async function load() {
    if (!purchase) return;
    busy = true;
    error = '';
    proposal = null;
    try {
      view = await api('/child-costs/' + purchase);
      physical = await api('/physical');
      await loadRollups();
      copyRows = physical.copies
        .filter(
          (c: Data) =>
            c.lifecycle.status === 'Active' &&
            view!.basis.purchase.draft.lines.some(
              (l: Data) => l.id === c.invoice_line_id,
            ),
        )
        .map((c: Data) => ({
          id: c.id,
          invoice_line_id: c.invoice_line_id,
          label: c.label,
          selected: false,
          share: '',
          pieces: '',
        }));
      rows = (Object.values(physical.holdings) as Data[])
        .filter((h) => !h.links.archived)
        .map((h) => ({
          id: h.id,
          label: h.label,
          selected: false,
          share: '',
          pieces: known(h.holding.pieces.owned),
        }));
    } catch (e) {
      error = (e as Error).message;
    } finally {
      busy = false;
    }
  }
  async function review() {
    try {
      if (!view) return;
      const cents = scaled(amount, 2);
      if (cents === null) throw new Error('Enter the allocated amount.');
      const old = view.plans.find(
        (p: Data) =>
          p.command.component === component &&
          p.command.invoice_line_id === (component === 'Fees' ? null : line),
      );
      command = {
        ...envelope(),
        reason,
        effective_on: date,
        purchase_id: purchase,
        invoice_line_id: component === 'Fees' ? null : line,
        component,
        method,
        expected_revision: old?.plan.revision ?? 0,
        expected_inventory_sequence: view.inventory_sequence,
        expected_purchase_revision: view.basis.purchase.revision,
        expected_fulfilment_revision: view.fulfilment_revision,
        expected_shipping_revision: view.basis.shipping.revision,
        allocated_cents: cents,
        reviewed_disposed_cents: scaled(reviewedDisposed, 2),
        copy_targets: Object.fromEntries(
          copyRows
            .filter((r) => r.selected)
            .map((r) => [
              r.id,
              method === 'Manual' ? scaled(r.share, 2) : null,
            ]),
        ),
        copy_piece_bases:
          method === 'PieceCount'
            ? Object.fromEntries(
                copyRows
                  .filter((r) => r.selected && r.pieces !== '')
                  .map((r) => [r.id, integer(r.pieces)]),
              )
            : {},
        targets: Object.fromEntries(
          rows
            .filter((r) => r.selected)
            .map((r) => [
              r.id,
              method === 'Manual' ? scaled(r.share, 2) : null,
            ]),
        ),
      };
      proposal = await api('/child-costs/preview', 'POST', command);
    } catch (e) {
      error = (e as Error).message;
    }
  }
  async function apply() {
    if (!command) return;
    try {
      await api('/child-costs', 'POST', command);
      await load();
    } catch (e) {
      error = (e as Error).message;
    }
  }
</script>

{#if error}<p class="error" role="alert">{error}</p>{/if}
<fieldset class="workflows" disabled={busy}>
  <section>
    <h2>Acquisition cost allocations</h2>
    <p>
      Allocate derived cost without changing original purchase prices or stock.
      Item, shipping and fees remain separate. Any unallocated balance stays on
      its source.
    </p>
    <label
      >Cost source purchase<select bind:value={purchase} onchange={load}
        ><option value="">Choose purchase</option
        >{#each data.purchases as p}<option value={p.id}
            >{p.draft.order_number ?? p.draft.seller ?? 'Unnamed purchase'} · {p.id.slice(
              -8,
            )}</option
          >{/each}</select
      ></label
    >
  </section>
  {#if view}<section
      oninput={() => (proposal = null)}
      onchange={() => (proposal = null)}
    >
      <h2>Prepare allocation or reallocation</h2>
      <label
        >Cost component<select bind:value={component}
          ><option>Item</option><option>Shipping</option><option>Fees</option
          ></select
        ></label
      >{#if component !== 'Fees'}<label
          >Source invoice line<select bind:value={line}
            ><option value="">Choose source line</option
            >{#each view.basis.purchase.draft.lines as l}<option value={l.id}
                >{l.description ?? l.set_number ?? 'Invoice line'}</option
              >{/each}</select
          ></label
        >{/if}<label
        >Allocation method<select bind:value={method}
          ><option>Manual</option><option>PieceCount</option></select
        ></label
      ><label
        >Amount to allocate · EUR<input
          bind:value={amount}
          inputmode="decimal"
        /></label
      ><label
        >Reviewed disposed cost · EUR (blank preserves current disposal cost)<input
          bind:value={reviewedDisposed}
          inputmode="decimal"
        /></label
      ><label>Cost effective date<input type="date" bind:value={date} /></label
      ><label>Cost audit reason<input bind:value={reason} /></label>
      <p>
        Choose holdings derived from this source. Piece Count requires known
        physical counts; no fallback to weight or equal shares.
      </p>
      {#each rows as row}<div class="audit">
          <label
            ><input type="checkbox" bind:checked={row.selected} />{row.label} · {row.pieces ||
              'Unknown'} pieces · {row.id.slice(-8)}</label
          >{#if row.selected && method === 'Manual'}<label
              >Derived share · EUR<input
                bind:value={row.share}
                inputmode="decimal"
              /></label
            >{/if}
        </div>{/each}
      <h3>Direct physical-copy costs</h3>
      <p>
        Copy and holding shares draw from the same source amount. Enter a
        piece-count basis explicitly for Piece Count; this is a costing basis
        and does not create or verify stock.
      </p>
      {#each copyRows.filter((r) => component === 'Fees' || !line || r.invoice_line_id === line) as row}<div
          class="audit"
        >
          <label
            ><input type="checkbox" bind:checked={row.selected} />{row.label} · copy
            {row.id.slice(-8)}</label
          >{#if row.selected}{#if method === 'Manual'}<label
                >Direct copy share · EUR<input
                  bind:value={row.share}
                  inputmode="decimal"
                /></label
              >{:else}<label
                >Copy piece-count cost basis<input
                  bind:value={row.pieces}
                  inputmode="numeric"
                /></label
              >{/if}{/if}
        </div>{/each}
      <button onclick={review}>Review cost allocation</button>
    </section>
    {#if proposal}<section>
        <h2>Review derived costs</h2>
        <p>
          Source basis: {money(proposal.total_cents)}. Unallocated source
          balance: {money(proposal.residual_cents)}.
        </p>
        <p>Reviewed disposed cost: {money(proposal.disposed_cents)}.</p>
        {#each Object.entries(proposal.shares) as [id, n]}<p>
            {physical.holdings[id]?.label ?? id.slice(-8)}: {money(n)}
          </p>{/each}<button class="primary" onclick={apply}
          >Apply reviewed cost allocation</button
        ><button onclick={() => (proposal = null)}>Cancel</button>
        {#each Object.entries(proposal.copy_shares ?? {}) as [id, n]}<p>
            {physical.copies.find((c: Data) => c.id === id)?.label ??
              id.slice(-8)} · direct: {money(n)}
          </p>{/each}
      </section>{/if}
    <section>
      <h2>Current cost layers</h2>
      {#each view.plans as p}<div class="audit">
          <strong>{p.command.component} · revision {p.plan.revision}</strong
          >{#if p.requires_reallocation}<p class="error">
              Source changed: review a reallocation. Historical allocated costs
              have not been silently repriced.
            </p>{/if}
          <p>
            Unallocated source balance: {money(p.current.residual_cents)}.
            Disposed cost: {money(p.current.disposed_cents)}.
          </p>
          {#each Object.entries(p.current.shares) as [id, n]}<p>
              {physical.holdings[id]?.label ?? id.slice(-8)}: {money(n)}
            </p>{/each}{#each Object.entries(p.current.copy_shares ?? {}) as [id, n]}<p
            >
              {physical.copies?.find((c: Data) => c.id === id)?.label ??
                id.slice(-8)} · direct: {money(n)}
            </p>{/each}{#each p.current.issues as issue}<p class="error">
              {issue}
            </p>{/each}
        </div>{/each}
    </section>
    <section>
      <h2>Cost allocation history</h2>
      {#each [...view.history].reverse() as [c, p]}<div class="audit">
          <strong>{c.component} · {c.method} · revision {p.revision}</strong>
          <p>{c.effective_on} · {c.actor} · {c.reason}</p>
          {#each Object.entries(p.copy_shares ?? {}) as [id, n]}<p>
              {physical.copies?.find((copy: Data) => copy.id === id)?.label ??
                id.slice(-8)} · originally assigned direct cost: {money(n)}
            </p>{/each}
        </div>{/each}
    </section>{/if}
  <section>
    <h2>Cost rollups</h2>
    <button onclick={() => loadRollups().catch((e) => (error = e.message))}
      >Refresh cost rollups</button
    >
    <p>
      Rollups are views of existing allocations, not additional cost. Project
      totals include allocated stock; reservations and market estimates are
      separate. Collection totals count each holding and direct copy share once.
      Unassigned costs are not zero; reviewed totals can be partial.
    </p>
    {#if rollups}<p>
        Reviewed owned cost: {money(rollups.reviewed_owned.known_cents)}{rollups
          .reviewed_owned.incomplete
          ? ' + unknown costs'
          : ''}. Unallocated source balances: {money(
          rollups.unallocated_source.known_cents,
        )}. Disposed cost: {money(rollups.disposed.known_cents)}.
      </p>
      <p>
        {rollups.uncosted_holdings.length} holding(s) have no reviewed cost. {rollups
          .requires_reallocation.length} source allocation(s) require review.
      </p>
      <h3>Physical copies</h3>
      {#each values(rollups.copies) as row}<div
          class="audit"
          data-testid={'copy-cost-rollup-' + row.id}
        >
          <strong>{row.label} · {row.status}</strong>
          <p>
            Direct: {rollups.copies_without_direct_assignment.includes(row.id)
              ? 'Not assigned'
              : money(row.direct.known_cents)}{row.direct.incomplete
              ? ' + unknown costs'
              : ''}. Allocated parts: {money(
              row.allocated_parts.known_cents,
            )}{row.allocated_parts.incomplete ? ' + unknown costs' : ''}.
          </p>
          <p>
            Combined copy view: {money(row.total.known_cents)}{row.total
              .incomplete
              ? ' + unknown costs'
              : ''}.
          </p>
        </div>{/each}
      <h3>Projects</h3>
      {#each values(rollups.projects) as row}<div class="audit">
          <strong>{row.name}</strong>
          <p>
            Allocated parts: {money(row.allocated_parts.known_cents)}{row
              .allocated_parts.incomplete
              ? ' + unknown costs'
              : ''}. Linked copy direct costs remain in the copy view.
          </p>
        </div>{/each}{/if}
  </section>
</fieldset>
