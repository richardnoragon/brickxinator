import { test, expect } from '@playwright/test';

test.beforeEach(async ({ page }) => {
  await page.route('**/*', (route) =>
    new URL(route.request().url()).hostname === '127.0.0.1'
      ? route.continue()
      : route.abort(),
  );
});

test('draft, receipt, search, costs and disabled backup defaults', async ({
  page,
}, info) => {
  const failures: string[] = [];
  page.on('pageerror', (error) => failures.push(error.message));
  await page.goto('/');
  await expect(
    page.getByRole('heading', { name: 'Dashboard', exact: true }),
  ).toBeVisible();
  const seller = `Browser acceptance ${info.project.name} ${Date.now()}`;
  await page.getByRole('button', { name: '＋ New purchase' }).click();
  await page.getByLabel('Seller', { exact: true }).fill(seller);
  await page.getByLabel('Purchase date', { exact: true }).fill('2026-11-01');
  await page.getByLabel('Seller country', { exact: true }).fill('Germany');
  await page.getByRole('button', { name: '＋ Add invoice line' }).click();
  await page.getByLabel('Description', { exact: true }).fill(seller);
  await page.getByLabel('Brick count', { exact: true }).fill('12');
  await page.getByLabel('Paid line total · EUR').fill('4.80');
  await page.getByRole('button', { name: 'Save draft / changes' }).click();
  await expect(
    page.getByRole('button', { name: 'Receive whole purchase' }),
  ).toBeVisible();
  await page.getByLabel('Choose images or PDFs').setInputFiles({
    name: 'invoice.pdf',
    mimeType: 'application/pdf',
    buffer: Buffer.from('%PDF-1.4 browser fixture'),
  });
  await expect(
    page.getByRole('link', { name: 'invoice.pdf', exact: true }),
  ).toBeVisible();
  await expect(page.locator('main')).toHaveAttribute('aria-busy', 'false');
  await page.evaluate(async () => {
    const canvas = document.createElement('canvas');
    canvas.width = canvas.height = 1;
    canvas.getContext('2d')!.fillRect(0, 0, 1, 1);
    const blob = await new Promise<Blob>((resolve) =>
      canvas.toBlob((blob) => resolve(blob!), 'image/png'),
    );
    const transfer = new DataTransfer();
    transfer.items.add(
      new File([blob], 'pasted-image.png', { type: 'image/png' }),
    );
    const event = new ClipboardEvent('paste', {
      bubbles: true,
      cancelable: true,
    });
    // Firefox discards the constructor's synthetic clipboard files. Supply the
    // same FileList a trusted native paste delivers, without touching OS clipboard.
    Object.defineProperty(event, 'clipboardData', { value: transfer });
    window.dispatchEvent(event);
  });
  await expect(
    page.getByRole('img', { name: 'pasted-image.png', exact: true }),
  ).toBeVisible();
  await page.getByRole('button', { name: 'Receive whole purchase' }).click();
  await expect(
    page.getByText('Received once.', { exact: false }),
  ).toBeVisible();
  await page.getByRole('button', { name: 'Shipping & costs' }).click();
  await expect(
    page.getByRole('heading', { name: 'Shipping allocation & costs' }),
  ).toBeVisible();
  await expect(
    page.getByRole('cell', { name: '0.4000', exact: true }),
  ).toBeVisible();
  await page
    .getByRole('button', { name: 'Archive purchase', exact: true })
    .click();
  await expect(
    page.getByRole('button', { name: 'Unarchive purchase', exact: true }),
  ).toBeVisible();
  await page
    .getByRole('button', { name: 'Unarchive purchase', exact: true })
    .click();
  await page.getByRole('button', { name: 'Inventory', exact: true }).click();
  await expect(page.getByText(seller, { exact: true })).toBeVisible();
  await page.getByRole('button', { name: 'Recovery', exact: true }).click();
  await page
    .getByRole('button', { name: 'Load backup settings and status' })
    .click();
  await expect(page.getByLabel('Daily repository folder')).toHaveValue(
    /backups\/daily$/,
  );
  await expect(page.getByLabel('Monthly ZIP folder')).toHaveValue(
    /backups\/montly$/,
  );
  await expect(page.getByLabel('Enable this destination')).not.toBeChecked();
  expect(failures).toEqual([]);
});

