// E2E check for first start, new profiles, and the Profile and Settings pages.
// Start `bun run start` first, then run `bun scripts/e2e-profiles.ts`.
// Screenshots go to artifacts/e2e/profiles/.
import puppeteer, { type ElementHandle, type Page } from 'puppeteer-core';
import * as fs from 'fs';
import * as path from 'path';

const baseUrl = process.env['DAGSVERK_URL'] ?? 'http://localhost:4200';
const outDir = path.join(__dirname, '..', 'artifacts', 'e2e', 'profiles');

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
    await page.goto(baseUrl);
    await page.evaluate(() => {
      localStorage.clear();
      localStorage.setItem('dagsverk_preferences', JSON.stringify({ languagePreference: 1 }));
    });
    await page.goto(baseUrl, { waitUntil: 'networkidle0' });

    // First start: the welcome form makes the first profile.
    await page.waitForSelector('app-profile-form-dialog');
    await settle();
    await page.screenshot({ path: path.join(outDir, '01-welcome.png') });
    await typeInto(await field(page, 'Profile name'), 'Office');
    await typeInto(await field(page, 'Employer or client'), 'Acme AB');
    await typeInto(await field(page, 'Your name'), 'Elias');
    await clickButton(page, 'Get started');
    await page.waitForSelector('app-profile-form-dialog', { hidden: true });
    expect(
      (await text(page, '.ws-name')) === 'Office' &&
        (await text(page, '.ws-employer')) === 'Acme AB',
      'sidebar shows the first profile and employer',
    );
    expect(
      Boolean((await text(page, 'app-summary-cards'))?.includes('Time balance')),
      'fixed-schedule profile shows the time balance',
    );

    // A second profile with no fixed schedule, like a café job with random shifts.
    await page.click('.workspace-switcher-btn');
    await clickButton(page, 'New profile');
    await page.waitForSelector('app-profile-form-dialog');
    await settle();
    await typeInto(await field(page, 'Profile name'), 'Hamnkafe');
    await typeInto(await field(page, 'Hourly rate'), '134.5');
    await clickButton(page, 'No fixed schedule');
    await page.screenshot({ path: path.join(outDir, '02-new-profile.png') });
    await clickButton(page, 'Create profile');
    await page.waitForSelector('app-profile-form-dialog', { hidden: true });
    await settle();
    expect((await text(page, '.ws-name')) === 'Hamnkafe', 'new profile becomes active');
    const summary = (await text(page, 'app-summary-cards')) ?? '';
    expect(
      !summary.includes('Time balance') && !summary.includes('/ '),
      `no-schedule profile hides expected time and balance, got "${summary}"`,
    );
    await page.screenshot({ path: path.join(outDir, '03-timesheet-no-schedule.png') });

    // The Profile page holds this profile's settings, including the details set above.
    await clickNav(page, 'Profile');
    await page.waitForSelector('app-profile-settings');
    await settle();
    expect(
      (await inputValue(await field(page, 'Profile name'))) === 'Hamnkafe',
      'profile page edits the active profile',
    );
    await typeInto(await field(page, 'Employer or client'), 'Måns Hamnkafé & Kök');
    await clickButton(page, 'Save changes');
    await settle();
    expect(
      (await text(page, '.ws-employer')) === 'Måns Hamnkafé & Kök',
      'saving the profile updates the employer in the sidebar',
    );
    await page.screenshot({ path: path.join(outDir, '04-profile-details.png') });
    await clickTab(page, 'Schedule');
    expect(
      Boolean(
        (await text(page, '.schedule-toggle .mat-button-toggle-checked'))?.includes(
          'No fixed schedule',
        ),
      ),
      'schedule tab shows no fixed schedule',
    );
    await page.screenshot({ path: path.join(outDir, '05-profile-schedule.png') });
    await clickTab(page, 'Pay and tax');
    expect(
      (await inputValue(await field(page, 'Hourly rate'))) === '134.5',
      'pay tab shows the hourly rate from the form',
    );

    // Settings only holds app-wide items.
    await clickNav(page, 'Settings');
    await page.waitForSelector('app-settings');
    await settle();
    const settings = (await text(page, 'app-settings')) ?? '';
    expect(
      settings.includes('Appearance and language') &&
        settings.includes('Backup and restore') &&
        !settings.includes('Hourly rate') &&
        !settings.includes('Workdays'),
      'settings page has app-wide items only',
    );
    await page.screenshot({ path: path.join(outDir, '06-settings.png'), fullPage: true });

    console.log(`PASS profiles. Screenshots: ${outDir}`);
  } finally {
    await browser.close();
  }
}

function expect(condition: boolean, message: string): void {
  if (!condition) throw new Error(`FAIL ${message}`);
  console.log(`ok ${message}`);
}

const settle = () => new Promise((resolve) => setTimeout(resolve, 600));

async function text(page: Page, selector: string): Promise<string | undefined> {
  return page.$eval(selector, (el) => el.textContent?.replace(/\s+/g, ' ').trim());
}

// Finds the input of the visible form field with this label.
async function field(page: Page, label: string): Promise<ElementHandle<HTMLInputElement>> {
  const handle = await page.evaluateHandle((wanted) => {
    const fields = [...document.querySelectorAll('mat-form-field')].filter(
      (el) => (el as HTMLElement).offsetParent !== null,
    );
    const match = fields.find(
      (el) => el.querySelector('mat-label')?.textContent?.trim() === wanted,
    );
    return match?.querySelector('input') ?? null;
  }, label);
  const input = handle.asElement() as ElementHandle<HTMLInputElement> | null;
  if (!input) throw new Error(`No field "${label}"`);
  return input;
}

async function inputValue(input: ElementHandle<HTMLInputElement>): Promise<string> {
  return input.evaluate((el) => el.value);
}

async function typeInto(input: ElementHandle<HTMLInputElement>, value: string): Promise<void> {
  await input.click({ count: 3 });
  await input.press('Backspace');
  await input.type(value);
  await settle();
}

async function clickButton(page: Page, label: string): Promise<void> {
  const clicked = await page.evaluate((wanted) => {
    const button = [...document.querySelectorAll('button')].find(
      (el) =>
        el.offsetParent !== null && el.textContent?.trim().replace(/\s+/g, ' ').endsWith(wanted),
    );
    button?.click();
    return Boolean(button);
  }, label);
  if (!clicked) throw new Error(`No button "${label}"`);
  await settle();
}

async function clickNav(page: Page, label: string): Promise<void> {
  await page.evaluate((wanted) => {
    [...document.querySelectorAll<HTMLElement>('app-sidebar a')]
      .find((el) => el.textContent?.trim().endsWith(wanted))
      ?.click();
  }, label);
  await settle();
}

async function clickTab(page: Page, label: string): Promise<void> {
  await page.evaluate((wanted) => {
    [...document.querySelectorAll<HTMLElement>('[role="tab"]')]
      .find((el) => el.textContent?.trim() === wanted)
      ?.click();
  }, label);
  await settle();
}

main().catch((error) => {
  console.error(error);
  process.exit(1);
});
