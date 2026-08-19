import fs from 'node:fs';
import path from 'node:path';

const root = path.resolve(import.meta.dirname, '..');
const source = path.join(root, 'src/app');
const output = path.join(root, 'gpui/assets/icons/material-symbols.json');

function filesIn(directory: string): string[] {
  return fs.readdirSync(directory, { withFileTypes: true }).flatMap((entry) => {
    const target = path.join(directory, entry.name);
    return entry.isDirectory() ? filesIn(target) : [target];
  });
}

const icons = new Set<string>();
for (const file of filesIn(source).filter(
  (file) => file.endsWith('.html') || file.endsWith('.ts'),
)) {
  const contents = fs.readFileSync(file, 'utf8');
  for (const match of contents.matchAll(/<mat-icon\b[^>]*>([\s\S]*?)<\/mat-icon>/g)) {
    const body = match[1].trim();
    if (/^[a-z0-9_]+$/.test(body)) icons.add(body);
    for (const quoted of body.matchAll(/['"]([a-z0-9_]+)['"]/g)) icons.add(quoted[1]);
  }
  for (const match of contents.matchAll(/\bicon\s*:\s*['"]([a-z0-9_]+)['"]/g)) {
    icons.add(match[1]);
  }
}

// These values come from the sidebar's computed update icon.
for (const icon of ['check_circle', 'download', 'error']) icons.add(icon);

fs.mkdirSync(path.dirname(output), { recursive: true });
fs.writeFileSync(output, `${JSON.stringify([...icons].sort(), null, 2)}\n`);
console.log(`Wrote ${path.relative(root, output)} with ${icons.size} icons`);