test('responsive dashboard and recovery stay within the viewport', async ({
  page,
}, info) => {
  await page.goto('/');
  await expect(
    page.getByRole('heading', { name: 'Dashboard', exact: true }),
  ).toBeVisible();
  await page.screenshot({
    path: info.outputPath('desktop.png'),
    fullPage: true,
  });
  await page.setViewportSize({ width: 390, height: 844 });
  await expect
    .poll(() =>
      page.evaluate(
        () => document.documentElement.scrollWidth <= window.innerWidth,
      ),
    )
    .toBe(true);
  await page.screenshot({
    path: info.outputPath('mobile.png'),
    fullPage: true,
  });
  await page.getByRole('button', { name: 'Recovery', exact: true }).click();
  await expect(
    page.getByRole('heading', {
      name: 'Portable collection exchange',
      exact: true,
    }),
  ).toBeVisible();
  await expect
    .poll(() =>
      page.evaluate(
        () => document.documentElement.scrollWidth <= window.innerWidth,
      ),
    )
    .toBe(true);
});

test('set verification is absolute, assembly gates stock and copy labels persist', async ({
  page,
}, info) => {
  const title = `Set acceptance ${info.project.name} ${Date.now()}`;
  await page.goto('/');
  await page.getByRole('button', { name: '＋ New purchase' }).click();
  await page.getByRole('button', { name: '＋ Add invoice line' }).click();
  await page
    .getByRole('combobox', { name: 'Kind', exact: true })
    .selectOption('Set');
  await page.getByLabel('Description', { exact: true }).fill(title);
  await page.getByLabel('Set number', { exact: true }).fill(title);
  await page.getByLabel('Physical copies', { exact: true }).fill('1');
  await page.getByLabel('Brick count', { exact: true }).fill('451');
  await page
    .getByRole('combobox', { name: 'Assembly', exact: true })
    .selectOption('Assembled');
  await page.getByRole('button', { name: 'Save draft / changes' }).click();
  await page.getByRole('button', { name: 'Receive whole purchase' }).click();
  await expect(
    page.getByText('Received once.', { exact: false }),
  ).toBeVisible();
  await page.getByRole('button', { name: 'Sets', exact: true }).click();
  await page.getByRole('button').filter({ hasText: title }).click();
  await page.getByLabel('Expected pieces', { exact: true }).fill('7');
  await page
    .getByRole('button', { name: 'Add expected line', exact: true })
    .click();
  await expect(
    page.getByLabel('Verified total', { exact: true }),
  ).toBeVisible();
  await page.getByLabel('Verified total', { exact: true }).fill('7');
  await page
    .getByLabel('Count reason', { exact: true })
    .fill('Initial physical count');
  await page.getByRole('button', { name: 'Record count', exact: true }).click();
  await page.getByRole('button', { name: 'Finish line', exact: true }).click();
  await page
    .getByRole('button', { name: 'Complete verification', exact: true })
    .click();
  await expect(page.getByText('Completed', { exact: true })).toBeVisible();
  await page
    .getByRole('combobox', { name: 'Whole-set assembly', exact: true })
    .selectOption('Loose');
  await page.getByLabel('Copy label', { exact: true }).fill(`${title} renamed`);
  await page.getByRole('button', { name: 'Rename copy', exact: true }).click();
  await expect(
    page.getByRole('button').filter({ hasText: `${title} renamed` }),
  ).toBeVisible();
  await page
    .getByRole('button', { name: 'Retire checklist line', exact: true })
    .click();
  await expect(page.getByLabel('Verified total', { exact: true })).toHaveCount(
    0,
  );
  await page.getByLabel('Show retired checklist lines').check();
  await expect(
    page.getByText('Retired; stock retained', { exact: true }),
  ).toBeVisible();
  await page
    .getByRole('button', { name: 'Restore checklist line', exact: true })
    .click();
  await page.getByRole('button', { name: 'Inventory', exact: true }).click();
  await page.getByLabel('Number or text').fill(title);
  await page.getByRole('button', { name: 'Search', exact: true }).click();
  const contents = page
    .getByRole('row')
    .filter({ hasText: 'Verified set contents' });
  await expect(contents).toHaveCount(1);
  await contents.getByRole('button', { name: 'Adjust', exact: true }).click();
  await page
    .getByRole('combobox', { name: 'Action', exact: true })
    .selectOption('Lost');
  await page.getByLabel('Removed pieces', { exact: true }).fill('2');
  await page
    .getByLabel('Reason', { exact: true })
    .fill('Two pieces lost after verification');
  await page
    .getByRole('button', { name: 'Record stock adjustment', exact: true })
    .click();
  await page.getByRole('button', { name: 'Sets', exact: true }).click();
  await page
    .getByRole('button')
    .filter({ hasText: `${title} renamed` })
    .click();
  await page.getByLabel('Verified total', { exact: true }).fill('7');
  await page
    .getByLabel('Count reason', { exact: true })
    .fill('Fresh physical recount found seven');
  await page.getByRole('button', { name: 'Record count', exact: true }).click();
  await expect(
    page.getByRole('heading', { name: 'Review fresh physical recount' }),
  ).toBeVisible();
  await page
    .getByRole('button', {
      name: 'Apply reviewed physical recount',
      exact: true,
    })
    .click();
  await expect(
    page.getByRole('heading', { name: 'Review fresh physical recount' }),
  ).toHaveCount(0);
  await page.reload();
  await page.getByRole('button', { name: 'Sets', exact: true }).click();
  await expect(
    page.getByRole('button').filter({ hasText: `${title} renamed` }),
  ).toBeVisible();
  await expect(page.getByRole('alert')).toHaveCount(0);
});

