export enum WorkEntryStatus {
  Incomplete = 0,
  Worked = 1,
  Off = 2
}

export enum ThemePreference {
  System = 0,
  Light = 1,
  Dark = 2
}

export enum MonthViewPreference {
  Ledger = 0,
  Calendar = 1
}

export enum LanguagePreference {
  System = 0,
  English = 1,
  Swedish = 2
}

export enum WorkspaceType {
  Employment = 0,
  Contract = 1,
  Personal = 2
}

export type CurrencyPreference = 'SEK' | 'EUR' | 'USD' | 'GBP' | 'NOK' | 'DKK';

export enum ExportLanguagePreference {
  Swedish = 0,
  English = 1,
  System = 2
}

export enum OvertimeCompensationMode {
  CompTime = 0,
  Paid = 1
}

export enum OvertimeThresholdMode {
  FixedDailyHours = 0,
  ScheduledHours = 1
}

export enum CompensationRuleType {
  Overtime = 0,
  Ob = 1
}

export enum CompensationRateType {
  HourlyPremiumPercent = 0,
  FixedHourlyAmount = 1,
  FullTimeMonthlySalaryDivisor = 2
}

export enum OvertimeDayCategory {
  AllDays = 0,
  ScheduledWorkdays = 1,
  NonWorkdays = 2,
  Monday = 3,
  Tuesday = 4,
  Wednesday = 5,
  Thursday = 6,
  Friday = 7,
  Saturday = 8,
  Sunday = 9,
  PublicHolidays = 10,
  ScheduledWeekdays = 11,
  Weekends = 12,
  MajorHolidays = 13
}

export enum SalaryType {
  Hourly = 0,
  Monthly = 1
}

export enum TaxMode {
  Disabled = 0,
  PrimaryIncomeTaxTable = 1,
  SecondaryIncomeThirtyPercent = 2,
  ManualMonthlyDeduction = 3
}

export type TaxUnavailableReason = 'None' | 'ManualDeductionNotConfigured' | 'TaxYearNotBundled';

export interface Workspace {
  id: string;
  name: string;
  color: string;
  type: WorkspaceType;
  organizationName?: string;
  workerName?: string;
  createdAt: string;
  updatedAt: string;
}

export interface AppPreferences {
  id?: number;
  activeWorkspaceId: string;
  themePreference: ThemePreference;
  languagePreference: LanguagePreference;
  interfaceScalePercent: number;
  monthViewPreference: MonthViewPreference;
}

export interface WorkEntry {
  workspaceId?: string;
  date: string; // YYYY-MM-DD
  status: WorkEntryStatus;
  startTime: string | null; // HH:mm
  endTime: string | null; // HH:mm
  lunchMinutes: number;
  projectName: string | null;
  notes: string | null;
  scheduledMinutesOverride: number | null;
  createdAt?: string;
  updatedAt?: string;
}

export interface OvertimeRateBand {
  name: string;
  dayCategory: OvertimeDayCategory;
  startTime: string; // HH:mm
  endTime: string; // HH:mm
  compensationType: CompensationRuleType;
  rateType: CompensationRateType;
  rateValue: number;
}

export interface SalarySettings {
  type: SalaryType;
  hourlyRate: number;
  monthlySalary: number;
  employmentPercent: number;
}

export interface ExpectedHoursSettings {
  hoursPerWorkday: number;
  workingWeekdays: number[]; // 1=Mon, 2=Tue, 3=Wed, 4=Thu, 5=Fri, 6=Sat, 0=Sun (matching JS getDay())
  excludePublicHolidays: boolean;
}

export interface TaxSettings {
  mode: TaxMode;
  taxYear: number;
  tableNumber: number;
  column: number;
  manualMonthlyDeduction: number | null;
}

export interface OvertimeCompensationSettings {
  mode: OvertimeCompensationMode;
  defaultRateType: CompensationRateType;
  defaultRateValue: number;
  dailyThresholdHours: number;
  thresholdMode: OvertimeThresholdMode;
  rateBands: OvertimeRateBand[];
}

