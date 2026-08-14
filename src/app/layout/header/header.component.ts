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
import { AppStateService } from '../../core/app-state.service';
import { ElectronBridgeService } from '../../core/electron-bridge.service';
import { MonthViewPreference } from '../../core/models';

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
    MatDividerModule
  ],
  templateUrl: './header.component.html',
  styleUrls: ['./header.component.scss']
})
export class HeaderComponent {
  public state = inject(AppStateService);
  public bridge = inject(ElectronBridgeService);
  private router = inject(Router);

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
    { num: 12, name: 'December' }
  ];

  public constructor() {
    this.currentRoute.set(this.router.url);
    this.router.events
      .pipe(filter(event => event instanceof NavigationEnd))
      .subscribe((event: any) => {
        this.currentRoute.set(event.urlAfterRedirects || event.url);
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
}
