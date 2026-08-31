import * as ExcelJS from 'exceljs';

export interface ReportExportRequest {
  year: number;
  month: number;
  employeeName: string;
  employerName: string;
  entries: ReportEntry[];
  summary: ReportSummary;
  language: number; // 0=Swedish, 1=English, 2=System
  overtimeMode: number; // 0=CompTime, 1=Paid
  dailyOvertimeThresholdHours: number;
  hourlyPayBasis?: number; // 0=DailyRegularHours, 1=MonthlyExpectedHours
  thresholdMinutesByDate?: Record<string, number>;
  scheduledMinutesByDate?: Record<string, number>;
  overtimeSettings?: { rateBands?: Array<{ compensationType: number }> };
}

export interface ReportEntry {
  date: string;
  status: number;
  startTime: string | null;
  endTime: string | null;
  lunchMinutes: number;
  projectName: string | null;
  dayOffReason?: string | null;
  notes?: string | null;
  scheduledMinutesOverride: number | null;
  compTimeMinutes: number;
}

export interface ReportSummary {
  workedHours: number;
  regularHours: number;
  overtimeHours: number;
  ordinaryPaidHours?: number;
  compTimeEarnedHours?: number;
  compTimeUsedHours?: number;
  obHours: number;
  expectedHours: number;
  monthlyDifferenceMinutes: number;
  openingBalanceMinutes: number;
  closingBalanceMinutes: number;
}

export class ExcelExportService {
  public static async exportToFile(
    request: ReportExportRequest,
    outputPath: string,
  ): Promise<void> {
    validateReportRequest(request);
    const workbook = this.createWorkbook(request);
    await workbook.xlsx.writeFile(outputPath);
  }

