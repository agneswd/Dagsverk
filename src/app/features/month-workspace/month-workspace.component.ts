import { Component, HostListener, inject, signal } from '@angular/core';
import { CommonModule } from '@angular/common';
import { MatSidenavModule } from '@angular/material/sidenav';
import { AppStateService } from '../../core/app-state.service';
import { MonthViewPreference } from '../../core/models';
import { SummaryCardsComponent } from './summary-cards/summary-cards.component';
import { LedgerViewComponent } from './ledger-view/ledger-view.component';
import { CalendarViewComponent } from './calendar-view/calendar-view.component';
import { DayEditorComponent } from './day-editor/day-editor.component';

@Component({
  selector: 'app-month-workspace',
  standalone: true,
  imports: [
    CommonModule,
    MatSidenavModule,
    SummaryCardsComponent,
    LedgerViewComponent,
    CalendarViewComponent,
    DayEditorComponent
  ],
  templateUrl: './month-workspace.component.html',
  styleUrls: ['./month-workspace.component.scss']
})
export class MonthWorkspaceComponent {
  public state = inject(AppStateService);
  public readonly MonthViewPreference = MonthViewPreference;

  public isWideScreen = signal<boolean>(
    typeof window !== 'undefined' ? window.innerWidth >= 1600 : false
  );

  @HostListener('window:resize')
  public onResize(): void {
    if (typeof window !== 'undefined') {
      this.isWideScreen.set(window.innerWidth >= 1600);
    }
  }
}
