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

@Component({
  selector: 'app-report-preview-dialog',
  standalone: true,
  imports: [CommonModule, MatButtonModule, MatDialogModule, MatIconModule],
  template: `
    <h2 mat-dialog-title>Export monthly report</h2>
    <mat-dialog-content class="report-dialog-content">
      <p class="report-context">{{ data.workspaceName }} - {{ data.month }}</p>
      <p class="report-summary">
        {{ data.entryCount }} saved entries and {{ data.workedHours | number: '1.2-2' }} worked
        hours.
      </p>
      @if (data.missingCount > 0) {
        <p class="report-warning">
          <mat-icon aria-hidden="true">warning</mat-icon>
          <span>{{ data.missingCount }} past workdays are still unlogged.</span>
        </p>
      }
    </mat-dialog-content>
    <mat-dialog-actions align="end" class="report-dialog-actions">
      <button mat-button type="button" (click)="dialogRef.close(false)">Cancel</button>
      <button mat-flat-button type="button" (click)="dialogRef.close(true)">Choose file</button>
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
  public data = inject<{
    workspaceName: string;
    month: string;
    entryCount: number;
    workedHours: number;
    missingCount: number;
  }>(MAT_DIALOG_DATA);
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
  ],
  templateUrl: './header.component.html',
  styleUrls: ['./header.component.scss'],
})
export class HeaderComponent {
  public state = inject(AppStateService);
  public bridge = inject(ElectronBridgeService);
  private router = inject(Router);
  private dialog = inject(MatDialog);

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

  public async onExport(): Promise<void> {
    const confirmed = await firstValueFrom(
      this.dialog
        .open(ReportPreviewDialogComponent, {
          width: '440px',
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
    if (confirmed) await this.state.exportExcel();
  }
}
