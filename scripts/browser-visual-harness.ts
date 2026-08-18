import type { Page } from 'puppeteer-core';
import { ThemePreference } from '../src/app/core/models';
import { browserStorageEntries, VISUAL_NOW } from './visual-fixture-data';

export async function installVisualFixture(
  page: Page,
  theme = ThemePreference.Light,
  scale = 100,
): Promise<void> {
  const storage = browserStorageEntries(theme, scale).map(([key, value]) => [
    key,
    JSON.stringify(value),
  ]);
  await page.evaluateOnNewDocument(`
    (() => {
      const fixedNow = ${JSON.stringify(VISUAL_NOW)};
      const NativeDate = Date;
      const FixedDate = class extends NativeDate {
        constructor(...args) {
          super(...(args.length ? args : [fixedNow]));
        }
        static now() {
          return new NativeDate(fixedNow).valueOf();
        }
      };
      Object.defineProperty(globalThis, 'Date', { configurable: true, value: FixedDate });
      localStorage.clear();
      for (const [key, value] of ${JSON.stringify(storage)}) localStorage.setItem(key, value);
    })();
  `);
}

export async function waitForVisualApp(page: Page): Promise<void> {
  await page.waitForSelector('.app-shell-container');
  await page.waitForSelector('.m3-ledger-table .ledger-data-row');
  await page.evaluate(async () => document.fonts.ready);
}
