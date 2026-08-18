import { mkdtempSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { spawnSync } from 'node:child_process';
import { DatabaseService } from '../electron/database.service';
import {
  CompensationRateType,
  CompensationRuleType,
  OvertimeDayCategory,
  WorkEntryStatus,
} from '../src/app/core/models';

const root = join(import.meta.dirname, '..');
const directory = mkdtempSync(join(tmpdir(), 'dagsverk-db-compat-'));
const electronDatabase = join(directory, 'electron-created.db');
const rustDatabase = join(directory, 'rust-created.db');

function runRust(mode: string) {
  const result = spawnSync(
    'cargo',
    [
      'run',
      '--quiet',
      '--manifest-path',
      join(root, 'gpui', 'Cargo.toml'),
      '-p',
      'dagsverk-data',
      '--example',
      'compatibility',
      '--',
      mode,
      electronDatabase,
      rustDatabase,
    ],
    { cwd: root, encoding: 'utf8' },
  );
  if (result.status !== 0) {
    throw new Error(`Rust compatibility pass failed:\n${result.stdout}\n${result.stderr}`);
  }
}

try {
  const electron = new DatabaseService(electronDatabase);
  electron.saveWorkspace({
    id: 'electron-workspace',
    name: 'Electron Workspace',
    color: '#123456',
    type: 1,
    workerName: 'Electron Worker',
    organizationName: 'Electron Client',
    createdAt: '2026-08-18T10:00:00.000Z',
  });
  const settings = electron.getSettings('electron-workspace');
  settings.employeeName = 'Electron Worker';
  settings.salary.hourlyRate = 321.5;
  settings.overtimeCompensation.rateBands = [
    {
      name: 'Electron Night',
      dayCategory: OvertimeDayCategory.MajorHolidays,
      startTime: '22:00',
      endTime: '06:00',
      compensationType: CompensationRuleType.Ob,
      rateType: CompensationRateType.FixedHourlyAmount,
      rateValue: 55.5,
    },
  ];
  electron.saveSettings(settings, 'electron-workspace');
  electron.saveWorkEntry(
    {
      date: '2026-08-17',
      status: WorkEntryStatus.Worked,
      startTime: '22:00',
      endTime: '06:00',
      lunchMinutes: 0,
      projectName: 'Electron Project',
      notes: 'electron-created',
      scheduledMinutesOverride: 0,
    },
    'electron-workspace',
  );
  electron.saveMonthRecord(
    {
      year: 2026,
      month: 8,
      openingBalanceMinutes: 45,
      expectedMinutesOverride: 6000,
      openingBalanceWasEdited: true,
    },
    'electron-workspace',
  );
  electron.saveProject(
    {
      id: 'electron-project',
      name: 'Electron Project',
      color: '#abcdef',
      isActive: true,
      isDefault: true,
    },
    'electron-workspace',
  );
  electron.close();

  runRust('exchange');

  const electronRoundTrip = new DatabaseService(electronDatabase);
  const electronEntries = electronRoundTrip.getWorkEntries(2026, 8, 'electron-workspace');
  if (
    !electronEntries.some((entry) => entry.date === '2026-08-18' && entry.notes === 'rust-created')
  ) {
    throw new Error('Electron did not read the Rust modification.');
  }
  electronRoundTrip.close();

  const rustCreated = new DatabaseService(rustDatabase);
  const rustWorkspace = rustCreated
    .getWorkspaces()
    .find((workspace) => workspace.id === 'rust-workspace');
  if (!rustWorkspace || rustWorkspace.name !== 'Rust Workspace') {
    throw new Error('Electron did not read the Rust workspace.');
  }
  const rustRecord = rustCreated.getMonthRecord(2026, 8, 0, 'rust-workspace');
  if (rustRecord.openingBalanceMinutes !== 90 || !rustRecord.openingBalanceWasEdited) {
    throw new Error('Electron did not read the Rust month record.');
  }
  rustCreated.saveWorkEntry(
    {
      date: '2026-08-19',
      status: WorkEntryStatus.Off,
      startTime: null,
      endTime: null,
      lunchMinutes: 0,
      projectName: null,
      notes: 'electron-round-trip',
      scheduledMinutesOverride: null,
    },
    'rust-workspace',
  );
  rustCreated.close();

  runRust('assert-round-trip');
  console.log('Electron and Rust database compatibility passed.');
} finally {
  rmSync(directory, { recursive: true, force: true });
}
