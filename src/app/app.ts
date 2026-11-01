import { Component, DestroyRef, effect, inject, signal } from '@angular/core';
import { CommonModule } from '@angular/common';
import { Router, RouterOutlet } from '@angular/router';
import { MatSidenavModule } from '@angular/material/sidenav';
import { MatDialog } from '@angular/material/dialog';
import { MatSnackBar, MatSnackBarModule } from '@angular/material/snack-bar';
import { HeaderComponent } from './layout/header/header.component';
import { SidebarComponent } from './layout/sidebar/sidebar.component';
import { AppStateService } from './core/app-state.service';
import { ElectronBridgeService } from './core/electron-bridge.service';
import { MonthViewPreference } from './core/models';
import { ProfileFormDialogComponent } from './features/profiles/profile-form-dialog.component';

@Component({
  selector: 'app-root',
  standalone: true,
  imports: [
    CommonModule,
    RouterOutlet,
    MatSidenavModule,
    MatSnackBarModule,
    HeaderComponent,
    SidebarComponent,
  ],
  templateUrl: './app.html',
  styleUrl: './app.scss',
})
export class App {
  public state = inject(AppStateService);
  private dialog = inject(MatDialog);
  private router = inject(Router);
  private destroyRef = inject(DestroyRef);
  private bridge = inject(ElectronBridgeService);
  private snackBar = inject(MatSnackBar);
  private setupOpened = false;
  private notifiedUpdateVersion?: string;
  private isNarrowWindow = window.innerWidth < 1200;
  public sidebarCollapsed = signal(this.isNarrowWindow);

  public constructor() {
    const handleResize = () => {
      const isNarrowWindow = window.innerWidth < 1200;
      if (isNarrowWindow && !this.isNarrowWindow) this.sidebarCollapsed.set(true);
      this.isNarrowWindow = isNarrowWindow;
    };
    window.addEventListener('resize', handleResize);
    this.destroyRef.onDestroy(() => window.removeEventListener('resize', handleResize));

    effect(() => {
      const update = this.bridge.updateState();
      if (update.status !== 'ready' || update.availableVersion === this.notifiedUpdateVersion)
        return;
      this.notifiedUpdateVersion = update.availableVersion;
      const notification = this.snackBar.open(
        `Dagsverk ${update.availableVersion || ''} is ready to install.`,
        'Restart now',
        { duration: 0 },
      );
      notification.onAction().subscribe(() => this.bridge.restartToUpdate());
    });

    effect(() => {
      if (
        this.state.isInitialized() &&
        !this.state.preferences().hasCompletedSetup &&
        !this.setupOpened
      ) {
        this.setupOpened = true;
        this.dialog.open(ProfileFormDialogComponent, {
          data: { mode: 'setup' },
          disableClose: true,
          width: '640px',
          maxWidth: 'calc(100vw - 32px)',
        });
      }
    });
  }

  public onShortcut(event: KeyboardEvent): void {
    const target = event.target as HTMLElement | null;
    const editing = target?.matches('input, textarea, select, [contenteditable="true"]');
    if (editing && !(event.ctrlKey && event.key.toLowerCase() === 's')) return;

    if (event.ctrlKey && event.key === '1') {
      event.preventDefault();
      this.router.navigate(['/timesheet']);
      this.state.setView(MonthViewPreference.Ledger);
    } else if (event.ctrlKey && event.key === '2') {
      event.preventDefault();
      this.router.navigate(['/timesheet']);
      this.state.setView(MonthViewPreference.Calendar);
    } else if (event.ctrlKey && event.key.toLowerCase() === 'm') {
      event.preventDefault();
      this.state.startCatchUp();
    } else if (event.ctrlKey && event.key.toLowerCase() === 'e') {
      event.preventDefault();
      this.state.exportExcel();
    } else if (event.ctrlKey && event.key === ',') {
      event.preventDefault();
      this.router.navigate(['/settings']);
    } else if (event.ctrlKey && event.key.toLowerCase() === 's') {
      event.preventDefault();
      window.dispatchEvent(new Event('dagsverk-save'));
    } else if (event.key === 'PageUp') {
      event.preventDefault();
      this.state.previousMonth();
    } else if (event.key === 'PageDown') {
      event.preventDefault();
      this.state.nextMonth();
    } else if (event.key === 'Escape' && this.state.isEditorOpen()) {
      event.preventDefault();
      this.state.closeCatchUp();
    }
  }
}
