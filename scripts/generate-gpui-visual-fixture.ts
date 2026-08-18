import { mkdirSync, rmSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import Database from 'better-sqlite3';
import { DatabaseService } from '../electron/database.service';
import { createVisualFixture, VISUAL_NOW } from './visual-fixture-data';

const output = resolve(process.argv[2] || 'gpui/fixtures/databases/visual-parity.db');
const fixture = createVisualFixture();
mkdirSync(dirname(output), { recursive: true });
for (const path of [output, `${output}-wal`, `${output}-shm`]) rmSync(path, { force: true });

const database = new DatabaseService(output);
try {
  for (const workspace of fixture.workspaces) database.saveWorkspace(workspace);
  database.saveAppPreferences(fixture.preferences);
  database.saveSettings(fixture.settings, 'ws-default');
  for (const project of fixture.projects) database.saveProject(project, 'ws-default');
  for (const entry of fixture.entries) database.saveWorkEntry(entry, 'ws-default');
  for (const record of fixture.monthRecords) database.saveMonthRecord(record, 'ws-default');
} finally {
  database.close();
}

const normalized = new Database(output);
normalized
  .prepare('UPDATE Workspaces SET CreatedAt = ?, UpdatedAt = ?')
  .run(VISUAL_NOW, VISUAL_NOW);
normalized
  .prepare('UPDATE WorkEntries SET CreatedAt = ?, UpdatedAt = ?')
  .run(VISUAL_NOW, VISUAL_NOW);
normalized.exec('VACUUM');
normalized.close();

console.log(output);