  public static createWorkbook(request: ReportExportRequest): ExcelJS.Workbook {
    validateReportRequest(request);
    const workbook = new ExcelJS.Workbook();
    workbook.creator = 'Dagsverk';
    workbook.lastModifiedBy = 'Dagsverk';
    workbook.created = new Date();
    workbook.calcProperties.fullCalcOnLoad = true;

    const report = workbook.addWorksheet(monthTitle(request), {
      pageSetup: { orientation: 'portrait' },
      views: [{ state: 'frozen', ySplit: 4 }],
    });
    const headerRow = 4;
    const firstDayRow = 5;
    const dayCount = new Date(Date.UTC(request.year, request.month, 0)).getUTCDate();
    const lastDayRow = headerRow + dayCount;
    const totalsRow = dayCount + 6;

    report.getCell('A1').value = text(request, 'Dagsverk - Time report', 'Dagsverk - Tidrapport');
    report.getCell('A1').font = { bold: true, size: 16 };
    report.mergeCells('A1:H1');
    report.getCell('A2').value = request.employeeName;
    report.getCell('D2').value = request.employerName;

    const headings = [
      text(request, 'Day', 'Dag'),
      'Start',
      text(request, 'Stop', 'Slut'),
      'Lunch',
      text(request, 'Hours', 'Timmar'),
      'Status',
      text(request, 'Project', 'Projekt'),
      text(request, 'Comp used', 'Uttagen komp'),
    ];
    for (let column = 1; column <= headings.length; column++) {
      const cell = report.getCell(headerRow, column);
      cell.value = headings[column - 1];
      cell.font = { bold: true };
      cell.fill = {
        type: 'pattern',
        pattern: 'solid',
        fgColor: { argb: 'FFE3ECE7' },
      };
    }

    const entries = new Map(request.entries.map((entry) => [entry.date, entry]));
    for (let day = 1; day <= dayCount; day++) {
      const row = headerRow + day;
      const date = `${request.year}-${String(request.month).padStart(2, '0')}-${String(day).padStart(2, '0')}`;
      const entry = entries.get(date);
      report.getCell(row, 1).value = day;

      if (entry?.status === 1 && entry.startTime && entry.endTime) {
        const startMinutes = timeToMinutes(entry.startTime);
        const endMinutes = timeToMinutes(entry.endTime);
        report.getCell(row, 2).value = startMinutes / 1440;
        report.getCell(row, 3).value = endMinutes / 1440;
        report.getCell(row, 4).value = entry.lunchMinutes / 1440;
        for (let column = 2; column <= 4; column++) report.getCell(row, column).numFmt = 'hh:mm';

        const workedHours = workedMinutes(entry) / 60;
        report.getCell(row, 5).value = {
          formula: `IF(OR(B${row}="",C${row}=""),"",MAX(0,(MOD(C${row}-B${row},1)-D${row})*24))`,
          result: workedHours,
        };
        const thresholdHours = thresholdMinutes(request, entry) / 60;
        report.getCell(row, 9).value = {
          formula: `IF(OR(B${row}="",C${row}=""),"",MAX(0,(MOD(C${row}-B${row},1)-D${row})*24-${thresholdHours}))`,
          result: Math.max(0, workedHours - thresholdHours),
        };
        report.getCell(row, 7).value = entry.projectName || '';
      } else if (entry?.status === 2) {
        report.getCell(row, 6).value = entry.compTimeMinutes
          ? text(request, 'Comp time', 'Komptid')
          : text(request, 'Day off', 'Ledig');
      }
      if (entry?.compTimeMinutes) report.getCell(row, 8).value = entry.compTimeMinutes / 60;

      for (let column = 1; column <= 8; column++) {
        report.getCell(row, column).border = {
          bottom: { style: 'thin', color: { argb: 'FFD9DAD3' } },
        };
      }
    }

    if (usesMonthlyHourlyPayBasis(request)) {
      report.getCell(totalsRow, 4).value = text(
        request,
        'Total paid hours',
        'Totalt betalda timmar',
      );
      report.getCell(totalsRow, 5).value = paidOrdinaryHours(request);
      report.getCell(totalsRow + 1, 4).value = text(
        request,
        'Comp time earned',
        'Intjänad komptid',
      );
      report.getCell(totalsRow + 1, 5).value = overtimeOrCompTimeHours(request);
      report.getCell(totalsRow + 2, 4).value = text(request, 'Comp time used', 'Uttagen komptid');
      report.getCell(totalsRow + 2, 5).value = compTimeUsedHours(request);
    } else {
      report.getCell(totalsRow, 4).value = text(
        request,
        'Total regular hours',
        'Totalt ordinarie timmar',
      );
      report.getCell(totalsRow, 5).value = {
        formula: `SUM(E${firstDayRow}:E${lastDayRow})-SUM(I${firstDayRow}:I${lastDayRow})`,
        result: request.summary.regularHours,
      };
      report.getCell(totalsRow + 1, 4).value = text(request, 'Total overtime', 'Total övertid');
      report.getCell(totalsRow + 1, 5).value = {
        formula: `SUM(I${firstDayRow}:I${lastDayRow})`,
        result: request.summary.overtimeHours,
      };
    }

    if (hasOb(request)) {
      const obRow = totalsRow + (usesMonthlyHourlyPayBasis(request) ? 3 : 2);
      report.getCell(obRow, 4).value = text(request, 'Total OB hours', 'Totala OB-timmar');
      report.getCell(obRow, 5).value = request.summary.obHours;
    }
    for (let row = totalsRow; row <= totalsRow + 3; row++) {
      report.getCell(row, 4).font = { bold: true };
      report.getCell(row, 5).font = { bold: true };
    }

    for (let row = firstDayRow; row <= totalsRow + 3; row++) {
      report.getCell(row, 5).numFmt = '0.00';
      report.getCell(row, 8).numFmt = '0.00';
      report.getCell(row, 9).numFmt = '0.00';
    }
    [8, 12, 12, 25, 18, 14, 24, 16].forEach((width, index) => {
      report.getColumn(index + 1).width = width;
    });
    report.getColumn(9).hidden = true;

    this.addBalanceSheet(workbook, request, report.name, totalsRow);
    return workbook;
  }

