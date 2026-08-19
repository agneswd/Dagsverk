import fs from 'node:fs';
import path from 'node:path';
import puppeteer, { type Browser, type Page } from 'puppeteer-core';
import { ThemePreference } from '../src/app/core/models';
import { installVisualFixture, waitForVisualApp } from './browser-visual-harness';

const outputArgument = process.argv.indexOf('--output');
const screenshotDir = path.resolve(
  outputArgument >= 0 && process.argv[outputArgument + 1]
    ? process.argv[outputArgument + 1]
    : path.join('reference', 'screenshots', 'electron'),
);
const baseUrl = process.env.DAGSVERK_CAPTURE_URL ?? 'http://127.0.0.1:4200';

type CaptureOptions = {
  name: string;
  route?: string;
  ready?: string;
  theme?: ThemePreference;
  scale?: number;
  width?: number;
  height?: number;
  prepare?: (page: Page) => Promise<void>;
};

async function clickText(page: Page, selector: string, text: string): Promise<void> {
  const clicked = await page.evaluate(
    (query, expected) => {
      const normalizedExpected = expected.replace(/\s+/g, ' ').trim();
      const element = [...document.querySelectorAll<HTMLElement>(query)].find((item) =>
        item.textContent?.replace(/\s+/g, ' ').trim().includes(normalizedExpected),
      );
      element?.click();
      return Boolean(element);
    },
    selector,
    text,
  );
  if (!clicked) throw new Error(`Could not click "${text}" in ${selector}`);
}

async function openRoute(page: Page, route: string, ready: string): Promise<void> {
  await page.goto(`${baseUrl}/`, { waitUntil: 'networkidle0' });
  if (route !== '/timesheet') {
    await page.evaluate((nextRoute) => {
      history.pushState(null, '', nextRoute);
      dispatchEvent(new PopStateEvent('popstate'));
    }, route);
  }
  await page.waitForSelector(ready);
  await page.evaluate(async () => document.fonts.ready);
}

async function openDayEditor(page: Page): Promise<void> {
  const rows = await page.$$('.m3-ledger-table .ledger-data-row');
  if (rows.length < 4) throw new Error('The visual fixture did not render four ledger rows.');
  await rows[3].click();
  await page.waitForSelector('.m3-day-editor');
  if (!(await page.$('.pay-summary-card'))) {
    await page.click('.m3-status-toggle mat-button-toggle:first-child button');
    await page.waitForSelector('.pay-summary-card');
  }
}

async function captureScreen(browser: Browser, options: CaptureOptions): Promise<void> {
  const page = await browser.newPage();
  try {
    await page.setViewport({
      width: options.width ?? 1366,
      height: options.height ?? 820,
      deviceScaleFactor: 1,
    });
    await installVisualFixture(page, options.theme ?? ThemePreference.Light, options.scale ?? 100);
    const route = options.route ?? '/timesheet';
    await openRoute(page, route, options.ready ?? '.m3-ledger-table');
    if (route === '/timesheet') await waitForVisualApp(page);
    await options.prepare?.(page);
    await page.evaluate(async () => document.fonts.ready);
    await page.screenshot({ path: path.join(screenshotDir, options.name) });
    console.log(`Captured ${options.name}`);
  } finally {
    await page.close();
  }
}

