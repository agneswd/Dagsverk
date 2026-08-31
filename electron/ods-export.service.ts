import { promises as fs } from 'fs';
import JSZip from 'jszip';
import {
  compTimeUsedHours,
  hasOb,
  monthTitle,
  overtimeOrCompTimeHours,
  paidOrdinaryHours,
  ReportExportRequest,
  text,
  usesMonthlyHourlyPayBasis,
  validateReportRequest,
  workedMinutes,
} from './excel-export.service';

type Cell = { value?: string | number; style?: 'title' | 'heading' | 'bold' };

export class OdsExportService {
  public static async exportToFile(
    request: ReportExportRequest,
    outputPath: string,
  ): Promise<void> {
    validateReportRequest(request);
    const zip = new JSZip();
    zip.file('mimetype', 'application/vnd.oasis.opendocument.spreadsheet', {
      compression: 'STORE',
    });
    zip.file('content.xml', this.content(request));
    zip.file('styles.xml', stylesXml);
    zip.file('meta.xml', metaXml);
    zip.file('META-INF/manifest.xml', manifestXml);
    await fs.writeFile(outputPath, await zip.generateAsync({ type: 'nodebuffer' }));
  }

  private static content(request: ReportExportRequest): string {
    return `<?xml version="1.0" encoding="UTF-8"?>
<office:document-content xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0" xmlns:table="urn:oasis:names:tc:opendocument:xmlns:table:1.0" xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0" xmlns:style="urn:oasis:names:tc:opendocument:xmlns:style:1.0" xmlns:fo="urn:oasis:names:tc:opendocument:xmlns:xsl-fo-compatible:1.0" office:version="1.3">
  <office:automatic-styles>
    <style:style style:name="title" style:family="table-cell"><style:text-properties fo:font-size="16pt" fo:font-weight="bold"/></style:style>
    <style:style style:name="heading" style:family="table-cell"><style:table-cell-properties fo:background-color="#E3ECE7"/><style:text-properties fo:font-weight="bold"/></style:style>
    <style:style style:name="bold" style:family="table-cell"><style:text-properties fo:font-weight="bold"/></style:style>
  </office:automatic-styles>
  <office:body><office:spreadsheet>
    ${this.reportTable(request)}
    ${this.balanceTable(request)}
  </office:spreadsheet></office:body>
</office:document-content>`;
  }

  private static reportTable(request: ReportExportRequest): string {
    const rows: Cell[][] = [
      [{ value: text(request, 'Dagsverk - Time report', 'Dagsverk - Tidrapport'), style: 'title' }],
      [{ value: request.employeeName }, {}, {}, { value: request.employerName }],
      [],
      [
        { value: text(request, 'Day', 'Dag'), style: 'heading' },
        { value: 'Start', style: 'heading' },
        { value: text(request, 'Stop', 'Slut'), style: 'heading' },
        { value: 'Lunch', style: 'heading' },
        { value: text(request, 'Hours', 'Timmar'), style: 'heading' },
        { value: 'Status', style: 'heading' },
        { value: text(request, 'Project', 'Projekt'), style: 'heading' },
        { value: text(request, 'Comp used', 'Uttagen komp'), style: 'heading' },
      ],
    ];
    const entries = new Map(request.entries.map((entry) => [entry.date, entry]));
    const dayCount = new Date(Date.UTC(request.year, request.month, 0)).getUTCDate();
    for (let day = 1; day <= dayCount; day++) {
      const date = `${request.year}-${String(request.month).padStart(2, '0')}-${String(day).padStart(2, '0')}`;
      const entry = entries.get(date);
      if (entry?.status === 1) {
        rows.push([
          { value: day },
          { value: entry.startTime || '' },
          { value: entry.endTime || '' },
          { value: entry.lunchMinutes },
          { value: workedMinutes(entry) / 60 },
          {},
          { value: entry.projectName || '' },
          { value: entry.compTimeMinutes / 60 },
        ]);
      } else if (entry?.status === 2) {
        rows.push([
          { value: day },
          {},
          {},
          {},
          {},
          {
            value: entry.compTimeMinutes
              ? text(request, 'Comp time', 'Komptid')
              : text(request, 'Day off', 'Ledig'),
          },
          {},
          { value: entry.compTimeMinutes / 60 },
        ]);
      } else {
        rows.push([{ value: day }]);
      }
    }
    rows.push([]);
    rows.push([
      {},
      {},
      {},
      {
        value: usesMonthlyHourlyPayBasis(request)
          ? text(request, 'Total paid hours', 'Totalt betalda timmar')
          : text(request, 'Total regular hours', 'Totalt ordinarie timmar'),
        style: 'bold',
      },
      { value: paidOrdinaryHours(request), style: 'bold' },
    ]);
    rows.push([
      {},
      {},
      {},
      {
        value: usesMonthlyHourlyPayBasis(request)
          ? text(request, 'Comp time earned', 'Intjänad komptid')
          : text(request, 'Total overtime', 'Total övertid'),
        style: 'bold',
      },
      { value: overtimeOrCompTimeHours(request), style: 'bold' },
    ]);
    if (usesMonthlyHourlyPayBasis(request)) {
      rows.push([
        {},
        {},
        {},
        { value: text(request, 'Comp time used', 'Uttagen komptid'), style: 'bold' },
        { value: compTimeUsedHours(request), style: 'bold' },
      ]);
    }
    if (hasOb(request)) {
      rows.push([
        {},
        {},
        {},
        { value: text(request, 'Total OB hours', 'Totala OB-timmar'), style: 'bold' },
        { value: request.summary.obHours, style: 'bold' },
      ]);
    }
    return tableXml(monthTitle(request), rows);
  }

