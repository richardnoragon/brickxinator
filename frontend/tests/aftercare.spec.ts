import { test, expect } from '@playwright/test';
import { uuid } from '../src/api';
test('financial review, physical supplier return and derived costs remain separate', async ({
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
    line = uuid();
  const holding = await page.evaluate(
    async ({ purchase, line }) => {
      const token = (await (await fetch('/api/v1/bootstrap')).json()).token;
      const uid = () => {
        const t = Date.now().toString(16).padStart(12, '0'),
          r = crypto.randomUUID().replaceAll('-', '');
        return `${t.slice(0, 8)}-${t.slice(8)}-7${r.slice(0, 3)}-8${r.slice(3, 6)}-${r.slice(6, 18)}`;
      };
      async function post(body: unknown) {
        const r = await fetch('/api/v1/commands/purchases', {
          method: 'POST',
          headers: {
            'content-type': 'application/json',
            'x-brickxinator-token': token,
          },
          body: JSON.stringify({
            id: uid(),
            request_key: uid(),
            actor: 'test',
            reason: 'disposable aftercare acceptance',
            body,
          }),
        });
        if (!r.ok) throw new Error(await r.text());
        return r.json();
      }
      await post({
        Create: {
          purchase_id: purchase,
          draft: {
            shipping_cents: 0,
            fees_cents: 0,
            notes: '',
            lines: [
              {
                id: line,
                kind: 'MixedLot',
                description: 'Aftercare bricks',
                pieces: { Known: 100 },
                weight_mg: 'Unknown',
                volume_ml: 1000,
                paid_total_cents: 2500,
                assembly: 'Loose',
              },
            ],
          },
        },
      });
      const r = await post({
        Receive: { purchase_id: purchase, expected_revision: 1 },
      });
      return Object.keys(r.receipt.holdings)[0];
    },
    { purchase, line },
  );
  await page.reload();
  await page
    .getByRole('button', { name: 'Orders & warranty', exact: true })
    .click();
  await page
    .getByRole('combobox', { name: 'Order', exact: true })
    .selectOption(purchase);
  await page
    .getByLabel('Audit reason', { exact: true })
    .fill('reviewed money and physical flow');
  await page.getByLabel('Gross · Aftercare bricks · EUR').fill('30');
  await page
    .getByRole('button', { name: 'Review gross basis', exact: true })
    .click();
  await page
    .getByRole('button', {
      name: 'Apply reviewed fulfilment event',
      exact: true,
    })
    .click();
  await page.getByLabel('Financial label').fill('Coupon');
  await page.getByLabel('Amount · EUR', { exact: true }).fill('5');
  await page.getByLabel('Event date', { exact: true }).fill('2026-11-23');
  await page
    .getByRole('button', { name: 'Review financial event', exact: true })
    .click();
  await expect(page.getByText('€25.00 net', { exact: false })).toBeVisible();
  await page
    .getByRole('button', {
      name: 'Apply reviewed fulfilment event',
      exact: true,
    })
    .click();
  await page.getByLabel('Return label', { exact: true }).fill('Wrong pieces');
  await page
    .getByRole('button', { name: 'Review return plan', exact: true })
    .click();
  await page
    .getByRole('button', {
      name: 'Apply reviewed fulfilment event',
      exact: true,
    })
    .click();
  await page
    .getByRole('combobox', { name: 'Supplier return', exact: true })
    .selectOption({ label: 'Wrong pieces' });
  await page
    .getByRole('combobox', { name: 'Returned stock', exact: true })
    .selectOption(holding);
  await page.getByLabel('Returned pieces', { exact: true }).fill('20');
  await page.getByLabel('Returned damaged pieces', { exact: true }).fill('0');
  await page
    .getByRole('button', { name: 'Review actual return dispatch', exact: true })
    .click();
  await expect(
    page.getByText('100 → 80 owned pieces', { exact: false }),
  ).toBeVisible();
  await page
    .getByRole('button', {
      name: 'Apply reviewed fulfilment event',
      exact: true,
    })
    .click();
  await page
    .getByRole('button', { name: 'Cost allocations', exact: true })
    .click();
  await page
    .getByRole('combobox', { name: 'Cost source purchase', exact: true })
    .selectOption(purchase);
  await page
    .getByRole('combobox', { name: 'Source invoice line', exact: true })
    .selectOption(line);
  await page.getByLabel('Amount to allocate · EUR').fill('20');
  await page.getByLabel('Cost effective date').fill('2026-11-23');
  await page
    .getByLabel('Cost audit reason')
    .fill('explicit derived attribution');
  await page
    .getByLabel(
      new RegExp('Unidentified stock.*80 pieces.*' + holding.slice(-8)),
    )
    .check();
  await page.getByLabel('Derived share · EUR').fill('20');
  await page
    .getByRole('button', { name: 'Review cost allocation', exact: true })
    .click();
  await expect(
    page
      .getByText('Unallocated source balance: €5.00', { exact: false })
      .first(),
  ).toBeVisible();
  await page
    .getByRole('button', {
      name: 'Apply reviewed cost allocation',
      exact: true,
    })
    .click();
  await expect(
    page.getByText('Item · revision 1', { exact: true }),
  ).toBeVisible();
  await page.setViewportSize({ width: 390, height: 844 });
  expect(
    await page.evaluate(
      () => document.documentElement.scrollWidth <= innerWidth,
    ),
  ).toBe(true);
  await page.screenshot({
    path: info.outputPath('cost-mobile.png'),
    fullPage: true,
  });
  expect(errors).toEqual([]);
});
test('reviewed Rebrickable CSV catalog import is cached and Markdown downloads offline', async ({
  page,
}, info) => {
  await page.route('**/*', (r) =>
    new URL(r.request().url()).hostname === '127.0.0.1'
      ? r.continue()
      : r.abort(),
  );
  await page.goto('/');
  await page
    .getByRole('button', { name: 'Imports & exports', exact: true })
    .click();
  const part = 'adapter-' + info.project.name + '-' + Date.now();
  await page.getByLabel('Local source file').setInputFiles({
    name: 'parts.csv',
    mimeType: 'text/csv',
    buffer: Buffer.from(
      `part_num,name,part_cat_id\n${part},Adapter brick,11\n`,
    ),
  });
  await page
    .getByLabel('Import audit reason')
    .fill('reviewed catalog enrichment');
  await page
    .getByLabel('Manufacturer for new definitions')
    .selectOption({ label: 'Unknown' });
  await page.getByLabel(`${part} · Adapter brick`, { exact: true }).check();
  await page
    .getByRole('button', {
      name: 'Review selected catalog changes',
      exact: true,
    })
    .click();
  await page
    .getByRole('button', { name: 'Apply reviewed import', exact: true })
    .click();
  await expect(
    page.getByRole('heading', {
      name: 'Review import before applying',
      exact: true,
    }),
  ).toBeHidden();
  await expect(
    page
      .getByRole('button', { name: 'Download cached source', exact: true })
      .first(),
  ).toBeVisible();
  const download = page.waitForEvent('download');
  await page
    .getByRole('button', { name: 'Download collection Markdown', exact: true })
    .click();
  expect((await download).suggestedFilename()).toBe(
    'brickxinator-collection.md',
  );
  await page.setViewportSize({ width: 390, height: 844 });
  expect(
    await page.evaluate(
      () => document.documentElement.scrollWidth <= innerWidth,
    ),
  ).toBe(true);
  await page.screenshot({
    path: info.outputPath('imports-mobile.png'),
    fullPage: true,
  });
});
