import { test, expect } from '@playwright/test';
import { uuid } from '../src/api';
test('direct copy costs, color/theme enrichment and catalog images stay offline and preserve stock', async ({
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
    definition = uuid(),
    tag = `${info.project.name} ${Date.now()}`;
  const fixture = await page.evaluate(
    async ({ purchase, line, definition, tag }) => {
      const token = (await (await fetch('/api/v1/bootstrap')).json()).token;
      const uid = () => {
        const t = Date.now().toString(16).padStart(12, '0'),
          r = crypto.randomUUID().replaceAll('-', '');
        return `${t.slice(0, 8)}-${t.slice(8)}-7${r.slice(0, 3)}-8${r.slice(3, 6)}-${r.slice(6, 18)}`;
      };
      async function post(kind: string, body: object) {
        const r = await fetch('/api/v1/commands/' + kind, {
          method: 'POST',
          headers: {
            'content-type': 'application/json',
            'x-brickxinator-token': token,
          },
          body: JSON.stringify({
            id: uid(),
            request_key: uid(),
            actor: 'test',
            reason: 'disposable copy/enrichment acceptance',
            ...body,
          }),
        });
        if (!r.ok) throw new Error(await r.text());
        return r.json();
      }
      const state = await (await fetch('/api/v1/state')).json();
      const manufacturer = state.settings.find(
        (r: any) =>
          r.kind.Vocabulary?.vocabulary === 'Manufacturer' &&
          r.kind.Vocabulary.system_unknown,
      ).id;
      await post('catalog', {
        action: {
          Create: {
            id: definition,
            kind: 'Set',
            details: {
              manufacturer_id: manufacturer,
              name: 'Image target ' + tag,
              number: 'TEST ' + tag,
            },
          },
        },
      });
      await post('purchases', {
        body: {
          Create: {
            purchase_id: purchase,
            draft: {
              shipping_cents: 0,
              fees_cents: 0,
              notes: '',
              lines: [
                {
                  id: line,
                  kind: 'Set',
                  description: 'Copy rollup ' + tag,
                  copy_count: 1,
                  pieces: 'Unknown',
                  weight_mg: 'Unknown',
                  volume_ml: 1000,
                  paid_total_cents: 10000,
                  assembly: 'Loose',
                },
              ],
            },
          },
        },
      });
      const r = await post('purchases', {
        body: { Receive: { purchase_id: purchase, expected_revision: 1 } },
      });
      const copy = Object.keys(r.receipt.set_copies)[0];
      const costs = await (
        await fetch('/api/v1/child-costs/' + purchase)
      ).json();
      return { copy, sequence: costs.inventory_sequence };
    },
    { purchase, line, definition, tag },
  );
  await page.reload();
  await page
    .getByRole('button', { name: 'Cost allocations', exact: true })
    .click();
  await page
    .getByRole('combobox', { name: 'Cost source purchase', exact: true })
    .selectOption(purchase);
  await page
    .getByRole('combobox', { name: 'Source invoice line', exact: true })
    .selectOption(line);
  await page
    .getByLabel('Amount to allocate · EUR', { exact: true })
    .fill('100');
  await page
    .getByLabel('Cost effective date', { exact: true })
    .fill('2026-11-23');
  await page
    .getByLabel('Cost audit reason', { exact: true })
    .fill('direct cost uses the same source budget');
  await page
    .getByRole('checkbox', {
      name: new RegExp(fixture.copy.slice(-8)),
    })
    .check();
  await page.getByLabel('Direct copy share · EUR', { exact: true }).fill('100');
  await page
    .getByRole('button', { name: 'Review cost allocation', exact: true })
    .click();
  await page
    .getByRole('button', {
      name: 'Apply reviewed cost allocation',
      exact: true,
    })
    .click();
  await expect(
    page
      .getByTestId('copy-cost-rollup-' + fixture.copy)
      .getByText('Combined copy view: €100.00.', { exact: true }),
  ).toBeVisible();
  await page
    .getByRole('button', { name: 'Imports & exports', exact: true })
    .click();
  await page
    .getByRole('combobox', { name: 'Import format', exact: true })
    .selectOption('RebrickableColors');
  const colorId = Date.now(),
    colorName = 'Imported color ' + tag;
  await page.getByLabel('Local source file').setInputFiles({
    name: 'colors.csv',
    mimeType: 'text/csv',
    buffer: Buffer.from(
      `id,name,rgb,is_trans\n${colorId},${colorName},AABBCC,f\n`,
    ),
  });
  await page
    .getByLabel('Import audit reason', { exact: true })
    .fill('explicit local color/theme enrichment');
  await page.getByLabel(`${colorId} · ${colorName}`, { exact: true }).check();
  await page
    .getByRole('button', {
      name: 'Review selected color/theme changes',
      exact: true,
    })
    .click();
  await page
    .getByRole('button', {
      name: 'Apply reviewed color/theme import',
      exact: true,
    })
    .click();
  await expect(
    page.getByRole('heading', {
      name: 'Review enrichment before applying',
      exact: true,
    }),
  ).toBeHidden();
  await expect(page.getByText(colorName, { exact: true })).toBeVisible();
  await page
    .getByRole('combobox', { name: 'Import format', exact: true })
    .selectOption('RebrickableThemes');
  const parent = Date.now(),
    child = parent + 1,
    parentName = 'Train theme ' + tag,
    childName = 'Cargo theme ' + tag;
  await page.getByLabel('Local source file').setInputFiles({
    name: 'themes.csv',
    mimeType: 'text/csv',
    buffer: Buffer.from(
      `id,name,parent_id\n${parent},${parentName},\n${child},${childName},${parent}\n`,
    ),
  });
  await page.getByLabel(`${parent} · ${parentName}`, { exact: true }).check();
  await page.getByLabel(`${child} · ${childName}`, { exact: true }).check();
  await page
    .getByRole('button', {
      name: 'Review selected color/theme changes',
      exact: true,
    })
    .click();
  await page
    .getByRole('button', {
      name: 'Apply reviewed color/theme import',
      exact: true,
    })
    .click();
  await expect(
    page.getByRole('heading', {
      name: 'Review enrichment before applying',
      exact: true,
    }),
  ).toBeHidden();
  await expect(page.getByText(childName, { exact: true })).toBeVisible();
  await page
    .getByRole('combobox', { name: 'Image catalog definition', exact: true })
    .selectOption(definition);
  const base64 = await page.evaluate(() => {
    const c = document.createElement('canvas');
    c.width = c.height = 1;
    c.getContext('2d')!.fillRect(0, 0, 1, 1);
    return c.toDataURL('image/png').split(',')[1];
  });
  await page.getByLabel('Attach catalog image', { exact: true }).setInputFiles({
    name: 'catalog-image.png',
    mimeType: 'image/png',
    buffer: Buffer.from(base64, 'base64'),
  });
  await expect(
    page.getByRole('img', { name: 'catalog-image.png', exact: true }),
  ).toBeVisible();
  const after = await page.evaluate(
    async (p) =>
      (await (await fetch('/api/v1/child-costs/' + p)).json())
        .inventory_sequence,
    purchase,
  );
  expect(after).toBe(fixture.sequence);
  await page.setViewportSize({ width: 390, height: 844 });
  expect(
    await page.evaluate(
      () => document.documentElement.scrollWidth <= innerWidth,
    ),
  ).toBe(true);
  await page.screenshot({
    path: info.outputPath('enrichment-mobile.png'),
    fullPage: true,
  });
  expect(errors).toEqual([]);
});
