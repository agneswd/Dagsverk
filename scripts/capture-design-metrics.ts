import fs from 'node:fs';
import path from 'node:path';
import puppeteer, { type Page } from 'puppeteer-core';
import { installVisualFixture, waitForVisualApp } from './browser-visual-harness';

const root = path.resolve(import.meta.dirname, '..');
const output = path.resolve(
  process.argv[2] ?? path.join(root, 'reference/design-metrics/design.json'),
);
const url = process.env.DAGSVERK_CAPTURE_URL ?? 'http://localhost:4200';

type SelectorMap = Record<string, string>;

async function measure(page: Page, selectors: SelectorMap) {
  const result = await page.evaluate((required) => {
    const values: Record<string, object[]> = {};
    const missing: string[] = [];
    for (const [name, selector] of Object.entries(required)) {
      const elements = [...document.querySelectorAll<HTMLElement>(selector)];
      if (!elements.length) missing.push(`${name}: ${selector}`);
      else
        values[name] = elements.map((element) => {
          const bounds = element.getBoundingClientRect();
          const style = getComputedStyle(element);
          return {
            x: bounds.x,
            y: bounds.y,
            width: bounds.width,
            height: bounds.height,
            paddingTop: style.paddingTop,
            paddingRight: style.paddingRight,
            paddingBottom: style.paddingBottom,
            paddingLeft: style.paddingLeft,
            rowGap: style.rowGap,
            columnGap: style.columnGap,
            borderTopWidth: style.borderTopWidth,
            borderRightWidth: style.borderRightWidth,
            borderBottomWidth: style.borderBottomWidth,
            borderLeftWidth: style.borderLeftWidth,
            borderTopColor: style.borderTopColor,
            borderRightColor: style.borderRightColor,
            borderBottomColor: style.borderBottomColor,
            borderLeftColor: style.borderLeftColor,
            borderRadius: style.borderRadius,
            backgroundColor: style.backgroundColor,
            color: style.color,
            fontFamily: style.fontFamily,
            fontSize: style.fontSize,
            fontWeight: style.fontWeight,
            lineHeight: style.lineHeight,
            letterSpacing: style.letterSpacing,
            opacity: style.opacity,
            boxShadow: style.boxShadow,
            overflow: style.overflow,
            display: style.display,
            gridTemplateColumns: style.gridTemplateColumns,
          };
        });
    }
    return { missing, values };
  }, selectors);
  if (result.missing.length)
    throw new Error(`Missing required selectors:\n${result.missing.join('\n')}`);
  return result.values;
}

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

async function open(page: Page, route: string, ready: string): Promise<void> {
  await page.goto(`${url}/`, { waitUntil: 'networkidle0' });
  if (route !== '/timesheet') {
    await page.evaluate((nextRoute) => {
      history.pushState(null, '', nextRoute);
      dispatchEvent(new PopStateEvent('popstate'));
    }, route);
  }
  await page.waitForSelector(ready);
  await page.evaluate(async () => document.fonts.ready);
}

