import { DatabaseService } from './database.service';
import { ExcelExportService, ReportExportRequest } from './excel-export.service';
import { OdsExportService } from './ods-export.service';
import JSZip from 'jszip';
import * as path from 'path';
import * as fs from 'fs';
import * as ExcelJS from 'exceljs';
import Database from 'better-sqlite3';
import { createHash } from 'crypto';

async function runTests() {
  console.log('--- Starting Dagsverk Backend Tests ---');

  // 1. Test SQLite Database Service
  const tempDbPath = path.join(__dirname, 'test-dagsverk.db');
  if (fs.existsSync(tempDbPath)) fs.unlinkSync(tempDbPath);

  const db = new DatabaseService(tempDbPath);
  console.log('✔ SQLite Database initialized successfully');

  // Test Settings
  const settings = db.getSettings();
  if (!settings || settings.expectedHours.hoursPerWorkday !== 8) {
    throw new Error('Default settings failed to load');
  }
  console.log('✔ Default settings verified');

  settings.employeeName = 'Agnes Tester';
  settings.employerName = 'Dagsverk AB';
  db.saveSettings(settings);
  const updatedSettings = db.getSettings();
  if (updatedSettings.employeeName !== 'Agnes Tester') {
    throw new Error('Settings save failed');
  }
  console.log('✔ Settings save/load verified');

  // Test WorkEntries
  db.saveWorkEntry({
    date: '2026-08-17',
    status: 1, // Worked
    startTime: '08:00',
    endTime: '17:00',
    lunchMinutes: 30,
    projectName: 'General',
    notes: 'Unit test work',
    scheduledMinutesOverride: null,
    compTimeMinutes: 90,
  });

  const entries = db.getWorkEntries(2026, 8);
  if (entries.length !== 1 || entries[0].startTime !== '08:00') {
    throw new Error('WorkEntry save/get failed');
  }
  if (entries[0].compTimeMinutes !== 90) {
    throw new Error('WorkEntry comp-time save/get failed');
  }
  console.log('✔ WorkEntry CRUD verified');
  const history = db.getBalanceHistory(2026, 9);
  if (history.length !== 1 || history[0].entries.length !== 1) {
    throw new Error('Balance history did not return the tracked month');
  }
  console.log('✔ Time balance history verified');

  // Test Projects
  const projects = db.getProjects();
  if (projects.length === 0) {
    throw new Error('Default project missing');
  }
  db.saveProject({ id: 'test-p1', name: 'Design Sprint', isActive: true, isDefault: false });
  const updatedProjects = db.getProjects();
  if (updatedProjects.length < 2) {
    throw new Error('Project save failed');
  }
  console.log('✔ Projects management verified');

  const tidverkPath = path.join(__dirname, 'test-tidverk.db');
  fs.rmSync(tidverkPath, { force: true });
  const tidverk = new Database(tidverkPath);
  tidverk.exec(`
    CREATE TABLE Settings (
      Id INTEGER PRIMARY KEY,
      EmployeeName TEXT,
      EmployerName TEXT,
      DefaultProject TEXT,
      HourlyRate TEXT,
      ExpectedHoursPerWorkday TEXT,
      ExpectedWorkingWeekdays TEXT,
      ExcludePublicHolidays INTEGER,
      DefaultStartTime TEXT,
      DefaultEndTime TEXT,
      DefaultLunchMinutes INTEGER,
      ThemePreference INTEGER,
      MonthViewPreference INTEGER,
      LanguagePreference INTEGER,
      InterfaceScalePercent INTEGER,
      CurrencyPreference INTEGER,
      ExportLanguagePreference INTEGER,
      OpeningBalanceMinutes INTEGER
    );
    CREATE TABLE WorkEntries (
      Date TEXT PRIMARY KEY,
      Status INTEGER,
      StartTime TEXT,
      EndTime TEXT,
      LunchMinutes INTEGER,
      ProjectName TEXT,
      Notes TEXT,
      ScheduledMinutesOverride INTEGER,
      CreatedAt TEXT,
      UpdatedAt TEXT
    );
    CREATE TABLE Months (
      Year INTEGER,
      Month INTEGER,
      OpeningBalanceMinutes INTEGER,
      ExpectedMinutesOverride INTEGER,
      OpeningBalanceWasEdited INTEGER
    );
    CREATE TABLE Projects (Id TEXT, Name TEXT, IsActive INTEGER, IsDefault INTEGER);
    INSERT INTO Settings VALUES (
      1, 'Tidverk User', 'Imported AB', 'Imported Project', '275.5', '7.5',
      '1,2,3,4,5', 1, '07:30:00', '16:00:00', 30, 2, 1, 2, 110, 1, 1, 45
    );
    INSERT INTO WorkEntries VALUES (
      '2026-07-01', 1, '07:30:00', '16:00:00', 30, 'Imported Project', 'Imported', NULL,
      '2026-07-01T06:00:00Z', '2026-07-01T15:00:00Z'
    );
    INSERT INTO Months VALUES (2026, 7, 45, NULL, 1);
    INSERT INTO Projects VALUES ('tidverk-project', 'Imported Project', 1, 1);
  `);
  tidverk.close();
  const sourceHash = createHash('sha256').update(fs.readFileSync(tidverkPath)).digest('hex');
  const imported = await db.importTidverkDatabase(tidverkPath);
  const importedEntry = db.getWorkEntries(2026, 7, imported.workspaceId)[0];
  const importedSettings = db.getSettings(imported.workspaceId);
  if (
    imported.entryCount !== 1 ||
    imported.workspaceName !== 'Imported AB' ||
    importedEntry?.startTime !== '07:30' ||
    importedSettings.salary.hourlyRate !== 275.5 ||
    !fs.existsSync(imported.sourceBackupPath) ||
    createHash('sha256').update(fs.readFileSync(tidverkPath)).digest('hex') !== sourceHash
  ) {
    throw new Error('Tidverk import did not preserve the source data');
  }
  console.log('✔ Tidverk database import verified');

  // Test Backup
  const backupFile = await db.createBackup(__dirname);
  if (!fs.existsSync(backupFile)) {
    throw new Error('Backup creation failed');
  }
  console.log('✔ SQLite Backup creation verified:', backupFile);

  const oldBackup = new Database(backupFile);
  oldBackup.exec('ALTER TABLE WorkEntries DROP COLUMN CompTimeMinutes');
  oldBackup.exec('ALTER TABLE WorkEntries DROP COLUMN DayOffReason');
  oldBackup.close();

  db.saveWorkEntry({
    date: '2026-08-18',
    status: 1,
    startTime: '08:00',
    endTime: '16:30',
    lunchMinutes: 30,
    projectName: 'General',
    notes: 'Created after backup',
    scheduledMinutesOverride: null,
    compTimeMinutes: 0,
  });
  await db.restoreBackup(backupFile);
  if (db.getWorkEntries(2026, 8).some((entry) => entry.date === '2026-08-18')) {
    throw new Error('Valid backup did not replace the current database');
  }
  db.saveWorkEntry({
    ...db.getWorkEntries(2026, 8)[0],
    dayOffReason: 'Comp time',
    compTimeMinutes: 60,
  });
  if (
    db.getWorkEntries(2026, 8)[0].compTimeMinutes !== 60 ||
    db.getWorkEntries(2026, 8)[0].dayOffReason !== 'Comp time'
  ) {
    throw new Error('Restored pre-comp-time backup did not migrate');
  }
  console.log('✔ Valid backup restore verified');

  const unrelatedPath = path.join(__dirname, 'unrelated.db');
  fs.rmSync(unrelatedPath, { force: true });
  fs.rmSync(tidverkPath, { force: true });
  fs.rmSync(path.join(__dirname, 'backups'), { recursive: true, force: true });
  const unrelated = new Database(unrelatedPath);
  unrelated.exec('CREATE TABLE OtherData (Id INTEGER PRIMARY KEY)');
  unrelated.close();

  let unrelatedRejected = false;
  try {
    await db.restoreBackup(unrelatedPath);
  } catch {
    unrelatedRejected = true;
  }
  if (!unrelatedRejected || db.getWorkEntries(2026, 8).length !== 1) {
    throw new Error('Invalid backup changed the current database');
  }
  console.log('✔ Invalid backup rejection verified');

  for (let index = 0; index < 7; index++) {
    await db.createBackup(__dirname, `retention-${index}`);
  }
  const retained = fs
    .readdirSync(__dirname)
    .filter((file) => file.startsWith('dagsverk-backup-') && file.endsWith('.db'));
  if (retained.length !== 5) {
    throw new Error(`Backup retention kept ${retained.length} files instead of 5`);
  }
  console.log('✔ Backup retention verified');

  // 2. Test Excel Export Service
  const testExcelPath = path.join(__dirname, 'test-report.xlsx');
  if (fs.existsSync(testExcelPath)) fs.unlinkSync(testExcelPath);

  const req: ReportExportRequest = {
    year: 2026,
    month: 8,
    employeeName: 'Agnes Tester',
    employerName: 'Dagsverk AB',
    entries: [
      {
        date: '2026-08-17',
        status: 1,
        startTime: '08:00',
        endTime: '17:00',
        lunchMinutes: 30,
        projectName: 'Design Sprint',
        notes: 'Testing excel export',
        scheduledMinutesOverride: null,
        compTimeMinutes: 0,
      },
    ],
    summary: {
      workedHours: 8.5,
      regularHours: 8,
      overtimeHours: 0.5,
      ordinaryPaidHours: 8,
      compTimeEarnedHours: 0.5,
      compTimeUsedHours: 0,
      obHours: 0,
      expectedHours: 168,
      monthlyDifferenceMinutes: -9570,
      openingBalanceMinutes: 60,
      closingBalanceMinutes: -9510,
    },
    language: 1, // English
    overtimeMode: 0, // Comp-Time
    dailyOvertimeThresholdHours: 8,
    hourlyPayBasis: 0,
    thresholdMinutesByDate: { '2026-08-17': 480 },
    scheduledMinutesByDate: { '2026-08-17': 480 },
  };

  await ExcelExportService.exportToFile(req, testExcelPath);
  if (!fs.existsSync(testExcelPath)) {
    throw new Error('Excel file was not created');
  }

  const workbook = new ExcelJS.Workbook();
  await workbook.xlsx.readFile(testExcelPath);
  if (workbook.worksheets.length < 2) {
    throw new Error('Expected 2 sheets in Excel workbook');
  }

  const sheet1 = workbook.worksheets[0];
  const sheet2 = workbook.worksheets[1];
  if (!String((sheet1.getCell('E21').value as ExcelJS.CellFormulaValue).formula).includes('MOD(')) {
    throw new Error('Excel overnight formula is not parity-safe');
  }
  if ((sheet1.views[0] as { ySplit?: number } | undefined)?.ySplit !== 4) {
    throw new Error('Excel header rows are not frozen');
  }
  if (
    !String((sheet1.getCell('E37').value as ExcelJS.CellFormulaValue).formula).includes('SUM(I') ||
    !String((sheet1.getCell('E38').value as ExcelJS.CellFormulaValue).formula).includes('SUM(I')
  ) {
    throw new Error('Daily-basis Excel totals did not use the hidden overtime column');
  }
  if (sheet1.getCell('D39').value !== null || sheet2.getColumn(1).values.includes('OB hours')) {
    throw new Error('Excel included empty OB rows');
  }
  console.log(`✔ Excel Export verified: Sheet 1 "${sheet1.name}", Sheet 2 "${sheet2.name}"`);

  const testOdsPath = path.join(__dirname, 'test-report.ods');
  await OdsExportService.exportToFile(req, testOdsPath);
  const ods = await JSZip.loadAsync(fs.readFileSync(testOdsPath));
  const content = await ods.file('content.xml')?.async('text');
  if (!content?.includes('August 2026') || !content.includes('Time balance')) {
    throw new Error('OpenDocument report sheets are incomplete');
  }
  console.log('✔ OpenDocument export verified');

  const monthlyWorkbook = ExcelExportService.createWorkbook({
    ...req,
    month: 6,
    entries: [],
    language: 0,
    hourlyPayBasis: 1,
    summary: {
      ...req.summary,
      workedHours: 141,
      regularHours: 133.5,
      overtimeHours: 7.5,
      ordinaryPaidHours: 136,
      compTimeEarnedHours: 5,
      compTimeUsedHours: 0,
      expectedHours: 136,
      monthlyDifferenceMinutes: 300,
      closingBalanceMinutes: 360,
    },
  });
  const monthlyReport = monthlyWorkbook.worksheets[0];
  const monthlyBalance = monthlyWorkbook.getWorksheet('Tidsbalans');
  if (
    monthlyReport.getCell('D36').value !== 'Totalt betalda timmar' ||
    monthlyReport.getCell('E36').value !== 136 ||
    monthlyBalance?.getCell('A6').value !== 'Intjänad komptid'
  ) {
    throw new Error('Monthly hourly export did not match Tidverk');
  }
  console.log('✔ Monthly hourly export parity verified');

  const augustWorkbook = ExcelExportService.createWorkbook({
    ...req,
    entries: [14, 21, 24, 31].map((day) => ({
      date: `2026-08-${day}`,
      status: 2,
      startTime: null,
      endTime: null,
      lunchMinutes: 0,
      projectName: null,
      notes: 'Comp time',
      scheduledMinutesOverride: null,
      compTimeMinutes: 480,
    })),
    language: 0,
    hourlyPayBasis: 1,
    thresholdMinutesByDate: Object.fromEntries(
      [14, 21, 24, 31].map((day) => [`2026-08-${day}`, 480]),
    ),
    scheduledMinutesByDate: Object.fromEntries(
      [14, 21, 24, 31].map((day) => [`2026-08-${day}`, 480]),
    ),
    summary: {
      ...req.summary,
      workedHours: 154,
      regularHours: 136,
      overtimeHours: 18,
      ordinaryPaidHours: 168,
      compTimeEarnedHours: 18,
      compTimeUsedHours: 32,
      expectedHours: 168,
      monthlyDifferenceMinutes: -840,
      openingBalanceMinutes: 840,
      closingBalanceMinutes: 0,
    },
  });
  const augustReport = augustWorkbook.worksheets[0];
  const augustBalance = augustWorkbook.getWorksheet('Tidsbalans');
  if (
    augustReport.getCell('E37').value !== 168 ||
    augustReport.getCell('E38').value !== 18 ||
    augustReport.getCell('E39').value !== 32 ||
    augustBalance?.getCell('B5').value !== 154 ||
    augustBalance?.getCell('A7').value !== 'Uttagen komptid'
  ) {
    throw new Error('Comp-time payroll export did not preserve every payroll value');
  }
  console.log('✔ Comp-time payroll export verified');

  let invalidExportRejected = false;
  try {
    await ExcelExportService.exportToFile(
      {
        ...req,
        entries: [{ ...req.entries[0], date: '2026-09-17' }],
      },
      testExcelPath,
    );
  } catch {
    invalidExportRejected = true;
  }
  if (!invalidExportRejected) {
    throw new Error('Out-of-month report entry was not rejected');
  }
  let excessiveCompRejected = false;
  try {
    ExcelExportService.createWorkbook({
      ...req,
      entries: [{ ...req.entries[0], compTimeMinutes: 60 }],
    });
  } catch {
    excessiveCompRejected = true;
  }
  if (!excessiveCompRejected) {
    throw new Error('Export accepted comp time beyond the unworked scheduled time');
  }
  console.log('✔ Excel export validation verified');

  // Clean up
  db.close();
  fs.unlinkSync(tempDbPath);
  fs.unlinkSync(testExcelPath);
  fs.unlinkSync(testOdsPath);
  fs.rmSync(unrelatedPath, { force: true });
  for (const file of fs
    .readdirSync(__dirname)
    .filter((file) => file.startsWith('dagsverk-backup-') && file.endsWith('.db'))) {
    fs.rmSync(path.join(__dirname, file), { force: true });
  }

  console.log('--- ALL BACKEND TESTS PASSED SUCCESSFULLY ---');
}

runTests().catch((err) => {
  console.error('❌ Test failed:', err);
  process.exit(1);
});
