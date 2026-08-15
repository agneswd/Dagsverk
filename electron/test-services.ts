import { DatabaseService } from './database.service';
import { ExcelExportService, ReportExportRequest } from './excel-export.service';
import { OdsExportService } from './ods-export.service';
import JSZip from 'jszip';
import * as path from 'path';
import * as fs from 'fs';
import * as ExcelJS from 'exceljs';
import Database from 'better-sqlite3';

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
  });

  const entries = db.getWorkEntries(2026, 8);
  if (entries.length !== 1 || entries[0].startTime !== '08:00') {
    throw new Error('WorkEntry save/get failed');
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

  // Test Backup
  const backupFile = await db.createBackup(__dirname);
  if (!fs.existsSync(backupFile)) {
    throw new Error('Backup creation failed');
  }
  console.log('✔ SQLite Backup creation verified:', backupFile);

  db.saveWorkEntry({
    date: '2026-08-18',
    status: 1,
    startTime: '08:00',
    endTime: '16:30',
    lunchMinutes: 30,
    projectName: 'General',
    notes: 'Created after backup',
    scheduledMinutesOverride: null,
  });
  await db.restoreBackup(backupFile);
  if (db.getWorkEntries(2026, 8).some((entry) => entry.date === '2026-08-18')) {
    throw new Error('Valid backup did not replace the current database');
  }
  console.log('✔ Valid backup restore verified');

  const unrelatedPath = path.join(__dirname, 'unrelated.db');
  fs.rmSync(unrelatedPath, { force: true });
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
      },
    ],
    summary: {
      workedHours: 8.5,
      regularHours: 8,
      overtimeHours: 0.5,
      ordinaryPaidHours: 8,
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
  if (sheet1.getCell('D39').value !== null || sheet2.getCell('A7').value !== null) {
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
    monthlyBalance?.getCell('A5').value !== 'Intjänad komptid'
  ) {
    throw new Error('Monthly hourly export did not match Tidverk');
  }
  console.log('✔ Monthly hourly export parity verified');

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
  console.log('✔ Excel export validation verified');

  // Clean up
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
