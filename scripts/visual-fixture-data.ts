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

export const VISUAL_TODAY = '2026-08-18';
export const VISUAL_NOW = '2026-08-18T10:00:00.000Z';

export function createVisualFixture(theme = ThemePreference.Light, scale = 100) {
  const workspaces = [
    {
      id: 'ws-default',
      name: 'Main Workspace',
      color: '#5F875F',
      type: WorkspaceType.Employment,
      organizationName: 'Example Company',
      workerName: 'Example Worker',
      createdAt: VISUAL_NOW,
      updatedAt: VISUAL_NOW,
    },
    {
      id: 'ws-client',
      name: 'Client Work',
      color: '#8E24AA',
      type: WorkspaceType.Contract,
      organizationName: 'Example Client',
      workerName: 'Example Worker',
      createdAt: VISUAL_NOW,
      updatedAt: VISUAL_NOW,
    },
  ];
  const preferences = {
    ...DEFAULT_PREFERENCES,
    activeWorkspaceId: 'ws-default',
    themePreference: theme,
    languagePreference: LanguagePreference.English,
    interfaceScalePercent: scale,
    hasCompletedSetup: true,
  };
  const settings = {
    ...structuredClone(DEFAULT_SETTINGS),
    workspaceId: 'ws-default',
    employeeName: 'Example Worker',
    employerName: 'Example Company',
    openingBalanceMinutes: 300,
    themePreference: theme,
    languagePreference: LanguagePreference.English,
    interfaceScalePercent: scale,
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
  };
  const projects = [
    {
      workspaceId: 'ws-default',
      id: 'proj-default',
      name: 'General',
      color: '#5F875F',
      isActive: true,
      isDefault: true,
    },
    {
      workspaceId: 'ws-default',
      id: 'proj-client-a',
      name: 'Client A',
      color: '#039BE5',
      isActive: true,
      isDefault: false,
    },
    {
      workspaceId: 'ws-default',
      id: 'proj-internal',
      name: 'Internal',
      color: '#F6BF26',
      isActive: true,
      isDefault: false,
    },
    {
      workspaceId: 'ws-default',
      id: 'proj-archive',
      name: 'Archived',
      color: '#F4511E',
      isActive: false,
      isDefault: false,
    },
  ];
  const entries = [
    ['2026-08-03', WorkEntryStatus.Worked, '08:00', '16:30', 30, 'General', null],
    ['2026-08-04', WorkEntryStatus.Worked, '08:00', '19:30', 30, 'Client A', 'Overtime'],
    ['2026-08-05', WorkEntryStatus.Off, null, null, 0, null, 'Vacation'],
    ['2026-08-06', WorkEntryStatus.Worked, '22:00', '06:30', 30, 'Client A', 'Night shift'],
    ['2026-08-07', WorkEntryStatus.Worked, '08:30', '17:00', 30, 'Internal', 'Planning'],
    ['2026-08-15', WorkEntryStatus.Worked, '09:00', '14:00', 0, 'General', 'Weekend support'],
    ['2026-08-18', WorkEntryStatus.Worked, '08:00', '16:30', 30, 'General', 'Today'],
    ['2026-12-25', WorkEntryStatus.Off, null, null, 0, null, 'Public holiday'],
  ].map(([date, status, startTime, endTime, lunchMinutes, projectName, notes]) => ({
    workspaceId: 'ws-default',
    date: date as string,
    status: status as WorkEntryStatus,
    startTime: startTime as string | null,
    endTime: endTime as string | null,
    lunchMinutes: lunchMinutes as number,
    projectName: projectName as string | null,
    notes: notes as string | null,
    scheduledMinutesOverride: null,
    createdAt: VISUAL_NOW,
    updatedAt: VISUAL_NOW,
  }));
  const monthRecords = [
    {
      workspaceId: 'ws-default',
      year: 2026,
      month: 8,
      openingBalanceMinutes: 300,
      expectedMinutesOverride: null,
      openingBalanceWasEdited: true,
    },
    {
      workspaceId: 'ws-default',
      year: 2026,
      month: 12,
      openingBalanceMinutes: 300,
      expectedMinutesOverride: null,
      openingBalanceWasEdited: false,
    },
  ];

  return { workspaces, preferences, settings, projects, entries, monthRecords };
}

export function browserStorageEntries(theme = ThemePreference.Light, scale = 100) {
  const fixture = createVisualFixture(theme, scale);
  const clientSettings = {
    ...structuredClone(fixture.settings),
    workspaceId: 'ws-client',
    employeeName: 'Example Worker',
    employerName: 'Example Client',
  };
  return [
    ['dagsverk_workspaces', fixture.workspaces],
    ['dagsverk_preferences', fixture.preferences],
    ['dagsverk_settings_ws-default', fixture.settings],
    ['dagsverk_entries_ws-default', fixture.entries],
    ['dagsverk_months_ws-default', fixture.monthRecords],
    ['dagsverk_projects_ws-default', fixture.projects],
    ['dagsverk_settings_ws-client', clientSettings],
    ['dagsverk_entries_ws-client', []],
    [
      'dagsverk_projects_ws-client',
      [
        {
          workspaceId: 'ws-client',
          id: 'proj-client-default',
          name: 'General',
          color: '#8E24AA',
          isActive: true,
          isDefault: true,
        },
      ],
    ],
    ...fixture.monthRecords.map((record) => [
      `dagsverk_month_ws-default_${record.year}_${record.month}`,
      record,
    ]),
  ] as const;
}
