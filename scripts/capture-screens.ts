import puppeteer from 'puppeteer-core';
import * as path from 'path';
import * as fs from 'fs';

const outputArgument = process.argv.indexOf('--output');
const screenshotDir = path.resolve(
  outputArgument >= 0 && process.argv[outputArgument + 1]
    ? process.argv[outputArgument + 1]
    : path.join('reference', 'screenshots', 'electron'),
);
const baseUrl = process.env['DAGSVERK_CAPTURE_URL'] || 'http://localhost:4200';
const chromiumPath = process.env['CHROMIUM_PATH'] || '/usr/bin/chromium';
fs.mkdirSync(screenshotDir, { recursive: true });

async function capture() {
  const browser = await puppeteer.launch({
    executablePath: chromiumPath,
    headless: true,
    args: ['--no-sandbox', '--disable-setuid-sandbox', '--disable-gpu'],
  });

  const page = await browser.newPage();
  await page.setViewport({ width: 1366, height: 820, deviceScaleFactor: 1 });

  console.log(`Navigating to ${baseUrl} ...`);
  await page.goto(baseUrl, { waitUntil: 'networkidle0' });
  await new Promise((r) => setTimeout(r, 800));

  // Ensure Light theme
  await page.evaluate(() => {
    document.body.classList.remove('dark-theme');
    document.documentElement.classList.remove('dark-theme');
  });
  await new Promise((r) => setTimeout(r, 300));

  // 1. Ledger View (Light)
  await page.screenshot({ path: path.join(screenshotDir, '01_ledger_light.png') });
  console.log('✔ Captured 01_ledger_light.png');

  // 2. Open Day Editor (Light)
  const editBtn = await page.$('.row-edit-btn');
  if (editBtn) {
    await editBtn.click();
    await new Promise((r) => setTimeout(r, 500));
    await page.screenshot({ path: path.join(screenshotDir, '02_day_editor_light.png') });
    console.log('✔ Captured 02_day_editor_light.png');

    const closeBtn = await page.$('.close-btn');
    if (closeBtn) {
      await closeBtn.click();
      await new Promise((r) => setTimeout(r, 300));
    }
  }

  // 3. Calendar View (Light)
  const calendarToggle = await page.$('.m3-view-toggle mat-button-toggle:nth-child(2) button');
  if (calendarToggle) {
    await calendarToggle.click();
    await new Promise((r) => setTimeout(r, 400));
    await page.screenshot({ path: path.join(screenshotDir, '03_calendar_light.png') });
    console.log('✔ Captured 03_calendar_light.png');
  }

  // Switch back to Ledger view
  const ledgerToggle = await page.$('.m3-view-toggle mat-button-toggle:nth-child(1) button');
  if (ledgerToggle) {
    await ledgerToggle.click();
    await new Promise((r) => setTimeout(r, 400));
  }

  // 4. Workspaces Page (Light)
  await page.goto(`${baseUrl}/workspaces`, { waitUntil: 'networkidle0' });
  await page.evaluate(() => {
    document.body.classList.remove('dark-theme');
    document.documentElement.classList.remove('dark-theme');
  });
  await new Promise((r) => setTimeout(r, 400));
  await page.screenshot({ path: path.join(screenshotDir, '04_workspaces_light.png') });
  console.log('✔ Captured 04_workspaces_light.png');

  // 5. Projects Page (Light)
  await page.goto(`${baseUrl}/projects`, { waitUntil: 'networkidle0' });
  await page.evaluate(() => {
    document.body.classList.remove('dark-theme');
    document.documentElement.classList.remove('dark-theme');
  });
  await new Promise((r) => setTimeout(r, 400));
  await page.screenshot({ path: path.join(screenshotDir, '05_projects_light.png') });
  console.log('✔ Captured 05_projects_light.png');

  // 6. Settings Page (Light)
  await page.goto(`${baseUrl}/settings`, { waitUntil: 'networkidle0' });
  await page.evaluate(() => {
    document.body.classList.remove('dark-theme');
    document.documentElement.classList.remove('dark-theme');
  });
  await new Promise((r) => setTimeout(r, 400));
  await page.screenshot({ path: path.join(screenshotDir, '06_settings_light.png') });
  console.log('✔ Captured 06_settings_light.png');

  // 7. Backups Page (Light)
  await page.goto(`${baseUrl}/backups`, { waitUntil: 'networkidle0' });
  await page.evaluate(() => {
    document.body.classList.remove('dark-theme');
    document.documentElement.classList.remove('dark-theme');
  });
  await new Promise((r) => setTimeout(r, 400));
  await page.screenshot({ path: path.join(screenshotDir, '07_backups_light.png') });
  console.log('✔ Captured 07_backups_light.png');

  // 8. Populated Timesheet (Light)
  await page.goto(`${baseUrl}/timesheet`, { waitUntil: 'networkidle0' });
  await page.evaluate(() => {
    document.body.classList.remove('dark-theme');
    document.documentElement.classList.remove('dark-theme');
  });
  const catchupBtn = await page.$('.catchup-btn');
  if (catchupBtn) {
    await catchupBtn.click();
    await new Promise((r) => setTimeout(r, 800));
  }
  await page.screenshot({ path: path.join(screenshotDir, '08_populated_ledger_light.png') });
  console.log('✔ Captured 08_populated_ledger_light.png');

  // 9. Populated Timesheet (Dark)
  await page.evaluate(() => {
    document.body.classList.add('dark-theme');
    document.documentElement.classList.add('dark-theme');
  });
  await new Promise((r) => setTimeout(r, 400));
  await page.screenshot({ path: path.join(screenshotDir, '09_populated_ledger_dark.png') });
  console.log('✔ Captured 09_populated_ledger_dark.png');

  // 10. Populated Calendar (Dark)
  const calToggleDark = await page.$('.m3-view-toggle mat-button-toggle:nth-child(2) button');
  if (calToggleDark) {
    await calToggleDark.click();
    await new Promise((r) => setTimeout(r, 400));
    await page.screenshot({ path: path.join(screenshotDir, '10_populated_calendar_dark.png') });
    console.log('✔ Captured 10_populated_calendar_dark.png');
  }

  // 11. Workspaces (Dark)
  await page.goto(`${baseUrl}/workspaces`, { waitUntil: 'networkidle0' });
  await page.evaluate(() => {
    document.body.classList.add('dark-theme');
    document.documentElement.classList.add('dark-theme');
  });
  await new Promise((r) => setTimeout(r, 400));
  await page.screenshot({ path: path.join(screenshotDir, '11_workspaces_dark.png') });
  console.log('✔ Captured 11_workspaces_dark.png');

  // 12. Settings (Dark)
  await page.goto(`${baseUrl}/settings`, { waitUntil: 'networkidle0' });
  await page.evaluate(() => {
    document.body.classList.add('dark-theme');
    document.documentElement.classList.add('dark-theme');
  });
  await new Promise((r) => setTimeout(r, 400));
  await page.screenshot({ path: path.join(screenshotDir, '12_settings_dark.png') });
  console.log('✔ Captured 12_settings_dark.png');

  await browser.close();
  console.log('--- Visual capture complete ---');
}

capture().catch((err) => {
  console.error('Error capturing screenshots:', err);
  process.exit(1);
});