  private static balanceTable(request: ReportExportRequest): string {
    const rows: Cell[][] = [
      [
        {
          value: text(
            request,
            'Time balance - personal tracking',
            'Tidsbalans - personlig uppföljning',
          ),
          style: 'title',
        },
      ],
      [{ value: text(request, 'Month', 'Månad') }, { value: monthTitle(request) }],
      [],
      [
        {
          value: usesMonthlyHourlyPayBasis(request)
            ? text(request, 'Paid hours', 'Betalda timmar')
            : text(request, 'Regular hours', 'Ordinarie timmar'),
          style: 'bold',
        },
        { value: paidOrdinaryHours(request) },
      ],
    ];
    if (usesMonthlyHourlyPayBasis(request)) {
      rows.push(
        [
          { value: text(request, 'Worked hours', 'Arbetade timmar'), style: 'bold' },
          { value: request.summary.workedHours },
        ],
        [
          { value: text(request, 'Comp time earned', 'Intjänad komptid'), style: 'bold' },
          { value: overtimeOrCompTimeHours(request) },
        ],
        [
          { value: text(request, 'Comp time used', 'Uttagen komptid'), style: 'bold' },
          { value: compTimeUsedHours(request) },
        ],
      );
    } else {
      rows.push(
        [
          { value: text(request, 'Overtime', 'Övertid'), style: 'bold' },
          { value: overtimeOrCompTimeHours(request) },
        ],
        [
          { value: text(request, 'Worked hours', 'Arbetade timmar'), style: 'bold' },
          { value: request.summary.workedHours },
        ],
      );
    }
    if (hasOb(request)) {
      rows.push([
        { value: text(request, 'OB hours', 'OB-timmar'), style: 'bold' },
        { value: request.summary.obHours },
      ]);
    }
    rows.push(
      [
        { value: text(request, 'Expected hours', 'Förväntade timmar'), style: 'bold' },
        {
          value: request.summary.expectedHours,
        },
      ],
      [
        {
          value: text(request, 'Monthly time balance', 'Månadens tidsbalans'),
          style: 'bold',
        },
        { value: request.summary.monthlyDifferenceMinutes / 60 },
      ],
      [
        {
          value: text(request, 'Opening time balance', 'Ingående tidsbalans'),
          style: 'bold',
        },
        { value: request.summary.openingBalanceMinutes / 60 },
      ],
      [
        {
          value: text(request, 'Closing time balance', 'Utgående tidsbalans'),
          style: 'bold',
        },
        { value: request.summary.closingBalanceMinutes / 60 },
      ],
    );
    return tableXml(text(request, 'Time balance', 'Tidsbalans'), rows);
  }
}

function tableXml(name: string, rows: Cell[][]): string {
  return `<table:table table:name="${escapeXml(name)}">${rows
    .map((row) => `<table:table-row>${row.map(cellXml).join('')}</table:table-row>`)
    .join('')}</table:table>`;
}

function cellXml(cell: Cell): string {
  const style = cell.style ? ` table:style-name="${cell.style}"` : '';
  if (typeof cell.value === 'number') {
    return `<table:table-cell${style} office:value-type="float" office:value="${cell.value}"><text:p>${cell.value}</text:p></table:table-cell>`;
  }
  const value = escapeXml(cell.value ?? '');
  return `<table:table-cell${style} office:value-type="string"><text:p>${value}</text:p></table:table-cell>`;
}

function escapeXml(value: string): string {
  return value
    .replaceAll('&', '&amp;')
    .replaceAll('<', '&lt;')
    .replaceAll('>', '&gt;')
    .replaceAll('"', '&quot;')
    .replaceAll("'", '&apos;');
}

const manifestXml = `<?xml version="1.0" encoding="UTF-8"?>
<manifest:manifest xmlns:manifest="urn:oasis:names:tc:opendocument:xmlns:manifest:1.0" manifest:version="1.3">
  <manifest:file-entry manifest:full-path="/" manifest:media-type="application/vnd.oasis.opendocument.spreadsheet"/>
  <manifest:file-entry manifest:full-path="content.xml" manifest:media-type="text/xml"/>
  <manifest:file-entry manifest:full-path="styles.xml" manifest:media-type="text/xml"/>
  <manifest:file-entry manifest:full-path="meta.xml" manifest:media-type="text/xml"/>
</manifest:manifest>`;

const stylesXml = `<?xml version="1.0" encoding="UTF-8"?>
<office:document-styles xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0" office:version="1.3"><office:styles/></office:document-styles>`;

const metaXml = `<?xml version="1.0" encoding="UTF-8"?>
<office:document-meta xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0" xmlns:meta="urn:oasis:names:tc:opendocument:xmlns:meta:1.0" office:version="1.3"><office:meta><meta:generator>Dagsverk</meta:generator></office:meta></office:document-meta>`;