async function capture() {
  const browser = await puppeteer.launch({
    executablePath: process.env.CHROMIUM_PATH ?? '/usr/bin/chromium',
    headless: true,
    args: ['--no-sandbox', '--disable-setuid-sandbox', '--disable-gpu'],
  });

  try {
    const page = await browser.newPage();
    await page.setViewport({ width: 1366, height: 820, deviceScaleFactor: 1 });
    await installVisualFixture(page);
    await page.goto(`${url}/timesheet`, { waitUntil: 'networkidle0' });
    await waitForVisualApp(page);

    const foundations = await page.evaluate(() => {
      const typographyRoles = [
        'headline-small',
        'title-large',
        'title-medium',
        'title-small',
        'body-large',
        'body-medium',
        'body-small',
        'label-large',
        'label-medium',
        'label-small',
        'numeric',
      ];
      const host = document.createElement('div');
      host.style.position = 'fixed';
      host.style.visibility = 'hidden';
      document.body.append(host);
      const typography = Object.fromEntries(
        typographyRoles.map((role) => {
          const element = document.createElement('span');
          element.className = role;
          element.textContent = 'Dagsverk 0123456789';
          host.append(element);
          const style = getComputedStyle(element);
          return [
            role,
            {
              fontFamily: style.fontFamily,
              fontSize: style.fontSize,
              fontWeight: style.fontWeight,
              lineHeight: style.lineHeight,
              letterSpacing: style.letterSpacing,
              fontVariantNumeric: style.fontVariantNumeric,
            },
          ];
        }),
      );
      host.remove();

      const colorNames = [
        'bg',
        'surface',
        'surface-container-lowest',
        'surface-container-low',
        'surface-container',
        'surface-container-high',
        'surface-container-highest',
        'primary',
        'on-primary',
        'primary-container',
        'on-primary-container',
        'secondary-container',
        'on-secondary-container',
        'on-surface',
        'on-surface-variant',
        'outline',
        'outline-variant',
        'grid-line',
        'success',
        'success-container',
        'on-success-container',
        'warning',
        'warning-container',
        'on-warning-container',
        'error',
        'error-container',
        'on-error-container',
      ];
      document.documentElement.classList.remove('dark-theme');
      document.body.classList.remove('dark-theme');
      const lightStyle = getComputedStyle(document.documentElement);
      const light = Object.fromEntries(
        colorNames.map((name) => [name, lightStyle.getPropertyValue(`--app-${name}`).trim()]),
      );
      document.documentElement.classList.add('dark-theme');
      document.body.classList.add('dark-theme');
      const darkStyle = getComputedStyle(document.documentElement);
      const dark = Object.fromEntries(
        colorNames.map((name) => [name, darkStyle.getPropertyValue(`--app-${name}`).trim()]),
      );
      document.documentElement.classList.remove('dark-theme');
      document.body.classList.remove('dark-theme');
      return { typography, colors: { light, dark } };
    });

    const screens: Record<string, unknown> = {};
    screens.timesheet = await measure(page, {
      shell: '.app-shell-container',
      sidebar: '.app-sidebar-drawer',
      sidebarBrand: '.sidebar-brand-header',
      workspaceSwitcher: '.workspace-switcher-btn',
      expandedNavItem: '.app-sidebar-drawer:not(.collapsed) .m3-nav-item',
      header: '.app-top-toolbar',
      monthSelector: '.month-selector-btn',
      todayButton: '.today-btn',
      viewToggle: '.m3-view-toggle',
      catchUpButton: '.catchup-btn',
      mainViewport: '.app-main-viewport',
      workspaceContent: '.workspace-content',
      summaryBanner: '.m3-summary-banner',
      summaryMetric: '.metric-item',
      ledgerContainer: '.m3-table-container',
      ledgerHeader: '.m3-ledger-table .ledger-header-row',
      ledgerHeaderCells: '.m3-ledger-table .ledger-header-row th',
      ledgerRow: '.m3-ledger-table .ledger-data-row',
      ledgerDatePill: '.date-pill',
      ledgerStatusChip: '.status-chip',
      ledgerProjectPill: '.project-pill',
    });

    await page.click('.collapse-btn');
    await page.waitForSelector('.app-sidebar-drawer.collapsed');
    screens.collapsedSidebar = await measure(page, {
      sidebar: '.app-sidebar-drawer.collapsed',
      collapsedNavItem: '.app-sidebar-drawer.collapsed .m3-nav-item',
    });
    await page.click('.collapse-btn');
    await page.waitForSelector('.app-sidebar-drawer:not(.collapsed)');

    const rows = await page.$$('.m3-ledger-table .ledger-data-row');
    if (rows.length < 4) throw new Error('The visual fixture did not render four ledger rows.');
    await rows[3].click();
    await page.waitForSelector('.m3-day-editor');
    if (!(await page.$('.pay-summary-card'))) {
      await page.click('.m3-status-toggle mat-button-toggle:first-child button');
      await page.waitForSelector('.pay-summary-card');
    }
    screens.dayEditor = await measure(page, {
      dayEditor: '.m3-day-editor',
      editorHeader: '.editor-header',
      editorBody: '.editor-body',
      editorStatusToggle: '.m3-status-toggle',
      editorPayCard: '.pay-summary-card',
      editorFooter: '.editor-footer',
    });
    await page.click('[aria-label="Close day editor"]');
    await page.waitForSelector('.m3-day-editor', { hidden: true });

    await page.click('.m3-view-toggle mat-button-toggle:nth-child(2) button');
    await page.waitForSelector('.m3-calendar-container');
    screens.calendar = await measure(page, {
      calendarContainer: '.m3-calendar-container',
      calendarHeader: '.calendar-header-row',
      calendarCell: '.calendar-day-cell',
      calendarCurrentCell: '.calendar-day-cell:not(.other-month)',
      calendarAdjacentCell: '.calendar-day-cell.other-month',
      calendarToday: '.today-circle',
      calendarChip: '.calendar-chip',
      calendarWorkedChip: '.calendar-chip.worked-chip',
      calendarOffChip: '.calendar-chip.off-chip',
    });
    await page.click('.calendar-day-cell:not(.other-month)');
    await page.waitForSelector('.calendar-day-cell.is-selected');
    screens.calendarSelected = await measure(page, {
      calendarSelectedCell: '.calendar-day-cell.is-selected',
    });
    await page.click('[aria-label="Close day editor"]');

    await page.click('.workspace-switcher-btn');
    await page.waitForSelector('.mat-mdc-menu-panel');
    await clickText(page, '.mat-mdc-menu-panel button', 'Client Work');
    await page.waitForSelector('.month-notice');
    screens.unstartedMonth = await measure(page, { monthNotice: '.month-notice' });

    await open(page, '/projects', '.projects-page-container');
    screens.projects = await measure(page, {
      projectsPage: '.projects-page-container',
      projectsGrid: '.projects-layout-grid',
      projectCard: '.projects-page-container .m3-outlined-card',
      projectRow: '.project-item-row',
    });

    await open(page, '/settings', '.settings-page-container');
    screens.settings = await measure(page, {
      settingsPage: '.settings-page-container',
      settingsHeader: '.settings-header-block',
      settingsTabs: '.m3-settings-tabs .mat-mdc-tab-header',
      settingsCard: '.settings-page-container .m3-outlined-card',
      settingsField: '.settings-page-container .mat-mdc-form-field',
    });

    await open(page, '/settings/data-backups', '.backups-page-container');
    screens.backups = await measure(page, {
      backupsPage: '.backups-page-container',
      backupCard: '.backups-page-container .m3-outlined-card',
      dataLocation: '.data-location',
    });

    await open(page, '/timesheet', '.m3-ledger-table');
    await page.click('.month-selector-btn');
    await page.waitForSelector('.mat-mdc-menu-panel');
    screens.monthMenu = await measure(page, { menuPanel: '.mat-mdc-menu-panel' });
    await page.keyboard.press('Escape');
    await page.waitForSelector('.mat-mdc-menu-panel', { hidden: true });

    await page.click('.workspace-switcher-btn');
    await page.waitForSelector('.mat-mdc-menu-panel');
    screens.workspaceMenu = await measure(page, { menuPanel: '.mat-mdc-menu-panel' });
    await clickText(page, '.mat-mdc-menu-panel button', 'Manage workspaces');
    await page.waitForSelector('.mat-mdc-dialog-surface');
    screens.workspaceDialog = await measure(page, { dialogSurface: '.mat-mdc-dialog-surface' });
    await page.click('.mat-mdc-dialog-surface .add-ws-btn');
    await page.waitForSelector('.mat-mdc-dialog-surface input');
    await page.type('.mat-mdc-dialog-surface input', 'Visual workspace');
    await page.click('.mat-mdc-dialog-surface .save-btn');
    await page.waitForSelector('.mat-mdc-snack-bar-container');
    screens.snackbar = await measure(page, { snackbar: '.mat-mdc-snack-bar-container' });

    await open(page, '/projects', '.projects-page-container');
    await page.click('.color-trigger');
    await page.waitForSelector('.m3-color-menu');
    screens.colorPicker = await measure(page, { menuPanel: '.m3-color-menu' });
    await open(page, '/projects', '.projects-page-container');
    await page.click('[aria-label^="Delete "]');
    await page.waitForSelector('.mat-mdc-dialog-surface');
    screens.confirmationDialog = await measure(page, {
      dialogSurface: '.mat-mdc-dialog-surface',
    });
    await page.keyboard.press('Escape');

    await open(page, '/settings', '.settings-page-container');
    await page.click('.mat-mdc-select-trigger');
    await page.waitForSelector('.mat-mdc-select-panel');
    screens.selectPanel = await measure(page, { selectPanel: '.mat-mdc-select-panel' });

    fs.mkdirSync(path.dirname(output), { recursive: true });
    fs.writeFileSync(
      output,
      `${JSON.stringify(
        {
          viewport: { width: 1366, height: 820, deviceScaleFactor: 1 },
          fixedToday: '2026-08-18',
          ...foundations,
          screens,
        },
        null,
        2,
      )}\n`,
    );
    console.log(`Wrote ${path.relative(root, output)}`);
  } finally {
    await browser.close();
  }
}

capture().catch((error) => {
  console.error(error);
  process.exitCode = 1;
});
