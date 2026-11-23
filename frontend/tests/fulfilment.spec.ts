import { test, expect } from '@playwright/test';
import { uuid } from '../src/api';
test('partial receipts and reversals are reviewed and survive reload', async ({
  page,
}, info) => {
  await page.route('**/*', (r) =>
    new URL(r.request().url()).hostname === '127.0.0.1'
      ? r.continue()
      : r.abort(),
  );
  const errors: string[] = [];
  page.on('pageerror', (e) => errors.push(e.message));
  await page.goto('/');
  const purchase = uuid(),
    line = uuid(),
    holding = uuid(),
    receipt = uuid(),
    op = uuid();
  await page.evaluate(
    async ({ purchase, line, holding, receipt, op }) => {
      const token = (await (await fetch('/api/v1/bootstrap')).json()).token;
      const uid = () => {
        const t = Date.now().toString(16).padStart(12, '0'),
          r = crypto.randomUUID().replaceAll('-', '');
        return `${t.slice(0, 8)}-${t.slice(8)}-7${r.slice(0, 3)}-8${r.slice(3, 6)}-${r.slice(6, 18)}`;
      };
      async function post(path: string, body: unknown) {
        const r = await fetch('/api/v1/' + path, {
          method: 'POST',
          headers: {
            'content-type': 'application/json',
            'x-brickxinator-token': token,
          },
          body: JSON.stringify(body),
        });
        if (!r.ok) throw new Error(await r.text());
        return r.json();
      }
      await post('commands/purchases', {
        id: uid(),
        request_key: uid(),
        actor: 'test',
        reason: 'browser fixture',
        body: {
          Create: {
            purchase_id: purchase,
            draft: {
              notes: 'browser partial receipt',
              lines: [
                {
                  id: line,
                  kind: 'LooseBatch',
                  description: 'Partial receipt fixture',
                  pieces: { Known: 100 },
                  weight_mg: 'Unknown',
                  paid_total_cents: 1000,
                  assembly: 'Loose',
                },
              ],
            },
          },
        },
      });
      const c = {
        id: op,
        request_key: uid(),
        actor: 'test',
        reason: 'actual arrival',
        purchase_id: purchase,
        expected_revision: 0,
        expected_purchase_revision: 1,
        action: {
          Receipt: {
            id: receipt,
            shipment_id: null,
            purpose: 'Purchase',
            effective_on: '2026-11-23',
            lines: [
              {
                invoice_line_id: line,
                stock: {
                  Batch: {
                    holding_id: holding,
                    pieces: { Known: 80 },
                    weight_mg: 'Unknown',
                  },
                },
              },
            ],
            notes: 'partial delivery',
          },
        },
      };
      const p = await post('fulfilment/preview', c);
      const before = await (
        await fetch('/api/v1/fulfilment/' + purchase)
      ).json();
      if (before.state.revision !== 0)
        throw new Error('Preview mutated history');
      await post('fulfilment', {
        command: p.command,
        expected_inventory_sequence: p.expected_inventory_sequence,
        expected_profile_sequence: p.expected_profile_sequence,
      });
    },
    { purchase, line, holding, receipt, op },
  );
  await page.reload();
  await page
    .getByRole('button', { name: 'Orders & warranty', exact: true })
    .click();
  await page.locator('select').first().selectOption(purchase);
  await expect(
    page.getByText('PartiallyReceived', { exact: false }).first(),
  ).toBeVisible();
  await expect(
    page.getByText('Partial receipt fixture', { exact: false }).first(),
  ).toBeVisible();
  await page.getByLabel('Audit reason').fill('mistaken delivery scan');
  await page
    .getByRole('button', { name: 'Review reversing receipt', exact: true })
    .click();
  await expect(
    page.getByRole('button', {
      name: 'Apply reviewed fulfilment event',
      exact: true,
    }),
  ).toBeVisible();
  await page
    .getByRole('button', {
      name: 'Apply reviewed fulfilment event',
      exact: true,
    })
    .click();
  await expect(page.getByText('Open', { exact: true }).first()).toBeVisible();
  await page.setViewportSize({ width: 390, height: 844 });
  expect(
    await page.evaluate(
      () => document.documentElement.scrollWidth <= innerWidth,
    ),
  ).toBe(true);
  await page.screenshot({
    path: info.outputPath('fulfilment-mobile.png'),
    fullPage: true,
  });
  expect(errors).toEqual([]);
});
