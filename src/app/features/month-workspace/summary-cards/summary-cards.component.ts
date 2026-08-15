import { Component, inject } from '@angular/core';
import { CommonModule } from '@angular/common';
import { MatIconModule } from '@angular/material/icon';
import { AppStateService } from '../../../core/app-state.service';
import { OvertimeCompensationMode } from '../../../core/models';

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
}