test('mixed-lot classification, reviewed correction and portable exchange preserve totals', async ({
  page,
}, info) => {
  const title = `Lot acceptance ${info.project.name} ${Date.now()}`;
  await page.goto('/');
  await page.getByRole('button', { name: '＋ New purchase' }).click();
  await page.getByRole('button', { name: '＋ Add invoice line' }).click();
  await page
    .getByRole('combobox', { name: 'Kind', exact: true })
    .selectOption('MixedLot');
  await page.getByLabel('Description', { exact: true }).fill(title);
  await page.getByLabel('Brick count', { exact: true }).fill('100');
  await page.getByRole('button', { name: 'Save draft / changes' }).click();
  await page.getByRole('button', { name: 'Receive whole purchase' }).click();
  await expect(
    page.getByText('Received once.', { exact: false }),
  ).toBeVisible();
  await page.getByRole('button', { name: 'Inventory', exact: true }).click();
  await page.getByLabel('Number or text').fill(title);
  await page.getByRole('button', { name: 'Search', exact: true }).click();
  let lot = page
    .getByRole('row')
    .filter({ hasText: title })
    .filter({ hasText: 'Mixed lot' });
  await lot.getByRole('button', { name: 'Manage', exact: true }).click();
  await page.getByLabel('Identified pieces', { exact: true }).fill('40');
  await page
    .getByRole('button', { name: 'Extract identified child', exact: true })
    .click();
  await expect(
    page.getByText('100 known pieces across all matches', { exact: true }),
  ).toBeVisible();
  lot = page
    .getByRole('row')
    .filter({ hasText: title })
    .filter({ hasText: 'Mixed lot' });
  await lot
    .getByRole('button', { name: 'View source purchase', exact: true })
    .click();
  await page
    .getByRole('button', { name: 'Correct received purchase', exact: true })
    .click();
  await page.getByLabel('Brick count', { exact: true }).fill('30');
  await page
    .getByRole('button', { name: 'Preview correction', exact: true })
    .click();
  await expect(page.getByRole('alert')).toContainText(
    'below already transferred',
  );
  await page.getByLabel('Brick count', { exact: true }).fill('90');
  await page
    .getByRole('button', { name: 'Preview correction', exact: true })
    .click();
  await expect(
    page.getByText('Residual holding 1:', { exact: false }),
  ).toContainText('60 → 50 pieces');
  await page
    .getByLabel('Correction reason', { exact: true })
    .fill('Invoice originally transcribed as 100 instead of 90');
  await page
    .getByRole('button', { name: 'Apply reviewed correction', exact: true })
    .click();
  await expect(
    page.getByRole('heading', { name: 'Review received-purchase correction' }),
  ).toHaveCount(0);
  await page.getByRole('button', { name: 'Inventory', exact: true }).click();
  await expect(
    page.getByText('90 known pieces across all matches', { exact: true }),
  ).toBeVisible();
  await page.getByRole('button', { name: 'Recovery', exact: true }).click();
  const download = page.waitForEvent('download');
  await page
    .getByRole('button', { name: 'Download native JSON bundle', exact: true })
    .click();
  const bundle = await download;
  expect(await bundle.failure()).toBeNull();
  const path = await bundle.path();
  expect(path).not.toBeNull();
  await page
    .getByLabel(
      'Initialize this empty collection from the selected native bundle',
    )
    .check();
  await page
    .getByLabel('Native bundle ZIP', { exact: true })
    .setInputFiles(path!);
  await expect(
    page.getByLabel(
      'Initialize this empty collection from the selected native bundle',
    ),
  ).not.toBeChecked();
  await expect(page.getByRole('alert')).toHaveCount(0);
  await page.getByRole('button', { name: 'Inventory', exact: true }).click();
  await expect(
    page.getByText('90 known pieces across all matches', { exact: true }),
  ).toBeVisible();
});
