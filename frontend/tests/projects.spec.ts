import { test, expect } from '@playwright/test';
import { uuid } from '../src/api';
test('projects reserve verified physical stock and export reviewed requirements', async ({
  page,
}, info) => {
  await page.route('**/*', (route) =>
    new URL(route.request().url()).hostname === '127.0.0.1'
      ? route.continue()
      : route.abort(),
  );
  const failures: string[] = [];
  page.on('pageerror', (e) => failures.push(e.message));
  await page.goto('/');
  const part = uuid(),
    holding = uuid(),
    name = `Project fixture ${info.project.name} ${Date.now()}`;
  await page.evaluate(
    async ({ part, holding, name }) => {
      const token = (await (await fetch('/api/v1/bootstrap')).json()).token;
      const state = await (await fetch('/api/v1/state')).json();
      function id() {
        const chars = crypto.randomUUID().replaceAll('-', '');
        const now = Date.now().toString(16).padStart(12, '0');
        return `${now.slice(0, 8)}-${now.slice(8)}-7${chars.slice(0, 3)}-8${chars.slice(3, 6)}-${chars.slice(6, 18)}`;
      }
      async function post(kind: string, body: object) {
        const r = await fetch('/api/v1/commands/' + kind, {
          method: 'POST',
          headers: {
            'content-type': 'application/json',
            'x-brickxinator-token': token,
          },
          body: JSON.stringify({
            id: id(),
            request_key: id(),
            actor: 'test',
            reason: 'disposable project acceptance',
            ...body,
          }),
        });
        if (!r.ok) throw new Error(await r.text());
        return r.json();
      }
      const unknown = (v: string) =>
        state.settings.find(
          (r: any) =>
            r.kind.Vocabulary?.vocabulary === v &&
            r.kind.Vocabulary.system_unknown,
        ).id;
      await post('catalog', {
        action: {
          Create: {
            id: part,
            kind: 'Part',
            details: {
              manufacturer_id: unknown('Manufacturer'),
              number: name,
              name,
            },
          },
        },
      });
      await post('inventory', {
        body: {
          Create: {
            holding_id: holding,
            pieces: { Known: 100 },
            weight_mg: 'Unknown',
          },
        },
      });
      await post('profiles', {
        target: { Holding: holding },
        expected_revision: 0,
        links: {
          definition_id: part,
          color_id: unknown('Color'),
          condition_id: unknown('Condition'),
          location_id: null,
        },
      });
    },
    { part, holding, name },
  );
  await page.reload();
  await page.getByRole('button', { name: 'Projects', exact: true }).click();
  await page.getByLabel('Audit reason').fill('Reviewing a future MOC build');
  await page.getByLabel('Version label').fill(name + ' v1');
  await page
    .getByRole('combobox', { name: 'Part', exact: true })
    .selectOption(part);
  await page
    .getByRole('combobox', { name: 'Color', exact: true })
    .selectOption({ label: 'Unknown' });
  await page.getByLabel('Required pieces').fill('60');
  await page
    .getByRole('button', { name: 'Add requirement', exact: true })
    .click();
  await page.getByRole('button', { name: 'Save BOM version' }).click();
  await expect(
    page.getByRole('combobox', { name: 'BOM version', exact: true }),
  ).not.toHaveValue('');
  await page.getByLabel('Name', { exact: true }).fill(name);
  await page.getByRole('button', { name: 'Create planned project' }).click();
  await expect(
    page.getByText(/Progress: Planned · Readiness: Planned/),
  ).toBeVisible();
  const option = page
    .getByRole('combobox', { name: 'Requirement', exact: true })
    .locator('option')
    .nth(1);
  await page
    .getByRole('combobox', { name: 'Requirement', exact: true })
    .selectOption((await option.getAttribute('value'))!);
  await page
    .getByRole('combobox', { name: 'Loose holding', exact: true })
    .selectOption(holding);
  await page.getByLabel('Pieces', { exact: true }).fill('60');
  await page.getByRole('button', { name: 'Commit stock claim' }).click();
  await expect(page.getByText(/Readiness: PotentiallyReady/)).toBeVisible();
  await page
    .getByRole('button', { name: 'Confirm physically checked' })
    .click();
  await expect(page.getByText(/Readiness: ReadyToBuild/)).toBeVisible();
  await page
    .getByRole('button', { name: 'Start build — convert reservations' })
    .click();
  await expect(page.getByText(/Progress: InProgress/)).toBeVisible();
  const download = page.waitForEvent('download');
  await page.getByRole('button', { name: 'Download procurement CSV' }).click();
  expect((await download).suggestedFilename()).toMatch(/\.csv$/);
  await page.reload();
  await page.getByRole('button', { name: 'Projects', exact: true }).click();
  await page
    .getByRole('combobox', { name: 'Project', exact: true })
    .selectOption({ label: name });
  await expect(page.getByText(/Progress: InProgress/)).toBeVisible();
  await page.screenshot({
    path: info.outputPath('projects.png'),
    fullPage: true,
  });
  await page.setViewportSize({ width: 390, height: 844 });
  expect(
    await page.evaluate(
      () => document.documentElement.scrollWidth <= window.innerWidth + 1,
    ),
  ).toBe(true);
  await page.screenshot({
    path: info.outputPath('projects-mobile.png'),
    fullPage: true,
  });
  expect(failures).toEqual([]);
  await expect(page.getByRole('alert')).toHaveCount(0);
});