async function capture() {
  fs.mkdirSync(screenshotDir, { recursive: true });
  const browser = await puppeteer.launch({
    executablePath: process.env.CHROMIUM_PATH ?? '/usr/bin/chromium',
    headless: true,
    args: ['--no-sandbox', '--disable-setuid-sandbox', '--disable-gpu'],
  });

  try {
    const captures: CaptureOptions[] = [
      { name: '01_ledger_light.png' },
      { name: '02_day_editor_light.png', prepare: openDayEditor },
      {
        name: '03_calendar_light.png',
        prepare: async (page) => {
          await page.click('.m3-view-toggle mat-button-toggle:nth-child(2) button');
          await page.waitForSelector('.m3-calendar-container');
        },
      },
      {
        name: '04_projects_light.png',
        route: '/projects',
        ready: '.projects-page-container',
      },
      {
        name: '05_settings_general_light.png',
        route: '/settings',
        ready: '.settings-page-container',
      },
      {
        name: '06_settings_overtime_light.png',
        route: '/settings',
        ready: '.settings-page-container',
        prepare: (page) => clickText(page, '.mat-mdc-tab', 'Overtime and OB'),
      },
      {
        name: '07_backups_light.png',
        route: '/settings/data-backups',
        ready: '.backups-page-container',
      },
      {
        name: '08_workspace_dialog_light.png',
        prepare: async (page) => {
          await page.click('.workspace-switcher-btn');
          await page.waitForSelector('.mat-mdc-menu-panel');
          await clickText(page, '.mat-mdc-menu-panel button', 'Manage workspaces');
          await page.waitForSelector('.workspace-manager-dialog .mat-mdc-dialog-surface');
        },
      },
      {
        name: '09_month_menu_light.png',
        prepare: async (page) => {
          await page.click('.month-selector-btn');
          await page.waitForSelector('.mat-mdc-menu-panel');
        },
      },
      {
        name: '10_color_picker_light.png',
        route: '/projects',
        ready: '.projects-page-container',
        prepare: async (page) => {
          await page.click('.color-trigger');
          await page.waitForSelector('.m3-color-menu');
        },
      },
      { name: '11_ledger_dark.png', theme: ThemePreference.Dark },
      {
        name: '12_calendar_dark.png',
        theme: ThemePreference.Dark,
        prepare: async (page) => {
          await page.click('.m3-view-toggle mat-button-toggle:nth-child(2) button');
          await page.waitForSelector('.m3-calendar-container');
        },
      },
      { name: '13_day_editor_dark.png', theme: ThemePreference.Dark, prepare: openDayEditor },
      {
        name: '14_settings_dark.png',
        route: '/settings',
        ready: '.settings-page-container',
        theme: ThemePreference.Dark,
      },
      {
        name: '15_workspace_dialog_dark.png',
        theme: ThemePreference.Dark,
        prepare: async (page) => {
          await page.click('.workspace-switcher-btn');
          await page.waitForSelector('.mat-mdc-menu-panel');
          await clickText(page, '.mat-mdc-menu-panel button', 'Manage workspaces');
          await page.waitForSelector('.workspace-manager-dialog .mat-mdc-dialog-surface');
        },
      },
      {
        name: '16_month_actions_light.png',
        prepare: async (page) => {
          await page.click('[aria-label="Month actions"]');
          await page.waitForSelector('.mat-mdc-menu-panel');
        },
      },
      {
        name: '17_workspace_menu_light.png',
        prepare: async (page) => {
          await page.click('.workspace-switcher-btn');
          await page.waitForSelector('.mat-mdc-menu-panel');
        },
      },
      {
        name: '18_select_panel_light.png',
        route: '/settings',
        ready: '.settings-page-container',
        prepare: async (page) => {
          await page.click('.mat-mdc-select-trigger');
          await page.waitForSelector('.mat-mdc-select-panel');
        },
      },
      {
        name: '19_confirmation_dialog_light.png',
        route: '/projects',
        ready: '.projects-page-container',
        prepare: async (page) => {
          await page.click('[aria-label^="Delete "]');
          await page.waitForSelector('.mat-mdc-dialog-surface');
        },
      },
      { name: '21_ledger_960x640.png', width: 960, height: 640 },
      { name: '22_ledger_1200x760.png', width: 1200, height: 760 },
      {
        name: '23_day_editor_1600x900.png',
        width: 1600,
        height: 900,
        prepare: openDayEditor,
      },
      { name: '24_ledger_scale_80.png', scale: 80 },
      { name: '25_ledger_scale_125.png', scale: 125 },
      { name: '26_ledger_scale_150.png', scale: 150 },
    ];

    for (const options of captures) await captureScreen(browser, options);

    await captureScreen(browser, {
      name: '20_snackbar_light.png',
      prepare: async (page) => {
        await page.click('.workspace-switcher-btn');
        await page.waitForSelector('.mat-mdc-menu-panel');
        await clickText(page, '.mat-mdc-menu-panel button', 'Manage workspaces');
        await page.waitForSelector('.workspace-manager-dialog .mat-mdc-dialog-surface');
        await page.click('.mat-mdc-dialog-surface .add-ws-btn');
        await page.waitForSelector('.mat-mdc-dialog-surface input');
        await page.type('.mat-mdc-dialog-surface input', 'Visual workspace');
        await page.click('.mat-mdc-dialog-surface .save-btn');
        await page.waitForSelector('.mat-mdc-snack-bar-container');
      },
    });
  } finally {
    await browser.close();
  }
}

capture().catch((error) => {
  console.error(error);
  process.exitCode = 1;
});