export interface AppSettings {
  id?: number;
  workspaceId?: string;
  employeeName: string;
  employerName: string;
  defaultProject: string;
  salary: SalarySettings;
  expectedHours: ExpectedHoursSettings;
  defaultStartTime: string;
  defaultEndTime: string;
  defaultLunchMinutes: number;
  taxSettings: TaxSettings;
  themePreference: ThemePreference;
  openingBalanceMinutes: number;
  monthViewPreference: MonthViewPreference;
  languagePreference: LanguagePreference;
  currencyPreference: CurrencyPreference;
  interfaceScalePercent: number;
  exportLanguagePreference: ExportLanguagePreference;
  overtimeCompensation: OvertimeCompensationSettings;
}

export interface MonthRecord {
  workspaceId?: string;
  year: number;
  month: number; // 1-12
  openingBalanceMinutes: number;
  expectedMinutesOverride: number | null;
  openingBalanceWasEdited: boolean;
}

export interface Project {
  workspaceId?: string;
  id: string;
  name: string;
  color?: string;
  isActive: boolean;
  isDefault: boolean;
}

export interface MonthlySummary {
  year: number;
  month: number;
  workedMinutes: number;
  regularMinutes: number;
  overtimeMinutes: number;
  balanceEligibleMinutes: number;
  expectedMinutes: number;
  monthlyDifferenceMinutes: number;
  openingBalanceMinutes: number;
  closingBalanceMinutes: number;
  grossSalary: number;
  baseSalary: number;
  overtimeCompensation: number;
  obCompensation: number;
  obMinutes: number;
  completedDayCount: number;
  missingPastDays: string[];
  workedHours: number;
  regularHours: number;
  overtimeHours: number;
  obHours: number;
  expectedHours: number;
}

export interface DailyPayBreakdown {
  regularPay: number;
  overtimePay: number;
  obPay: number;
  obMinutes: number;
  total: number;
}

export interface TaxEstimate {
  grossPay: number;
  preliminaryTax: number | null;
  estimatedNetPay: number | null;
  unavailableReason: TaxUnavailableReason;
  isAvailable: boolean;
}

export interface ReportExportRequest {
  year: number;
  month: number;
  employeeName: string;
  employerName: string;
  entries: WorkEntry[];
  summary: MonthlySummary;
  language: ExportLanguagePreference;
  expectedHours?: ExpectedHoursSettings;
  overtimeSettings?: OvertimeCompensationSettings;
  overtimeMode: OvertimeCompensationMode;
  dailyOvertimeThresholdHours: number;
}

export const DEFAULT_WORKSPACE: Workspace = {
  id: 'ws-default',
  name: 'Main Workspace',
  color: '#5F875F',
  type: WorkspaceType.Employment,
  organizationName: 'Acme AB',
  workerName: 'Agnes Larsson',
  createdAt: new Date().toISOString(),
  updatedAt: new Date().toISOString()
};

export const DEFAULT_PREFERENCES: AppPreferences = {
  activeWorkspaceId: 'ws-default',
  themePreference: ThemePreference.System,
  languagePreference: LanguagePreference.System,
  interfaceScalePercent: 100,
  monthViewPreference: MonthViewPreference.Ledger
};

export const DEFAULT_SETTINGS: AppSettings = {
  workspaceId: 'ws-default',
  employeeName: 'Agnes Larsson',
  employerName: 'Acme AB',
  defaultProject: 'General',
  salary: {
    type: SalaryType.Hourly,
    hourlyRate: 250,
    monthlySalary: 40000,
    employmentPercent: 100
  },
  expectedHours: {
    hoursPerWorkday: 8,
    workingWeekdays: [1, 2, 3, 4, 5],
    excludePublicHolidays: true
  },
  defaultStartTime: '08:00',
  defaultEndTime: '16:30',
  defaultLunchMinutes: 30,
  taxSettings: {
    mode: TaxMode.PrimaryIncomeTaxTable,
    taxYear: 2026,
    tableNumber: 30,
    column: 1,
    manualMonthlyDeduction: null
  },
  themePreference: ThemePreference.System,
  openingBalanceMinutes: 0,
  monthViewPreference: MonthViewPreference.Ledger,
  languagePreference: LanguagePreference.System,
  currencyPreference: 'SEK',
  interfaceScalePercent: 100,
  exportLanguagePreference: ExportLanguagePreference.System,
  overtimeCompensation: {
    mode: OvertimeCompensationMode.CompTime,
    defaultRateType: CompensationRateType.HourlyPremiumPercent,
    defaultRateValue: 50,
    dailyThresholdHours: 8,
    thresholdMode: OvertimeThresholdMode.ScheduledHours,
    rateBands: []
  }
};
