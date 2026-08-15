import { Component, inject } from '@angular/core';
import { CommonModule } from '@angular/common';
import { MatIconModule } from '@angular/material/icon';
import { AppStateService } from '../../../core/app-state.service';
import { HourlyPayBasis, OvertimeCompensationMode, SalaryType } from '../../../core/models';

@Component({
  selector: 'app-summary-cards',
  standalone: true,
  imports: [CommonModule, MatIconModule],
  templateUrl: './summary-cards.component.html',
  styleUrls: ['./summary-cards.component.scss'],
})
export class SummaryCardsComponent {
  public state = inject(AppStateService);
  public readonly OvertimeCompensationMode = OvertimeCompensationMode;

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

  public usesMonthlyHourlyPayBasis(): boolean {
    const settings = this.state.settings();
    return (
      settings.salary.type === SalaryType.Hourly &&
      settings.salary.hourlyPayBasis === HourlyPayBasis.MonthlyExpectedHours &&
      settings.overtimeCompensation.mode === OvertimeCompensationMode.CompTime
    );
  }

  public compTimeEarnedHours(): number {
    const summary = this.state.summary();
    return Math.max(0, summary.workedHours - summary.ordinaryPaidHours);
  }
}
