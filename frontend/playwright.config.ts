import { defineConfig } from '@playwright/test';
import { mkdtempSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
const dataDirectory = mkdtempSync(join(tmpdir(), 'brickxinator-browser-'));
export default defineConfig({
  testDir: './tests',
  fullyParallel: false,
  workers: 1,
  use: {
    baseURL: process.env.BRICKXINATOR_TEST_URL ?? 'http://127.0.0.1:43179',
    headless: true,
  },
  webServer: process.env.BRICKXINATOR_TEST_URL
    ? undefined
    : {
        command: `../target/debug/brickxinator-app serve --data-dir '${dataDirectory}' --port 43179 --idle-seconds 600`,
        url: 'http://127.0.0.1:43179',
        reuseExistingServer: false,
      },
  projects: [
    { name: 'chromium', use: { browserName: 'chromium' } },
    { name: 'firefox', use: { browserName: 'firefox' } },
  ],
});
