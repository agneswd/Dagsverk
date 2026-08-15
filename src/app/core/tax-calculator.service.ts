import { Injectable, inject } from '@angular/core';
import { HttpClient } from '@angular/common/http';
import { firstValueFrom } from 'rxjs';
import { TaxEstimate, TaxMode, TaxSettings, TaxUnavailableReason } from './models';
import Decimal from 'decimal.js';

export interface TaxTableRange {
  TableNumber: number;
  LowerBound: number;
  UpperBound: number;
  AmountKind: string; // 'B' (fixed kronor) or '%' (percentage)
  Columns: number[];
}

export interface TaxTableFile {
  TaxYear: number;
  SourceFileName: string;
  SourceTitle: string;
  ImportedAt: string;
  Sha256: string;
  Ranges: TaxTableRange[];
}

@Injectable({
  providedIn: 'root',
})
export class TaxCalculatorService {
  private http = inject(HttpClient);
  private bundledYears = new Map<number, Map<number, TaxTableRange[]>>();
  private loadedYears = new Set<number>();

  public async loadTaxYear(year: number): Promise<boolean> {
    if (this.loadedYears.has(year)) {
      return true;
    }

    try {
      const file = await firstValueFrom(this.http.get<TaxTableFile>(`tax-data/tax-${year}.json`));

      const tableMap = new Map<number, TaxTableRange[]>();
      for (const range of file.Ranges) {
        if (!tableMap.has(range.TableNumber)) {
          tableMap.set(range.TableNumber, []);
        }
        tableMap.get(range.TableNumber)!.push(range);
      }

      // Sort each table by LowerBound for binary search
      for (const list of tableMap.values()) {
        list.sort((a, b) => a.LowerBound - b.LowerBound);
      }

      this.bundledYears.set(year, tableMap);
      this.loadedYears.add(year);
      return true;
    } catch {
      return false;
    }
  }

  public registerTaxData(year: number, file: TaxTableFile): void {
    const tableMap = new Map<number, TaxTableRange[]>();
    for (const range of file.Ranges) {
      if (!tableMap.has(range.TableNumber)) {
        tableMap.set(range.TableNumber, []);
      }
      tableMap.get(range.TableNumber)!.push(range);
    }
    for (const list of tableMap.values()) {
      list.sort((a, b) => a.LowerBound - b.LowerBound);
    }
    this.bundledYears.set(year, tableMap);
    this.loadedYears.add(year);
  }

  public calculate(grossPay: number, settings: TaxSettings): TaxEstimate {
    if (grossPay < 0) {
      grossPay = 0;
    }

    switch (settings.mode) {
      case TaxMode.Disabled:
        return {
          grossPay,
          preliminaryTax: 0,
          estimatedNetPay: grossPay,
          unavailableReason: 'None',
          isAvailable: true,
        };

      case TaxMode.SecondaryIncomeThirtyPercent: {
        const tax = Decimal.min(
          grossPay,
          new Decimal(grossPay).times('0.30').truncated(),
        ).toNumber();
        return {
          grossPay,
          preliminaryTax: tax,
          estimatedNetPay: new Decimal(grossPay).minus(tax).toNumber(),
          unavailableReason: 'None',
          isAvailable: true,
        };
      }

      case TaxMode.ManualMonthlyDeduction: {
        if (
          settings.manualMonthlyDeduction === null ||
          settings.manualMonthlyDeduction === undefined
        ) {
          return {
            grossPay,
            preliminaryTax: null,
            estimatedNetPay: null,
            unavailableReason: 'ManualDeductionNotConfigured',
            isAvailable: false,
          };
        }
        const deduction = Decimal.min(
          grossPay,
          Decimal.max(0, settings.manualMonthlyDeduction),
        ).toNumber();
        return {
          grossPay,
          preliminaryTax: deduction,
          estimatedNetPay: new Decimal(grossPay).minus(deduction).toNumber(),
          unavailableReason: 'None',
          isAvailable: true,
        };
      }

      case TaxMode.PrimaryIncomeTaxTable:
        return this.fromTable(grossPay, settings);

      default:
        return {
          grossPay,
          preliminaryTax: null,
          estimatedNetPay: null,
          unavailableReason: 'TaxYearNotBundled',
          isAvailable: false,
        };
    }
  }

  private fromTable(grossPay: number, settings: TaxSettings): TaxEstimate {
    if (!settings.taxYear || !this.bundledYears.has(settings.taxYear)) {
      return {
        grossPay,
        preliminaryTax: null,
        estimatedNetPay: null,
        unavailableReason: 'TaxYearNotBundled',
        isAvailable: false,
      };
    }

    if (grossPay <= 0) {
      return {
        grossPay,
        preliminaryTax: 0,
        estimatedNetPay: 0,
        unavailableReason: 'None',
        isAvailable: true,
      };
    }

    const tableMap = this.bundledYears.get(settings.taxYear)!;
    const ranges = tableMap.get(settings.tableNumber);
    if (!ranges || ranges.length === 0) {
      return {
        grossPay,
        preliminaryTax: null,
        estimatedNetPay: null,
        unavailableReason: 'TaxYearNotBundled',
        isAvailable: false,
      };
    }

    const wholeKrona = Math.floor(grossPay);
    const range = this.binarySearchRange(ranges, wholeKrona);
    if (!range) {
      return {
        grossPay,
        preliminaryTax: null,
        estimatedNetPay: null,
        unavailableReason: 'TaxYearNotBundled',
        isAvailable: false,
      };
    }

    const colIndex = Math.max(0, Math.min(5, settings.column - 1));
    const rawVal = range.Columns[colIndex] ?? 0;
    const preliminaryTax =
      range.AmountKind === '%'
        ? new Decimal(wholeKrona).times(rawVal).dividedBy(100).floor().toNumber()
        : rawVal;

    const clampedTax = Math.min(grossPay, Math.max(0, preliminaryTax));
    return {
      grossPay,
      preliminaryTax: clampedTax,
      estimatedNetPay: new Decimal(grossPay).minus(clampedTax).toNumber(),
      unavailableReason: 'None',
      isAvailable: true,
    };
  }

  private binarySearchRange(ranges: TaxTableRange[], wholeKrona: number): TaxTableRange | null {
    let low = 0;
    let high = ranges.length - 1;

    while (low <= high) {
      const mid = Math.floor((low + high) / 2);
      const r = ranges[mid];
      if (wholeKrona < r.LowerBound) {
        high = mid - 1;
      } else if (wholeKrona > r.UpperBound) {
        low = mid + 1;
      } else {
        return r;
      }
    }
    return null;
  }
}
