import { Component, inject, signal } from '@angular/core';
import { CommonModule } from '@angular/common';
import { NavigationEnd, Router } from '@angular/router';
import { filter } from 'rxjs/operators';
import { MatToolbarModule } from '@angular/material/toolbar';
import { MatButtonModule } from '@angular/material/button';
import { MatIconModule } from '@angular/material/icon';
import { MatButtonToggleModule } from '@angular/material/button-toggle';
import { MatTooltipModule } from '@angular/material/tooltip';
import { MatMenuModule } from '@angular/material/menu';
import { MatDividerModule } from '@angular/material/divider';
import { MatSnackBar, MatSnackBarModule } from '@angular/material/snack-bar';
import {
  MatDialog,
  MatDialogModule,
  MatDialogRef,
  MAT_DIALOG_DATA,
} from '@angular/material/dialog';
import { firstValueFrom } from 'rxjs';
import { AppStateService } from '../../core/app-state.service';
import { ElectronBridgeService } from '../../core/electron-bridge.service';
import { MonthViewPreference } from '../../core/models';
import { ConfirmDialogComponent } from '../../core/confirm-dialog.component';
import { LocalizationService } from '../../core/localization.service';

@Component({
  selector: 'app-report-preview-dialog',
  standalone: true,
  imports: [CommonModule, MatButtonModule, MatDialogModule, MatIconModule],
  template: `
    <h2 mat-dialog-title>Export monthly report</h2>
    <mat-dialog-content class="report-dialog-content">
      <p class="report-context">{{ data.workspaceName }} - {{ data.month }}</p>
      <p class="report-summary">{{ entrySummaryText }}</p>
      @if (data.missingCount > 0) {
        <p class="report-warning">
          <mat-icon aria-hidden="true">warning</mat-icon>
          <span>{{ data.missingCount }} past workdays are still unlogged.</span>
        </p>
      }
    </mat-dialog-content>
    <mat-dialog-actions align="end" class="report-dialog-actions">
      <button mat-button type="button" (click)="dialogRef.close()">Cancel</button>
      <button mat-button type="button" (click)="dialogRef.close('ods')">OpenDocument</button>
      <button mat-flat-button type="button" (click)="dialogRef.close('xlsx')">Excel</button>
    </mat-dialog-actions>
  `,
  styles: `
    .report-dialog-content {
      display: flex;
      flex-direction: column;
      gap: 4px;
      padding-top: 0;
    }

    .report-dialog-content p {
      margin: 0;
    }

    .report-context {
      color: var(--app-on-surface);
      font-weight: 500;
    }

    .report-summary,
    .report-warning {
      color: var(--app-on-surface-variant);
    }

    .report-warning {
      display: flex;
      align-items: center;
      gap: 8px;
      margin-top: 12px !important;
    }

    .report-warning mat-icon {
      flex: 0 0 20px;
      width: 20px;
      height: 20px;
      font-size: 20px;
    }

    .report-dialog-actions {
      gap: 8px;
      padding: 16px 24px 24px;
    }
  `,
})
export class ReportPreviewDialogComponent {
  public dialogRef = inject(MatDialogRef<ReportPreviewDialogComponent>);
  private localization = inject(LocalizationService);
  public data = inject<{
    workspaceName: string;
    month: string;
    entryCount: number;
    workedHours: number;
    missingCount: number;
  }>(MAT_DIALOG_DATA);

  public get entrySummaryText(): string {
    const lang = this.localization.language() === 'sv' ? 'sv-SE' : 'en-US';
    const hours = this.data.workedHours.toLocaleString(lang, {
      minimumFractionDigits: 2,
      maximumFractionDigits: 2,
    });
    return `${this.data.entryCount} saved entries and ${hours} worked hours.`;
  }
}

@Component({
  selector: 'app-header',
  standalone: true,
  imports: [
    CommonModule,
    MatToolbarModule,
    MatButtonModule,
    MatIconModule,
    MatButtonToggleModule,
    MatTooltipModule,
    MatMenuModule,
    MatDividerModule,
    MatDialogModule,
    MatSnackBarModule,
  ],
  templateUrl: './header.component.html',
  styleUrls: ['./header.component.scss'],
})
export class HeaderComponent {
  public state = inject(AppStateService);
  public bridge = inject(ElectronBridgeService);
  private router = inject(Router);
  private dialog = inject(MatDialog);
  private snackBar = inject(MatSnackBar);
  private localization = inject(LocalizationService);

  public readonly MonthViewPreference = MonthViewPreference;
  public currentRoute = signal<string>('/timesheet');

