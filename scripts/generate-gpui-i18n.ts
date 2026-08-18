import fs from 'node:fs';
import path from 'node:path';
import { ENGLISH_RESOURCES } from '../src/app/core/i18n/en';
import { SWEDISH_RESOURCES } from '../src/app/core/i18n/sv';

const root = path.resolve(import.meta.dirname, '..');
const output = path.join(root, 'gpui/assets/i18n');

fs.mkdirSync(output, { recursive: true });
for (const [name, resources] of [
  ['en.json', ENGLISH_RESOURCES],
  ['sv.json', SWEDISH_RESOURCES],
] as const) {
  const sorted = Object.fromEntries(Object.entries(resources).sort(([left], [right]) => left.localeCompare(right)));
  fs.writeFileSync(path.join(output, name), `${JSON.stringify(sorted, null, 2)}\n`);
}

console.log(`Wrote GPUI localization catalogs with ${Object.keys(ENGLISH_RESOURCES).length} keys`);
