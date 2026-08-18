import { mkdirSync, rmSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import Database from 'better-sqlite3';
import { DatabaseService } from '../electron/database.service';
import {
  CompensationRateType,
  CompensationRuleType,
  DEFAULT_PREFERENCES,
  DEFAULT_SETTINGS,
  LanguagePreference,
  ObOvertimeCombinationMode,
  OvertimeDayCategory,
  TaxMode,
  ThemePreference,
  WorkEntryStatus,
  WorkspaceType,
} from '../src/app/core/models';

const output = resolve(process.argv[2] || 'gpui/fixtures/databases/visual-parity.db');
const now = '2026-06-25T10:00:00.000Z';
mkdirSync(dirname(output), { recursive: true });
for (const path of [output, `${output}-wal`, `${output}-shm`]) rmSync(path, { force: true });

const database = new DatabaseService(output);
try {
  database.saveWorkspace({
    id: 'ws-default',
    name: 'Main Workspace',
    color: '#5F875F',
    type: WorkspaceType.Employment,
    organizationName: 'Example Company',
    workerName: 'Example Worker',
    createdAt: now,
    updatedAt: now,
  });
  database.saveWorkspace({
    id: 'ws-client',
    name: 'Client Work',
    color: '#7B61A8',
    type: WorkspaceType.Contract,
    organizationName: 'Example Client',
    workerName: 'Example Worker',
    createdAt: now,
    updatedAt: now,
  });
  database.saveAppPreferences({
    ...DEFAULT_PREFERENCES,
    activeWorkspaceId: 'ws-default',
    themePreference: ThemePreference.Light,
    languagePreference: LanguagePreference.English,
    hasCompletedSetup: true,
  });
  database.saveSettings(
    {
      ...DEFAULT_SETTINGS,
      employeeName: 'Example Worker',
      employerName: 'Example Company',
      openingBalanceMinutes: 300,
      taxSettings: { ...DEFAULT_SETTINGS.taxSettings, mode: TaxMode.PrimaryIncomeTaxTable },
      overtimeCompensation: {
        ...DEFAULT_SETTINGS.overtimeCompensation,
        obOvertimeCombination: ObOvertimeCombinationMode.IncludeOb,
        rateBands: [
          {
            name: 'Evening OB',
            dayCategory: OvertimeDayCategory.AllDays,
            startTime: '18:00',
            endTime: '23:59',
            compensationType: CompensationRuleType.Ob,
            rateType: CompensationRateType.FixedHourlyAmount,
            rateValue: 45,
          },
          {
            name: 'Paid overtime',
            dayCategory: OvertimeDayCategory.ScheduledWorkdays,
            startTime: '00:00',
            endTime: '00:00',
            compensationType: CompensationRuleType.Overtime,
            rateType: CompensationRateType.HourlyPremiumPercent,
            rateValue: 50,
          },
        ],
      },
    },
    'ws-default',
  );
  database.saveProject(
    {
      workspaceId: 'ws-default',
      id: 'proj-client-a',
      name: 'Client A',
      color: '#3F7CAC',
      isActive: true,
      isDefault: false,
    },
    'ws-default',
  );
  database.saveProject(
    {
      workspaceId: 'ws-default',
      id: 'proj-archive',
      name: 'Archived',
      color: '#B06000',
      isActive: false,
      isDefault: false,
    },
    'ws-default',
  );

  const entries = [
    ['2026-06-01', WorkEntryStatus.Worked, '08:00', '16:30', 30, 'General', null],
    ['2026-06-02', WorkEntryStatus.Worked, '08:00', '19:30', 30, 'Client A', 'Overtime'],
    ['2026-06-03', WorkEntryStatus.Off, null, null, 0, null, 'Vacation'],
    ['2026-06-04', WorkEntryStatus.Worked, '22:00', '06:30', 30, 'Client A', 'Night shift'],
    ['2026-06-05', WorkEntryStatus.Worked, '08:30', '17:00', 30, 'General', 'Planning'],
    ['2026-06-20', WorkEntryStatus.Worked, '09:00', '13:00', 0, 'General', 'Holiday work'],
  ] as const;
  for (const [date, status, startTime, endTime, lunchMinutes, projectName, notes] of entries) {
    database.saveWorkEntry(
      {
        workspaceId: 'ws-default',
        date,
        status,
        startTime,
        endTime,
        lunchMinutes,
        projectName,
        notes,
        scheduledMinutesOverride: null,
      },
      'ws-default',
    );
  }
  database.saveMonthRecord(
    {
      workspaceId: 'ws-default',
      year: 2026,
      month: 6,
      openingBalanceMinutes: 300,
      expectedMinutesOverride: null,
      openingBalanceWasEdited: true,
    },
    'ws-default',
  );
} finally {
  database.close();
}

const normalized = new Database(output);
normalized.prepare('UPDATE Workspaces SET CreatedAt = ?, UpdatedAt = ?').run(now, now);
normalized.prepare('UPDATE WorkEntries SET CreatedAt = ?, UpdatedAt = ?').run(now, now);
normalized.exec('VACUUM');
normalized.close();

console.log(output);
