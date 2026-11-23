<script lang="ts">
  import {
    api,
    envelope,
    uuid,
    integer,
    scaled,
    measure,
    known,
    money,
    amount,
    type Data,
  } from './api';
  let { data }: { data: Data } = $props();
  let order = $state(''),
    view = $state<Data | null>(null),
    error = $state(''),
    busy = $state(false),
    proposal = $state<Data | null>(null),
    reason = $state('');
  let shipmentId = $state(''),
    shipmentLabel = $state(''),
    tracking = $state(''),
    shipmentStatus = $state('Expected');
  let warrantyId = $state(''),
    warrantyLabel = $state(''),
    warrantyStatus = $state('Open'),
    warrantyNotes = $state('');
  let date = $state(''),
    purpose = $state('Purchase'),
    claimId = $state(''),
    notes = $state(''),
    items = $state<Data[]>([]);
  const values = (v: unknown): Data[] => Object.values(v ?? {}) as Data[];
  let physical = $state<Data>({ holdings: {}, copies: [] });
  let grossRows = $state<Data[]>([]),
    moneyKind = $state('Coupon'),
    moneyLabel = $state(''),
    moneyAmount = $state(''),
    moneyMethod = $state('PriceProportional'),
    paymentMethod = $state('Card');
  let returnLabel = $state(''),
    returnId = $state(''),
    returnKind = $state('Holding'),
    returnTarget = $state(''),
    returnPieces = $state(''),
    returnGrams = $state(''),
    returnDamaged = $state(''),
    returnDamagedGrams = $state('');
  let returnParts = $state<Data[]>([]);
  let returnSources = $state<Data[]>([]);
  function selectReturnCopy() {
    const layers = physical.holdings[returnTarget]?.acquisition_layers ?? {};
    returnSources =
      Object.keys(layers).length > 1
        ? Object.entries(layers).map(([id, n]) => ({
            id,
            available: n,
            pieces: '',
            grams: '',
          }))
        : [];
    const c = physical.copies.find((c: Data) => c.id === returnTarget);
    returnParts = (c?.linked ?? []).map((id: string) => ({
      id,
      handling: '',
      pieces: '',
      grams: '',
      damage: '',
      damageGrams: '',
    }));
  }
  function financialRecord() {
    void preview({
      Financial: {
        Record: {
          id: uuid(),
          kind: moneyKind,
          label: moneyLabel,
          effective_on: date,
          amount_cents: scaled(moneyAmount, 2),
          method: moneyMethod,
          payment_method: moneyKind === 'Payment' ? paymentMethod : null,
          warranty_id: claimId || null,
          return_id: returnId || null,
        },
      },
    });
  }
  function saveGross() {
    try {
      void preview({
        Financial: {
          SetGross: {
            lines: Object.fromEntries(
              grossRows.map((r) => {
                const n = scaled(r.gross, 2);
                if (n === null)
                  throw new Error('Enter every gross line amount.');
                return [r.id, n];
              }),
            ),
          },
        },
      });
    } catch (e) {
      error = (e as Error).message;
    }
  }
  function dispatchReturn() {
    try {
      let target: Data;
      if (returnKind === 'Holding') {
        const h = physical.holdings[returnTarget];
        if (!h) throw new Error('Choose returned stock.');
        target = {
          Holding: {
            holding_id: returnTarget,
            expected_revision: h.holding.revision,
            sources: returnSources
              .filter((r) => r.pieces !== '' || r.grams !== '')
              .map((r) => ({
                invoice_line_id: r.id,
                pieces: integer(r.pieces),
                weight_mg: scaled(r.grams, 3),
              })),
            portion: {
              pieces: integer(returnPieces),
              weight_mg: scaled(returnGrams, 3),
              damaged_pieces: integer(returnDamaged),
              damaged_weight_mg: scaled(returnDamagedGrams, 3),
            },
          },
        };
      } else {
        const c = physical.copies.find((c: Data) => c.id === returnTarget);
        if (!c) throw new Error('Choose returned copy.');
        target = {
          Copy: {
            copy_id: returnTarget,
            expected_revision: c.lifecycle.revision,
            status: 'Returned',
            effective_on: date,
            proceeds_cents: null,
            recipient: null,
            notes,
            corrects: null,
            parts: returnParts.map((r) => {
              if (!r.handling)
                throw new Error(
                  'Choose explicit handling for each linked holding.',
                );
              return {
                holding_id: r.id,
                expected_revision: physical.holdings[r.id].holding.revision,
                handling:
                  r.handling === 'Custom'
                    ? {
                        Custom: {
                          child_id: uuid(),
                          keep: {
                            pieces: integer(r.pieces),
                            weight_mg: scaled(r.grams, 3),
                            damaged_pieces: integer(r.damage),
                            damaged_weight_mg: scaled(r.damageGrams, 3),
                          },
                        },
                      }
                    : r.handling,
              };
            }),
          },
        };
      }
      void preview({
        SupplierReturn: {
          Dispatch: { return_id: returnId, effective_on: date, target },
        },
      });
    } catch (e) {
      error = (e as Error).message;
    }
  }
  let backorderLine = $state(''),
    backorderQty = $state('');
  const count = (m: unknown) => known(m) || 'Unknown';
  async function load() {
    if (!order) {
      view = null;
      return;
    }
    busy = true;
    error = '';
    proposal = null;
    try {
      view = await api('/fulfilment/' + order);
      physical = await api('/physical');
      grossRows = view!.purchase.draft.lines.map((l: Data) => ({
        id: l.id,
        label: l.description ?? l.set_number ?? 'Invoice line',
        gross:
          view!.state.financial?.gross[l.id] === undefined
            ? ''
            : amount(view!.state.financial.gross[l.id]),
      }));
      items = view!.purchase.draft.lines.map((l: Data) => ({
        id: l.id,
        label: l.description ?? l.set_number ?? 'Unnamed line',
        kind: l.kind === 'Set' ? 'Set' : 'Parts',
        enabled: false,
        copies: '',
        pieces: '',
        grams: '',
        assembly: l.assembly ?? 'Loose',
      }));
    } catch (e) {
      error = (e as Error).message;
    } finally {
      busy = false;
    }
  }
  async function preview(action: unknown) {
    if (!view) return;
    busy = true;
    error = '';
    try {
      if (!reason.trim()) throw new Error('Enter an audit reason.');
      proposal = await api('/fulfilment/preview', 'POST', {
        ...envelope(),
        reason,
        purchase_id: order,
        expected_revision: view.state.revision,
        expected_purchase_revision: view.purchase.revision,
        action,
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
      await api('/fulfilment', 'POST', {
        command: proposal.command,
        expected_inventory_sequence: proposal.expected_inventory_sequence,
        expected_profile_sequence: proposal.expected_profile_sequence,
      });
      await load();
    } catch (e) {
      error = (e as Error).message;
    } finally {
      busy = false;
    }
  }
  async function receipt() {
    try {
      const lines = items
        .filter((l) => l.enabled)
        .map((l) => {
          let stock: Data;
          if (l.kind === 'Set') {
            const n = integer(l.copies);
            if (n !== null && n > 10000)
              throw new Error('A receipt supports at most 10,000 copies.');
            stock = {
              Sets: {
                copies: Array.from({ length: n ?? 0 }, (_, i) => ({
                  id: uuid(),
                  label: l.label + ' · received ' + (i + 1),
                  assembly: l.assembly,
                })),
              },
            };
          } else
            stock = {
              Batch: {
                holding_id: uuid(),
                pieces: measure(l.pieces),
                weight_mg: measure(l.grams, 3),
              },
            };
          return { invoice_line_id: l.id, stock };
        });
      await preview({
        Receipt: {
          id: uuid(),
          shipment_id: shipmentId || null,
          purpose:
            purpose === 'Purchase'
              ? 'Purchase'
              : { Warranty: { claim_id: claimId } },
          effective_on: date,
          lines,
          notes,
        },
      });
    } catch (e) {
      error = (e as Error).message;
    }
  }
  function chooseShipment(event: Event) {
    shipmentId = (event.target as HTMLSelectElement).value;
    const s = view?.state.shipments[shipmentId];
    shipmentLabel = s?.label ?? '';
    tracking = s?.tracking ?? '';
    shipmentStatus = s?.status ?? 'Expected';
    proposal = null;
  }
  function chooseWarranty(event: Event) {
    warrantyId = (event.target as HTMLSelectElement).value;
    const w = view?.state.warranties[warrantyId];
    warrantyLabel = w?.label ?? '';
    warrantyStatus = w?.status ?? 'Open';
    warrantyNotes = w?.notes ?? '';
    proposal = null;
  }
  async function backorder() {
    try {
      if (!view) return;
      const c = view.coverage.find((c: Data) => c.line_id === backorderLine);
      const n = integer(backorderQty);
      await preview({
        Backorder: {
          line_id: backorderLine,
          copies: c?.unit === 'copies' ? n : null,
          pieces: c?.unit === 'pieces' ? n : null,
          weight_mg: c?.unit === 'mg' ? n : null,
          notes,
        },
      });
    } catch (e) {
      error = (e as Error).message;
    }
  }
</script>

<p>
  Orders track intent; only confirmed receipt creates ownership. Refunds,
  returns and warranty logistics are separate events.
</p>
{#if error}<p role="alert" class="message error">{error}</p>{/if}
<fieldset class="workflows" disabled={busy}>
  <section>
    <h2>Purchase fulfilment</h2>
    <label
      >Order<select
        value={order}
        onchange={(e) => {
          order = e.currentTarget.value;
          void load();
        }}
        ><option value="">Choose purchase order</option
        >{#each data.purchases as p}<option value={p.id}
            >{p.draft.order_number ?? p.draft.seller ?? 'Unnamed purchase'} · {p.id.slice(
              -8,
            )}</option
          >{/each}</select
      ></label
    >
    {#if view}<p>
        Current fulfilment: <strong>{view.status}</strong>. Source purchase
        revision {view.purchase.revision}.
      </p>
      <div class="table-wrap">
        <table>
          <thead
            ><tr
              ><th>Invoice line</th><th>Ordered</th><th>Net received</th><th
                >Outstanding</th
              ><th>Backorder</th></tr
            ></thead
          ><tbody
            >{#each view.coverage as c}<tr
                ><td
                  >{view.purchase.draft.lines.find(
                    (l: Data) => l.id === c.line_id,
                  )?.description ?? c.line_id.slice(-8)}</td
                ><td>{count(c.ordered)} {c.unit}</td><td>{count(c.received)}</td
                ><td>{count(c.outstanding)}</td><td
                  >{c.backordered ? 'Yes' : 'No'}</td
                ></tr
              >{/each}</tbody
          >
        </table>
      </div>
      <label
        >Audit reason<input
          bind:value={reason}
          oninput={() => (proposal = null)}
        /></label
      >{/if}
  </section>
  {#if view}
    <section
      oninput={() => (proposal = null)}
      onchange={() => (proposal = null)}
    >
      <h2>Shipments and tracking</h2>
      <label
        >Shipment<select value={shipmentId} onchange={chooseShipment}
          ><option value="">New shipment / no shipment selected</option
          >{#each Object.values(view.state.shipments) as s}<option
              value={(s as Data).id}
              >{(s as Data).label} · {(s as Data).status}</option
            >{/each}</select
        ></label
      >
      <div class="form-grid">
        <label>Shipment label<input bind:value={shipmentLabel} /></label><label
          >Tracking reference<input bind:value={tracking} /></label
        ><label
          >Logistics status<select bind:value={shipmentStatus}
            >{#each ['Expected', 'Shipped', 'Received', 'Lost', 'Returned'] as status}<option
                value={status}>{status}</option
              >{/each}</select
          ></label
        >
      </div>
      <p>
        Marking a shipment Shipped never adds stock. Received requires an actual
        receipt.
      </p>
      <button
        onclick={() =>
          preview({
            Shipment: {
              id: shipmentId || uuid(),
              label: shipmentLabel,
              tracking,
              status: shipmentStatus,
              notes,
            },
          })}>Review shipment update</button
      >
    </section>
    <section
      oninput={() => (proposal = null)}
      onchange={() => (proposal = null)}
    >
      <h2>Receive actual items</h2>
      <div class="form-grid">
        <label>Receipt date<input type="date" bind:value={date} /></label><label
          >Receipt purpose<select bind:value={purpose}
            ><option value="Purchase">Ordered acquisition</option><option
              value="Warranty">Warranty replacement</option
            ></select
          ></label
        >{#if purpose === 'Warranty'}<label
            >Warranty claim<select bind:value={claimId}
              ><option value="">Choose claim</option
              >{#each Object.values(view.state.warranties) as c}<option
                  value={(c as Data).id}>{(c as Data).label}</option
                >{/each}</select
            ></label
          >{/if}
      </div>
      <label>Receipt notes<textarea bind:value={notes}></textarea></label>
      <p>
        Select only what physically arrived. Set receipts create copy
        identities, not detailed part stock. Unknown copy count creates no
        invented copies. Warranty receipt never invents an acquisition price.
      </p>
      {#each items as item}<div class="panel">
          <label class="check"
            ><input type="checkbox" bind:checked={item.enabled} /> Receive {item.label}</label
          >{#if item.enabled}{#if purpose === 'Warranty'}<label
                >Replacement kind<select bind:value={item.kind}
                  ><option value="Parts">Parts</option><option value="Set"
                    >Set copy</option
                  ></select
                ></label
              >{/if}{#if item.kind === 'Set'}<label
                >Received physical copies (blank means unknown)<input
                  bind:value={item.copies}
                  inputmode="numeric"
                /></label
              ><label
                >Received construction state<select bind:value={item.assembly}
                  ><option>Loose</option><option>Assembled</option><option
                    >PartiallyAssembled</option
                  ></select
                ></label
              >{:else}<div class="form-grid">
                <label
                  >Received pieces<input
                    bind:value={item.pieces}
                    inputmode="numeric"
                  /></label
                ><label
                  >Received weight · g<input
                    bind:value={item.grams}
                    inputmode="decimal"
                  /></label
                >
              </div>{/if}{/if}
        </div>{/each}
      <button onclick={receipt}>Review partial receipt</button>
    </section>
    <section
      oninput={() => (proposal = null)}
      onchange={() => (proposal = null)}
    >
      <h2>Backorders and closure</h2>
      <label
        >Backordered line<select bind:value={backorderLine}
          ><option value="">Choose line</option
          >{#each view.coverage as c}<option value={c.line_id}
              >{view.purchase.draft.lines.find((l: Data) => l.id === c.line_id)
                ?.description ?? c.line_id.slice(-8)} · {c.unit}</option
            >{/each}</select
        ></label
      ><label
        >Backordered quantity in the displayed unit<input
          bind:value={backorderQty}
          inputmode="numeric"
        /></label
      ><button onclick={backorder}>Review backorder</button><button
        onclick={() => preview({ Close: { closed: !view?.state.closed } })}
        >{view.state.closed
          ? 'Review reopening'
          : 'Review closing outstanding order'}</button
      >
      <p>
        Closing/cancelling remaining intent does not delete already received
        stock.
      </p>
    </section>
    <section
      oninput={() => (proposal = null)}
      onchange={() => (proposal = null)}
    >
      <h2>Warranty claims</h2>
      <label
        >Claim<select value={warrantyId} onchange={chooseWarranty}
          ><option value="">New warranty claim</option
          >{#each Object.values(view.state.warranties) as w}<option
              value={(w as Data).id}
              >{(w as Data).label} · {(w as Data).status}</option
            >{/each}</select
        ></label
      >
      <div class="form-grid">
        <label>Claim label<input bind:value={warrantyLabel} /></label><label
          >Claim state<select bind:value={warrantyStatus}
            >{#each ['Open', 'Submitted', 'Approved', 'Rejected', 'ReplacementSent', 'Completed'] as status}<option
                value={status}>{status}</option
              >{/each}</select
          ></label
        >
      </div>
      <label>Claim notes<textarea bind:value={warrantyNotes}></textarea></label
      ><button
        onclick={() =>
          preview({
            Warranty: {
              id: warrantyId || uuid(),
              label: warrantyLabel,
              status: warrantyStatus,
              notes: warrantyNotes,
            },
          })}>Review warranty update</button
      >
      <p>
        Claim status alone never adds stock or pays money. Record replacements
        through an actual warranty receipt.
      </p>
    </section>
    <section
      oninput={() => (proposal = null)}
      onchange={() => (proposal = null)}
    >
      <h2>Financial adjustments and payments</h2>
      <p>
        Recorded paid invoice totals remain unchanged. Enter gross amounts
        separately before discounts. Gift cards and vouchers are payments.
      </p>
      <div class="table-wrap">
        <table>
          <thead
            ><tr
              ><th>Invoice line</th><th>Recorded paid</th><th>Current net</th
              ><th>Refunded</th></tr
            ></thead
          ><tbody
            >{#each view.purchase.draft.lines as line}<tr
                ><td>{line.description ?? line.set_number ?? 'Invoice line'}</td
                ><td>{money(line.paid_total_cents)}</td><td
                  >{money(view.financial_summary.net_lines[line.id])}</td
                ><td
                  >{money(
                    view.financial_summary.refunded_lines[line.id] ?? 0,
                  )}</td
                ></tr
              >{/each}</tbody
          >
        </table>
      </div>
      <label
        >Financial warranty claim (optional)<select bind:value={claimId}
          ><option value="">No claim link</option
          >{#each values(view.state.warranties) as claim}<option
              value={claim.id}>{claim.label}</option
            >{/each}</select
        ></label
      ><label
        >Financial supplier return (optional)<select bind:value={returnId}
          ><option value="">No return link</option
          >{#each values(view.state.returns) as r}<option value={r.id}
              >{r.label}</option
            >{/each}</select
        ></label
      >
      {#each grossRows as row}<label
          >Gross · {row.label} · EUR<input
            bind:value={row.gross}
            inputmode="decimal"
          /></label
        >{/each}
      <button onclick={saveGross}>Review gross basis</button>
      <div class="form-grid">
        <label
          >Financial event<select bind:value={moneyKind}
            >{#each ['Discount', 'Coupon', 'LoyaltyReduction', 'Refund', 'Payment'] as kind}<option
                >{kind}</option
              >{/each}</select
          ></label
        ><label>Financial label<input bind:value={moneyLabel} /></label><label
          >Amount · EUR<input
            bind:value={moneyAmount}
            inputmode="decimal"
          /></label
        ><label>Event date<input type="date" bind:value={date} /></label>
        {#if moneyKind === 'Payment'}<label
            >Payment instrument<select bind:value={paymentMethod}
              >{#each ['Cash', 'Card', 'Bank', 'GiftCard', 'StoredValueVoucher'] as kind}<option
                  >{kind}</option
                >{/each}</select
            ></label
          >{:else}<label
            >Automatic line allocation<select bind:value={moneyMethod}
              >{#each ['PriceProportional', 'Equal', 'PieceCount', 'Weight'] as kind}<option
                  >{kind}</option
                >{/each}</select
            ></label
          >{/if}
      </div>
      <button onclick={financialRecord}>Review financial event</button>
      {#each view.state.financial?.entries ?? [] as r}<div class="audit">
          <strong>{r.entry.kind} · {r.entry.label}</strong> · {money(
            r.entry.amount_cents,
          )} EUR
          {#if view.state.financial.corrected.includes(r.entry.id)}<p>
              Corrected; original retained.
            </p>{:else}<button
              onclick={() =>
                preview({
                  Financial: { CorrectLast: { entry_id: r.entry.id } },
                })}>Review correcting financial entry</button
            >{/if}
        </div>{/each}
      <p>
        Recorded payments: {money(view.financial_summary?.payments_cents ?? 0)} EUR.
        Refunds and financial corrections do not move stock.
      </p>
    </section>
    <section
      oninput={() => (proposal = null)}
      onchange={() => (proposal = null)}
    >
      <h2>Supplier returns</h2>
      <p>
        Plan first, then confirm actual physical handover. Acknowledgment and
        refunds do not remove stock again.
      </p>
      <label>Return label<input bind:value={returnLabel} /></label><label
        >Return date<input type="date" bind:value={date} /></label
      >
      <button
        onclick={() =>
          preview({
            SupplierReturn: {
              Plan: {
                id: uuid(),
                label: returnLabel,
                requested_on: date,
                notes,
                warranty_id: claimId || null,
                dispatched_by: null,
                acknowledged_on: null,
                corrected_by: null,
              },
            },
          })}>Review return plan</button
      >
      <label
        >Supplier return<select bind:value={returnId}
          ><option value="">Choose return</option
          >{#each values(view.state.returns) as r}<option value={r.id}
              >{r.label}</option
            >{/each}</select
        ></label
      >
      <label
        >Returned item type<select
          bind:value={returnKind}
          onchange={() => {
            returnTarget = '';
            returnParts = [];
          }}><option>Holding</option><option>Copy</option></select
        ></label
      >
      <label
        >Returned stock<select
          bind:value={returnTarget}
          onchange={selectReturnCopy}
          ><option value="">Choose owned stock</option
          >{#if returnKind === 'Holding'}{#each values(physical.holdings) as h}<option
                value={h.id}>{h.label} · {h.id.slice(-8)}</option
              >{/each}{:else}{#each physical.copies as c}<option value={c.id}
                >{c.label} · {c.lifecycle.status}</option
              >{/each}{/if}</select
        ></label
      >
      {#if returnKind === 'Holding'}<div class="form-grid">
          <label
            >Returned pieces<input
              bind:value={returnPieces}
              inputmode="numeric"
            /></label
          ><label
            >Returned weight · g<input
              bind:value={returnGrams}
              inputmode="decimal"
            /></label
          ><label
            >Returned damaged pieces<input
              bind:value={returnDamaged}
              inputmode="numeric"
            /></label
          ><label
            >Returned damaged weight · g<input
              bind:value={returnDamagedGrams}
              inputmode="decimal"
            /></label
          >
        </div>
        {#if returnSources.length}<h3>Explicit acquisition quantities</h3>
          <p>
            Select quantities from this order's source lines. Totals must equal
            the dispatched stock; no FIFO or proportional source choice is
            assumed.
          </p>
          {#each returnSources as row}<div class="audit">
              <strong
                >{data.purchases
                  .flatMap((p: Data) => p.draft.lines)
                  .find((l: Data) => l.id === row.id)?.description ??
                  'Invoice line'} · {row.id.slice(-8)}</strong
              >
              <p>
                Available source pieces: {row.available?.pieces ?? 'Unknown'}
              </p>
              <label
                >Source returned pieces<input
                  bind:value={row.pieces}
                  inputmode="numeric"
                /></label
              ><label
                >Source returned weight · g<input
                  bind:value={row.grams}
                  inputmode="decimal"
                /></label
              >
            </div>{/each}{/if}
      {:else}
        {#each returnParts as r}<p>
            {physical.holdings[r.id]?.label ?? 'Linked stock'}
          </p>
          <label
            >Linked part disposition<select bind:value={r.handling}
              ><option value="">Choose explicitly</option><option
                value="TransferAll">Return all linked parts</option
              ><option value="KeepAll">Keep all as loose stock</option><option
                value="Custom">Keep custom quantity</option
              ></select
            ></label
          >{#if r.handling === 'Custom'}<div class="form-grid">
              <label>Kept pieces<input bind:value={r.pieces} /></label><label
                >Kept weight · g<input bind:value={r.grams} /></label
              ><label>Kept damaged pieces<input bind:value={r.damage} /></label
              ><label
                >Kept damaged weight · g<input
                  bind:value={r.damageGrams}
                /></label
              >
            </div>{/if}{/each}{/if}
      <button onclick={dispatchReturn}>Review actual return dispatch</button>
      {#each values(view.state.returns) as r}<div class="audit">
          <strong>{r.label}</strong> · {r.corrected_by
            ? 'Dispatch corrected'
            : r.acknowledged_on
              ? 'Supplier acknowledged'
              : r.dispatched_by
                ? 'Dispatched'
                : 'Planned'}
          {#if r.dispatched_by && !r.corrected_by}<button
              onclick={() =>
                preview({
                  SupplierReturn: {
                    Acknowledge: { return_id: r.id, effective_on: date },
                  },
                })}>Review supplier acknowledgment</button
            ><button
              onclick={() =>
                preview({
                  SupplierReturn: {
                    CorrectDispatch: {
                      return_id: r.id,
                      dispatch_operation_id: r.dispatched_by,
                      effective_on: date,
                    },
                  },
                })}>Review correcting return dispatch</button
            >{/if}
        </div>{/each}
    </section>
    <section>
      <h2>Receipt corrections</h2>
      <p>
        Reverse a mistaken receipt, or correct a mistaken reversal, without
        deleting history. Later stock changes, assignments or claims can block
        the inverse. Set identities remain as Void while reversed.
      </p>
      {#each view.receipts as receipt}<div class="audit">
          <strong>{receipt.operation.id.slice(-8)}</strong> · {receipt.reversed
            ? 'Reversed'
            : 'Received'}<button
            onclick={() =>
              preview({
                ReceiptCorrection: {
                  receipt_operation_id: receipt.operation.id,
                  reverse: !receipt.reversed,
                },
              })}
            >{receipt.reversed
              ? 'Review correcting reversal'
              : 'Review reversing receipt'}</button
          >
        </div>{/each}
    </section>
    <section>
      <h2>Recent order events</h2>
      {#each view.history as h}<div class="audit">
          <strong>{Object.keys(h.command.action)[0]}</strong> · {h.at} · {h
            .command.actor}
          <p>{h.command.reason}</p>
        </div>{/each}
    </section>
  {/if}
  {#if proposal}<section>
      <h2>Review fulfilment effects</h2>
      <p>{proposal.command.reason}</p>
      <p>Resulting order status: {proposal.status_after}</p>
      {#if proposal.financial_after}<h3>Financial result</h3>
        {#each view?.purchase.draft.lines ?? [] as line}<p>
            {line.description ?? line.set_number ?? 'Invoice line'}: {proposal
              .financial_after.net_lines[line.id] === null
              ? 'Unknown'
              : money(proposal.financial_after.net_lines[line.id])} net; {money(
              proposal.financial_after.refunded_lines[line.id] ?? 0,
            )} EUR refunded.
          </p>{/each}{/if}

      {#each proposal.effects.holdings as effect}<p>
          {count(effect.before?.pieces.owned)} → {count(
            effect.after.pieces.owned,
          )} owned pieces
        </p>{/each}{#if Object.keys(proposal.effects.copies).length}<p>
          New physical set copies: {Object.keys(proposal.effects.copies)
            .length}. No detailed parts are invented.
        </p>{/if}<button class="primary" onclick={apply}
        >Apply reviewed fulfilment event</button
      ><button onclick={() => (proposal = null)}>Cancel</button>
    </section>{/if}
</fieldset>