  private static addBalanceSheet(
    workbook: ExcelJS.Workbook,
    request: ReportExportRequest,
    reportSheetName: string,
    totalsRow: number,
  ): void {
    const balance = workbook.addWorksheet(text(request, 'Time balance', 'Tidsbalans'));
    const sheetReference = `'${reportSheetName.replace(/'/g, "''")}'`;
    balance.getCell('A1').value = text(
      request,
      'Time balance - personal tracking',
      'Tidsbalans - personlig uppföljning',
    );
    balance.getCell('A1').font = { bold: true, size: 16 };
    balance.mergeCells('A1:B1');
    balance.getCell('A2').value = text(request, 'Month', 'Månad');
    balance.getCell('B2').value = monthTitle(request);
    const monthly = usesMonthlyHourlyPayBasis(request);
    balance.getCell('A4').value = monthly
      ? text(request, 'Paid hours', 'Betalda timmar')
      : text(request, 'Regular hours', 'Ordinarie timmar');
    balance.getCell('B4').value = {
      formula: `${sheetReference}!E${totalsRow}`,
      result: paidOrdinaryHours(request),
    };
    let row = 5;
    let workedRow: number;
    if (monthly) {
      workedRow = row;
      balance.getCell(row, 1).value = text(request, 'Worked hours', 'Arbetade timmar');
      balance.getCell(row++, 2).value = request.summary.workedHours;
      balance.getCell(row, 1).value = text(request, 'Comp time earned', 'Intjänad komptid');
      balance.getCell(row++, 2).value = {
        formula: `${sheetReference}!E${totalsRow + 1}`,
        result: overtimeOrCompTimeHours(request),
      };
      balance.getCell(row, 1).value = text(request, 'Comp time used', 'Uttagen komptid');
      balance.getCell(row++, 2).value = {
        formula: `${sheetReference}!E${totalsRow + 2}`,
        result: compTimeUsedHours(request),
      };
    } else {
      balance.getCell(row, 1).value = text(request, 'Overtime', 'Övertid');
      balance.getCell(row++, 2).value = {
        formula: `${sheetReference}!E${totalsRow + 1}`,
        result: overtimeOrCompTimeHours(request),
      };
      workedRow = row;
      balance.getCell(row, 1).value = text(request, 'Worked hours', 'Arbetade timmar');
      balance.getCell(row++, 2).value = {
        formula: 'B4+B5',
        result: request.summary.workedHours,
      };
    }
    if (hasOb(request)) {
      balance.getCell(row, 1).value = text(request, 'OB hours', 'OB-timmar');
      balance.getCell(row++, 2).value = request.summary.obHours;
    }
    const expectedRow = row;
    balance.getCell(row, 1).value = text(request, 'Expected hours', 'Förväntade timmar');
    balance.getCell(row++, 2).value = request.summary.expectedHours;
    const differenceRow = row;
    balance.getCell(row, 1).value = text(request, 'Monthly time balance', 'Månadens tidsbalans');
    balance.getCell(row++, 2).value = {
      formula: `B${request.overtimeMode === 0 ? workedRow : 4}-B${expectedRow}`,
      result: request.summary.monthlyDifferenceMinutes / 60,
    };
    const openingRow = row;
    balance.getCell(row, 1).value = text(request, 'Opening time balance', 'Ingående tidsbalans');
    balance.getCell(row++, 2).value = request.summary.openingBalanceMinutes / 60;
    balance.getCell(row, 1).value = text(request, 'Closing time balance', 'Utgående tidsbalans');
    balance.getCell(row, 2).value = {
      formula: `B${differenceRow}+B${openingRow}`,
      result: request.summary.closingBalanceMinutes / 60,
    };
    for (let formatRow = 4; formatRow <= row; formatRow++) {
      balance.getCell(formatRow, 1).font = { bold: true };
      balance.getCell(formatRow, 2).numFmt = '0.00';
    }
    balance.getColumn(1).width = 34;
    balance.getColumn(2).width = 18;
  }
}

