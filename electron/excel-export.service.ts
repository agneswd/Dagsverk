import * as ExcelJS from 'exceljs';

export interface ReportExportRequest {
  year: number;
  month: number;
  employeeName: string;
  employerName: string;
  entries: any[];
  summary: any;
  language: number; // 0=Swedish, 1=English, 2=System
  overtimeMode: number; // 0=CompTime, 1=Paid
  dailyOvertimeThresholdHours: number;
}

export class ExcelExportService {
  public static async exportToFile(request: ReportExportRequest, outputPath: string): Promise<void> {
    const workbook = new ExcelJS.Workbook();
    workbook.creator = 'Dagsverk';
    workbook.created = new Date();

    const isEnglish = request.language === 1;
    const culture = isEnglish ? 'en-US' : 'sv-SE';
    const monthDate = new Date(Date.UTC(request.year, request.month - 1, 1));
    const monthTitle = monthDate.toLocaleDateString(culture, { month: 'long', year: 'numeric' });

    // --- Sheet 1: Month Time Report ---
    const sheet = workbook.addWorksheet(monthTitle, {
      pageSetup: { orientation: 'portrait' }
    });

    const text = (en: string, sv: string) => (isEnglish ? en : sv);

    // Title & Metadata
    sheet.getCell('A1').value = text('Dagsverk - Time report', 'Dagsverk - Tidrapport');
    sheet.getCell('A1').font = { bold: true, size: 16 };
    sheet.mergeCells('A1:G1');

    sheet.getCell('A2').value = request.employeeName;
    sheet.getCell('D2').value = request.employerName;

    // Table Header (Row 4)
    const headerRow = 4;
    sheet.getCell(headerRow, 1).value = text('Day', 'Dag');
    sheet.getCell(headerRow, 2).value = 'Start';
    sheet.getCell(headerRow, 3).value = text('Stop', 'Slut');
    sheet.getCell(headerRow, 4).value = 'Lunch';
    sheet.getCell(headerRow, 5).value = text('Hours', 'Timmar');
    sheet.getCell(headerRow, 6).value = 'Status';
    sheet.getCell(headerRow, 7).value = text('Project', 'Projekt');

    const headerFill: ExcelJS.Fill = {
      type: 'pattern',
      pattern: 'solid',
      fgColor: { argb: 'FFE3ECE7' }
    };

    for (let c = 1; c <= 7; c++) {
      const cell = sheet.getCell(headerRow, c);
      cell.font = { bold: true };
      cell.fill = headerFill;
    }

    const daysInMonth = new Date(Date.UTC(request.year, request.month, 0)).getUTCDate();
    const entriesMap = new Map<string, any>();
    for (const e of request.entries) {
      entriesMap.set(e.date, e);
    }

    const firstDayRow = headerRow + 1;
    for (let day = 1; day <= daysInMonth; day++) {
      const row = headerRow + day;
      const dateStr = `${request.year}-${String(request.month).padStart(2, '0')}-${String(day).padStart(2, '0')}`;
      const entry = entriesMap.get(dateStr);

      sheet.getCell(row, 1).value = day;

      if (entry) {
        if (entry.status === 1 && entry.startTime && entry.endTime) { // Worked
          // Store time as fraction of day for Excel time arithmetic
          const startMin = this.timeToMinutes(entry.startTime);
          const stopMin = this.timeToMinutes(entry.endTime);
          const lunchMin = entry.lunchMinutes || 0;

          sheet.getCell(row, 2).value = startMin / 1440;
          sheet.getCell(row, 2).numFmt = 'hh:mm';

          sheet.getCell(row, 3).value = stopMin / 1440;
          sheet.getCell(row, 3).numFmt = 'hh:mm';

          sheet.getCell(row, 4).value = lunchMin / 1440;
          sheet.getCell(row, 4).numFmt = 'hh:mm';

          sheet.getCell(row, 5).value = {
            formula: `IF(OR(B${row}="",C${row}=""),"",MAX(0,(C${row}-B${row}-D${row})*24))`
          };
          sheet.getCell(row, 5).numFmt = '0.00';

          const threshold = entry.scheduledMinutesOverride !== null && entry.scheduledMinutesOverride !== undefined
            ? entry.scheduledMinutesOverride / 60
            : (request.dailyOvertimeThresholdHours || 8);

          sheet.getCell(row, 8).value = {
            formula: `IF(OR(B${row}="",C${row}=""),"",MAX(0,(C${row}-B${row}-D${row})*24-${threshold}))`
          };
          sheet.getCell(row, 8).numFmt = '0.00';

          sheet.getCell(row, 7).value = entry.projectName || '';
        } else if (entry.status === 2) { // Off
          sheet.getCell(row, 6).value = text('Day off', 'Ledig');
        }
      }

      // Add bottom border
      for (let c = 1; c <= 7; c++) {
        sheet.getCell(row, c).border = {
          bottom: { style: 'thin', color: { argb: 'FFD9DAD3' } }
        };
      }
    }

    const lastDayRow = headerRow + daysInMonth;
    const totalsRow = daysInMonth + 6;

    sheet.getCell(totalsRow, 4).value = text('Total regular hours', 'Totalt ordinarie timmar');
    sheet.getCell(totalsRow, 4).font = { bold: true };
    sheet.getCell(totalsRow, 5).value = {
      formula: `SUM(E${firstDayRow}:E${lastDayRow})-SUM(H${firstDayRow}:H${lastDayRow})`
    };
    sheet.getCell(totalsRow, 5).font = { bold: true };
    sheet.getCell(totalsRow, 5).numFmt = '0.00';

    sheet.getCell(totalsRow + 1, 4).value = text('Total overtime', 'Total övertid');
    sheet.getCell(totalsRow + 1, 4).font = { bold: true };
    sheet.getCell(totalsRow + 1, 5).value = {
      formula: `SUM(H${firstDayRow}:H${lastDayRow})`
    };
    sheet.getCell(totalsRow + 1, 5).font = { bold: true };
    sheet.getCell(totalsRow + 1, 5).numFmt = '0.00';

    sheet.getCell(totalsRow + 2, 4).value = text('Total OB hours', 'Totala OB-timmar');
    sheet.getCell(totalsRow + 2, 4).font = { bold: true };
    sheet.getCell(totalsRow + 2, 5).value = request.summary.obHours || 0;
    sheet.getCell(totalsRow + 2, 5).font = { bold: true };
    sheet.getCell(totalsRow + 2, 5).numFmt = '0.00';

    // Format Column widths & hidden overtime column
    sheet.getColumn(1).width = 8;
    sheet.getColumn(2).width = 12;
    sheet.getColumn(3).width = 12;
    sheet.getColumn(4).width = 25;
    sheet.getColumn(5).width = 18;
    sheet.getColumn(6).width = 14;
    sheet.getColumn(7).width = 24;
    sheet.getColumn(8).hidden = true;

    // --- Sheet 2: Time balance ---
    const balanceSheetName = text('Time balance', 'Tidsbalans');
    const balanceSheet = workbook.addWorksheet(balanceSheetName);
    const escapedSheetName = `'${monthTitle.replace(/'/g, "''")}'`;

    balanceSheet.getCell('A1').value = text('Time balance - personal tracking', 'Tidsbalans - personlig uppföljning');
    balanceSheet.getCell('A1').font = { bold: true, size: 16 };
    balanceSheet.mergeCells('A1:B1');

    balanceSheet.getCell('A2').value = text('Month', 'Månad');
    balanceSheet.getCell('B2').value = monthTitle;

    balanceSheet.getCell('A4').value = text('Regular hours', 'Ordinarie timmar');
    balanceSheet.getCell('B4').value = { formula: `${escapedSheetName}!E${totalsRow}` };

    balanceSheet.getCell('A5').value = text('Overtime', 'Övertid');
    balanceSheet.getCell('B5').value = { formula: `${escapedSheetName}!E${totalsRow + 1}` };

    balanceSheet.getCell('A6').value = text('Worked hours', 'Arbetade timmar');
    balanceSheet.getCell('B6').value = { formula: 'B4+B5' };

    balanceSheet.getCell('A7').value = text('OB hours', 'OB-timmar');
    balanceSheet.getCell('B7').value = request.summary.obHours || 0;

    balanceSheet.getCell('A8').value = text('Expected hours', 'Förväntade timmar');
    balanceSheet.getCell('B8').value = request.summary.expectedHours || 0;

    balanceSheet.getCell('A9').value = text('Monthly time balance', 'Månadens tidsbalans');
    balanceSheet.getCell('B9').value = {
      formula: request.overtimeMode === 0 ? 'B6-B8' : 'B4-B8'
    };

    balanceSheet.getCell('A10').value = text('Opening time balance', 'Ingående tidsbalans');
    balanceSheet.getCell('B10').value = (request.summary.openingBalanceMinutes || 0) / 60;

    balanceSheet.getCell('A11').value = text('Closing time balance', 'Utgående tidsbalans');
    balanceSheet.getCell('B11').value = { formula: 'B9+B10' };

    for (let r = 4; r <= 11; r++) {
      balanceSheet.getCell(`A${r}`).font = { bold: true };
      balanceSheet.getCell(`B${r}`).numFmt = '0.00';
    }

    balanceSheet.getColumn('A').width = 34;
    balanceSheet.getColumn('B').width = 18;

    await workbook.xlsx.writeFile(outputPath);
  }

  private static timeToMinutes(timeStr: string): number {
    const [h, m] = timeStr.split(':').map(Number);
    return h * 60 + m;
  }
}