  public monthsList = [
    { num: 1, name: 'January' },
    { num: 2, name: 'February' },
    { num: 3, name: 'March' },
    { num: 4, name: 'April' },
    { num: 5, name: 'May' },
    { num: 6, name: 'June' },
    { num: 7, name: 'July' },
    { num: 8, name: 'August' },
    { num: 9, name: 'September' },
    { num: 10, name: 'October' },
    { num: 11, name: 'November' },
    { num: 12, name: 'December' },
  ];

  public constructor() {
    this.currentRoute.set(this.router.url);
    this.router.events
      .pipe(filter((event) => event instanceof NavigationEnd))
      .subscribe((event: any) => {
        const route = event.urlAfterRedirects || event.url;
        this.currentRoute.set(route);
        if (!route.startsWith('/timesheet') && this.state.isEditorOpen()) {
          this.state.closeCatchUp();
        }
      });
  }

  public onSelectMonth(monthNum: number): void {
    this.state.selectMonth(this.state.currentYear(), monthNum);
  }

  public onSelectYear(delta: number): void {
    this.state.selectMonth(this.state.currentYear() + delta, this.state.currentMonth());
  }

  public onMinimize(): void {
    this.bridge.minimize();
  }

  public onMaximize(): void {
    this.bridge.maximize();
  }

  public onClose(): void {
    this.bridge.close();
  }

  public onBackToSettings(): void {
    void this.router.navigate(['/settings']);
  }

  public async onExport(): Promise<void> {
    const format = await firstValueFrom(
      this.dialog
        .open(ReportPreviewDialogComponent, {
          width: '440px',
          autoFocus: false,
          data: {
            workspaceName: this.state.activeWorkspace().name,
            month: this.state.formattedMonthTitle(),
            entryCount: this.state.entries().length,
            workedHours: this.state.summary().workedHours,
            missingCount: this.state.missingDaysCount(),
          },
        })
        .afterClosed(),
    );
    if (format === 'xlsx' || format === 'ods') await this.state.exportReport(format);
  }

  public async onFillMonth(): Promise<void> {
    const count = this.state.fillableWorkdayCount();
    if (!count) {
      this.snackBar.open(
        this.localization.t('All scheduled workdays already have entries.'),
        'OK',
        {
          duration: 4000,
        },
      );
      return;
    }
    const confirmed = await this.confirm(
      'Fill normal workdays?',
      this.format(
        'Add your default hours to {0} empty scheduled workdays? Existing entries will be kept.',
        count,
      ),
      'Fill workdays',
    );
    if (!confirmed) return;
    const added = await this.state.fillNormalWorkdays();
    this.snackBar.open(this.format('{0} normal workdays added.', added), 'OK', { duration: 4000 });
  }

  public onCopyMonth(): void {
    this.state.copyMonth();
    this.snackBar.open(
      this.format(
        '{0} copied. Open another month and choose Paste month.',
        this.state.formattedMonthTitle(),
      ),
      'OK',
      { duration: 4000 },
    );
  }

  public async onPasteMonth(): Promise<void> {
    const count = this.state.pasteableEntryCount();
    if (!count) {
      this.snackBar.open(this.localization.t('There are no copied entries to paste.'), 'OK', {
        duration: 4000,
      });
      return;
    }
    const confirmed = await this.confirm(
      'Paste copied month?',
      this.format(
        'Add {0} entries from {1} by matching each weekday occurrence? Existing entries will be kept.',
        count,
        this.state.copiedMonthTitle(),
      ),
      'Paste entries',
    );
    if (!confirmed) return;
    const pasted = await this.state.pasteMonth();
    this.snackBar.open(this.format('{0} entries pasted.', pasted), 'OK', { duration: 4000 });
  }

  public async onResetMonth(): Promise<void> {
    const confirmed = await this.confirm(
      'Reset this month?',
      this.format(
        'Delete every entry and balance adjustment for {0}? This cannot be undone.',
        this.state.formattedMonthTitle(),
      ),
      'Reset month',
      true,
    );
    if (!confirmed) return;
    const month = this.state.formattedMonthTitle();
    await this.state.resetMonth();
    this.snackBar.open(this.format('{0} was reset.', month), 'OK', { duration: 4000 });
  }

  private async confirm(
    title: string,
    message: string,
    confirmLabel: string,
    destructive = false,
  ): Promise<boolean> {
    return Boolean(
      await firstValueFrom(
        this.dialog
          .open(ConfirmDialogComponent, {
            width: '440px',
            data: {
              title: this.localization.t(title),
              message,
              confirmLabel: this.localization.t(confirmLabel),
              destructive,
            },
          })
          .afterClosed(),
      ),
    );
  }

  private format(source: string, ...values: Array<string | number>): string {
    let result = this.localization.t(source);
    values.forEach((value, index) => (result = result.replace(`{${index}}`, String(value))));
    return result;
  }
}