export function validateReportRequest(request: ReportExportRequest): void {
  if (!Number.isInteger(request.year) || request.month < 1 || request.month > 12) {
    throw new Error('The report month is invalid.');
  }
  const monthPrefix = `${request.year}-${String(request.month).padStart(2, '0')}-`;
  const dates = new Set<string>();
  for (const entry of request.entries) {
    if (!entry.date.startsWith(monthPrefix)) {
      throw new Error(`${entry.date} is outside the selected month.`);
    }
    if (dates.has(entry.date))
      throw new Error(`The report contains duplicate entries for ${entry.date}.`);
    dates.add(entry.date);
    if (entry.status === 1) {
      timeToMinutes(entry.startTime || '');
      timeToMinutes(entry.endTime || '');
      if (!Number.isInteger(entry.lunchMinutes) || entry.lunchMinutes < 0) {
        throw new Error(`The lunch duration for ${entry.date} is invalid.`);
      }
    }
    if (!Number.isInteger(entry.compTimeMinutes) || entry.compTimeMinutes < 0) {
      throw new Error(`The comp time used for ${entry.date} is invalid.`);
    }
    const scheduled =
      request.scheduledMinutesByDate?.[entry.date] ?? thresholdMinutes(request, entry);
    const available = Math.max(
      0,
      scheduled - Math.min(entry.status === 1 ? workedMinutes(entry) : 0, scheduled),
    );
    if (entry.compTimeMinutes > available) {
      throw new Error(`The comp time used for ${entry.date} exceeds the available hours.`);
    }
  }
}

export function monthTitle(request: ReportExportRequest): string {
  const locale = isEnglish(request) ? 'en-US' : 'sv-SE';
  return new Date(Date.UTC(request.year, request.month - 1, 1)).toLocaleDateString(locale, {
    month: 'long',
    year: 'numeric',
    timeZone: 'UTC',
  });
}

export function text(request: ReportExportRequest, english: string, swedish: string): string {
  return isEnglish(request) ? english : swedish;
}

export function usesMonthlyHourlyPayBasis(request: ReportExportRequest): boolean {
  return request.overtimeMode === 0 && request.hourlyPayBasis === 1;
}

export function paidOrdinaryHours(request: ReportExportRequest): number {
  return usesMonthlyHourlyPayBasis(request)
    ? (request.summary.ordinaryPaidHours ?? request.summary.regularHours)
    : request.summary.regularHours;
}

export function overtimeOrCompTimeHours(request: ReportExportRequest): number {
  return usesMonthlyHourlyPayBasis(request)
    ? (request.summary.compTimeEarnedHours ??
        Math.max(0, request.summary.workedHours - paidOrdinaryHours(request)))
    : request.summary.overtimeHours;
}

export function compTimeUsedHours(request: ReportExportRequest): number {
  return usesMonthlyHourlyPayBasis(request) ? (request.summary.compTimeUsedHours ?? 0) : 0;
}

export function hasOb(request: ReportExportRequest): boolean {
  return (
    request.summary.obHours !== 0 ||
    request.overtimeSettings?.rateBands?.some((band) => band.compensationType === 1) === true
  );
}

export function workedMinutes(entry: ReportEntry): number {
  if (!entry.startTime || !entry.endTime || entry.startTime === entry.endTime) return 0;
  const start = timeToMinutes(entry.startTime);
  const end = timeToMinutes(entry.endTime);
  const elapsed = end > start ? end - start : end - start + 1440;
  return Math.max(0, elapsed - entry.lunchMinutes);
}

function thresholdMinutes(request: ReportExportRequest, entry: ReportEntry): number {
  return (
    request.thresholdMinutesByDate?.[entry.date] ??
    entry.scheduledMinutesOverride ??
    Math.round(request.dailyOvertimeThresholdHours * 60)
  );
}

function timeToMinutes(time: string): number {
  if (!/^(?:[01]\d|2[0-3]):[0-5]\d$/.test(time)) throw new Error(`Invalid time value: ${time}`);
  const [hours, minutes] = time.split(':').map(Number);
  return hours * 60 + minutes;
}

function isEnglish(request: ReportExportRequest): boolean {
  return (
    request.language === 1 || (request.language === 2 && !process.env['LANG']?.startsWith('sv'))
  );
}
