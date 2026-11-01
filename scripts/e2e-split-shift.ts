// E2E check for split shifts and custom lunch in the browser build.
// Start `bun run start` first, then run `bun scripts/e2e-split-shift.ts`.
// Screenshots go to artifacts/e2e/split-shift/.
import puppeteer, { type Page } from 'puppeteer-core';
import * as fs from 'fs';
import * as path from 'path';

const baseUrl = process.env['DAGSVERK_URL'] ?? 'http://localhost:4200';
const outDir = path.join(__dirname, '..', 'artifacts', 'e2e', 'split-shift');

async function main(): Promise<void> {
  fs.mkdirSync(outDir, { recursive: true });
  const browser = await puppeteer.launch({
    executablePath: process.env['CHROMIUM_PATH'] ?? '/usr/bin/chromium',
    headless: true,
    args: ['--no-sandbox', '--disable-gpu', '--mute-audio'],
  });
  try {
    const page = await browser.newPage();
    await page.setViewport({ width: 1366, height: 860 });

    // Fresh browser storage, setup done, calendar view, English.
    await page.goto(baseUrl);
    await page.evaluate(() => {
      localStorage.clear();
      localStorage.setItem(
        'dagsverk_preferences',
        JSON.stringify({ hasCompletedSetup: true, monthViewPreference: 1, languagePreference: 1 }),
      );
    });
    await page.goto(baseUrl, { waitUntil: 'networkidle0' });

    const date = await page.$eval('.calendar-day-cell:not(.other-month)', (cell) =>
      cell.getAttribute('aria-label'),
    );
    await openEditor(page);
    await clickButton(page, 'Worked');
    await page.waitForSelector('.lunch-input input');

    await setInput(page, '.time-range-grid:nth-of-type(1) input', 0, '12:00');
    await setInput(page, '.time-range-grid:nth-of-type(1) input', 1, '15:00');
    await clickButton(page, 'Add period');
    const rows = await page.$$('.time-range-grid');
    const second = await rows[1].$$('input');
    await replaceText(second[0], '17:00');
    await replaceText(second[1], '21:00');
    await setInput(page, '.lunch-input input', 0, '15');

    const effective = await page.$eval('.pay-summary-card .title-small', (el) => el.textContent);
    expect(effective?.trim() === '6.75h', `editor shows 6.75h, got ${effective}`);
    await page.screenshot({ path: path.join(outDir, '01-editor.png') });

    await clickButton(page, 'Save entry');
    await page.waitForSelector('.m3-day-editor', { hidden: true });

    const chip = await page.$eval('.worked-chip', (el) => el.textContent?.replace(/\s+/g, ' '));
    expect(
      chip?.includes('12:00-15:00') && chip.includes('17:00-21:00') && chip.includes('6.75h'),
      `calendar chip shows both periods and 6.75h, got "${chip}"`,
    );
    await page.screenshot({ path: path.join(outDir, '02-calendar.png') });

    // Reopen: the editor must show both periods and the custom lunch again.
    await openEditor(page);
    const values = await page.$$eval('.time-range-grid input, .lunch-input input', (inputs) =>
      inputs.map((input) => (input as HTMLInputElement).value),
    );
    expect(
      values.join(' ') === '12:00 15:00 17:00 21:00 15',
      `reopened editor keeps periods and lunch, got ${values.join(' ')}`,
    );

    // An overlapping second period must be rejected.
    const reopened = await page.$$('.time-range-grid');
    await replaceText((await reopened[1].$$('input'))[0], '14:00');
    await clickButton(page, 'Save entry');
    const error = await page.$eval('[role="alert"]', (el) => el.textContent);
    expect(Boolean(error?.includes('must start after')), `overlap rejected, got ${error}`);
    await page.screenshot({ path: path.join(outDir, '03-overlap-error.png') });
    await clickButton(page, 'Cancel');

    await clickButton(page, 'Ledger');
    const ledger = await page.$eval('.time-range', (el) => el.textContent?.replace(/\s+/g, ' '));
    expect(
      ledger?.trim() === '12:00-15:00, 17:00-21:00',
      `ledger shows both periods, got "${ledger}"`,
    );
    await page.screenshot({ path: path.join(outDir, '04-ledger.png') });

    console.log(`PASS split shift on ${date}. Screenshots: ${outDir}`);
  } finally {
    await browser.close();
  }
}

// Opens the first day of the month and waits for the side sheet to finish sliding in.
async function openEditor(page: Page): Promise<void> {
  await page.click('.calendar-day-cell:not(.other-month)');
  await page.waitForSelector('.m3-day-editor');
  await new Promise((resolve) => setTimeout(resolve, 700));
}

function expect(condition: boolean | undefined, message: string): void {
  if (!condition) throw new Error(`FAIL ${message}`);
  console.log(`ok ${message}`);
}

async function clickButton(page: Page, label: string): Promise<void> {
  const clicked = await page.evaluate((text) => {
    const button = [...document.querySelectorAll('button')].find((el) =>
      el.textContent?.trim().replace(/\s+/g, ' ').endsWith(text),
    );
    button?.click();
    return Boolean(button);
  }, label);
  if (!clicked) throw new Error(`No button "${label}"`);
  await new Promise((resolve) => setTimeout(resolve, 250));
}

async function setInput(page: Page, selector: string, index: number, value: string): Promise<void> {
  const inputs = await page.$$(selector);
  await replaceText(inputs[index], value);
}

async function replaceText(input: Awaited<ReturnType<Page['$']>>, value: string): Promise<void> {
  if (!input) throw new Error('Input not found');
  await input.click({ count: 3 });
  await input.press('Backspace');
  await input.type(value);
  await input.evaluate((el) => (el as HTMLInputElement).blur());
  await new Promise((resolve) => setTimeout(resolve, 200));
}

main().catch((error) => {
  console.error(error);
  process.exit(1);
});
