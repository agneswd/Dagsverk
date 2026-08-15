import { Component, inject } from '@angular/core';
import { CommonModule } from '@angular/common';
import { MatIconModule } from '@angular/material/icon';
import { FormsModule } from '@angular/forms';
import { MatButtonModule } from '@angular/material/button';
import {
  MatDialog,
  MatDialogModule,
  MatDialogRef,
  MAT_DIALOG_DATA,
} from '@angular/material/dialog';
import { MatFormFieldModule } from '@angular/material/form-field';
import { MatInputModule } from '@angular/material/input';
import { AppStateService } from '../../../core/app-state.service';
import { OvertimeCompensationMode } from '../../../core/models';

@Component({
  selector: 'app-balance-dialog',
  standalone: true,
  imports: [FormsModule, MatButtonModule, MatDialogModule, MatFormFieldModule, MatInputModule],
  template: `
    <h2 mat-dialog-title>Adjust opening balance</h2>
    <mat-dialog-content class="balance-dialog-content">
      <p>Set the balance carried into {{ data.month }}.</p>
      <mat-form-field appearance="outline" subscriptSizing="dynamic">
        <mat-label>Opening balance</mat-label>
        <input matInput type="number" step="0.25" [(ngModel)]="hours" />
        <span matTextSuffix>hours</span>
      </mat-form-field>
    </mat-dialog-content>
    <mat-dialog-actions align="end" class="balance-dialog-actions">
      <button mat-button type="button" (click)="dialogRef.close()">Cancel</button>
      <button mat-flat-button type="button" (click)="save()">Save balance</button>
    </mat-dialog-actions>
  `,
  styles: `
    .balance-dialog-content {
      display: flex;
      flex-direction: column;
      gap: 16px;
      padding-top: 0;
    }

    .balance-dialog-content p {
      margin: 0;
      color: var(--app-on-surface-variant);
    }

    .balance-dialog-content mat-form-field {
      width: 100%;
    }

    .balance-dialog-actions {
      gap: 8px;
      padding: 16px 24px 24px;
    }
  `,
})
export class BalanceDialogComponent {
  public dialogRef = inject(MatDialogRef<BalanceDialogComponent>);
  public data = inject<{ month: string; minutes: number }>(MAT_DIALOG_DATA);
  public hours = this.data.minutes / 60;
  public save(): void {
    this.dialogRef.close(Math.round(this.hours * 60));
  }
}

@Component({
  selector: 'app-summary-cards',
  standalone: true,
  imports: [CommonModule, MatIconModule, MatButtonModule, MatDialogModule],
  templateUrl: './summary-cards.component.html',
  styleUrls: ['./summary-cards.component.scss'],
})
export class SummaryCardsComponent {
  public state = inject(AppStateService);
  public readonly OvertimeCompensationMode = OvertimeCompensationMode;
  private dialog = inject(MatDialog);

  public formatMinutes(totalMins: number): string {
    const sign = totalMins < 0 ? '-' : '';
    const abs = Math.abs(totalMins);
    const h = Math.floor(abs / 60);
    const m = abs % 60;
    return `${sign}${h}h ${m}m`;
  }

  public balanceMinutes(): number {
    return this.state.isMonthUnstarted()
      ? this.state.monthRecord().openingBalanceMinutes
      : this.state.summary().closingBalanceMinutes;
  }

  public editOpeningBalance(): void {
    const ref = this.dialog.open(BalanceDialogComponent, {
      width: '400px',
      data: {
        month: this.state.formattedMonthTitle(),
        minutes: this.state.monthRecord().openingBalanceMinutes,
      },
    });
    ref.afterClosed().subscribe((minutes) => {
      if (typeof minutes !== 'number') return;
      void this.state.saveMonthRecord({
        ...this.state.monthRecord(),
        openingBalanceMinutes: minutes,
        openingBalanceWasEdited: true,
      });
    });
  }
}
