import fs from 'node:fs';
import path from 'node:path';
import puppeteer from 'puppeteer-core';

const root = path.resolve(import.meta.dirname, '..');
const output = path.resolve(
  process.argv[2] ?? path.join(root, 'reference/design-metrics/design.json'),
);
const url = process.env.DAGSVERK_CAPTURE_URL ?? 'http://localhost:4200';

async function capture() {
  const browser = await puppeteer.launch({
    executablePath: process.env.CHROMIUM_PATH ?? '/usr/bin/chromium',
    headless: true,
    args: ['--no-sandbox', '--disable-setuid-sandbox'],
  });

  try {
    const page = await browser.newPage();
    await page.setViewport({ width: 1366, height: 820, deviceScaleFactor: 1 });
    await page.goto(url, { waitUntil: 'networkidle0' });

    const metrics = await page.evaluate(() => {
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
      const lightColors = Object.fromEntries(
        colorNames.map((name) => [name, lightStyle.getPropertyValue(`--app-${name}`).trim()]),
      );
      document.documentElement.classList.add('dark-theme');
      document.body.classList.add('dark-theme');
      const darkStyle = getComputedStyle(document.documentElement);
      const darkColors = Object.fromEntries(
        colorNames.map((name) => [name, darkStyle.getPropertyValue(`--app-${name}`).trim()]),
      );

      const selectors = {
        sidebar: '.app-sidebar',
        header: '.app-top-bar',
        ledgerHeader: '.ledger-table thead tr',
        ledgerRow: '.ledger-table tbody tr',
        dayEditor: '.editor-panel',
        dialog: '.mat-mdc-dialog-container',
        summaryCard: '.summary-card',
      };
      const components = Object.fromEntries(
        Object.entries(selectors).map(([name, selector]) => {
          const element = document.querySelector<HTMLElement>(selector);
          if (!element) return [name, null];
          const bounds = element.getBoundingClientRect();
          const style = getComputedStyle(element);
          return [
            name,
            {
              width: bounds.width,
              height: bounds.height,
              padding: style.padding,
              gap: style.gap,
              borderRadius: style.borderRadius,
              boxShadow: style.boxShadow,
              opacity: style.opacity,
            },
          ];
        }),
      );

      return {
        viewport: { width: innerWidth, height: innerHeight, deviceScaleFactor: devicePixelRatio },
        typography,
        colors: { light: lightColors, dark: darkColors },
        components,
      };
    });

    fs.mkdirSync(path.dirname(output), { recursive: true });
    fs.writeFileSync(output, `${JSON.stringify(metrics, null, 2)}\n`);
    console.log(`Wrote ${path.relative(root, output)}`);
  } finally {
    await browser.close();
  }
}

capture().catch((error) => {
  console.error(error);
  process.exitCode = 1;
});
