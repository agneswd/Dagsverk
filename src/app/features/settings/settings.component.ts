import { Component, inject } from '@angular/core';
import { CommonModule } from '@angular/common';
import { MatCardModule } from '@angular/material/card';
import { MatFormFieldModule } from '@angular/material/form-field';
import { MatSelectModule } from '@angular/material/select';
import { MatButtonModule } from '@angular/material/button';
import { MatIconModule } from '@angular/material/icon';
import { AppStateService } from '../../core/app-state.service';
import { ElectronBridgeService } from '../../core/electron-bridge.service';
import { AppPreferences, LanguagePreference, ThemePreference } from '../../core/models';
import { BackupsComponent } from '../backups/backups.component';

// App-wide settings that apply to every profile. Changes apply at once.
@Component({
  selector: 'app-settings',
  standalone: true,
  imports: [
    CommonModule,
    MatCardModule,
    MatFormFieldModule,
    MatSelectModule,
    MatButtonModule,
    MatIconModule,
    BackupsComponent,
  ],
  templateUrl: './settings.component.html',
  styleUrls: ['./settings.component.scss'],
})
export class SettingsComponent {
  public state = inject(AppStateService);
  private bridge = inject(ElectronBridgeService);

  public readonly ThemePreference = ThemePreference;
  public readonly LanguagePreference = LanguagePreference;
  public readonly scaleOptions = [80, 90, 100, 110, 125, 150];
  public readonly updateState = this.bridge.updateState;

  public setPreference<K extends keyof AppPreferences>(key: K, value: AppPreferences[K]): void {
    void this.state.updatePreferences({ ...this.state.preferences(), [key]: value });
  }

  public onCheckForUpdates(): void {
    void this.bridge.checkForUpdates();
  }

  public onRestartToUpdate(): void {
    this.bridge.restartToUpdate();
  }

  public updateStatusText(): string {
    const update = this.updateState();
    switch (update.status) {
      case 'checking':
        return 'Checking for updates...';
      case 'available':
        return `Downloading version ${update.availableVersion}...`;
      case 'downloading':
        return `Downloading update ${update.progress || 0}%`;
      case 'ready':
        return `Dagsverk ${update.availableVersion} is ready to install.`;
      case 'current':
        return 'Dagsverk is up to date.';
      case 'error':
        return update.message || 'Update check failed.';
      case 'unavailable':
        return 'Update checks are available in the installed desktop app.';
      default:
        return `Version ${update.currentVersion || 'development'}`;
    }
  }
}
