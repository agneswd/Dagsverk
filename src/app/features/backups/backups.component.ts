import { Component, inject, signal } from '@angular/core';
import { CommonModule } from '@angular/common';
import { MatCardModule } from '@angular/material/card';
import { MatButtonModule } from '@angular/material/button';
import { MatIconModule } from '@angular/material/icon';
import { MatSnackBar, MatSnackBarModule } from '@angular/material/snack-bar';
import { MatDialog, MatDialogModule } from '@angular/material/dialog';
import { firstValueFrom } from 'rxjs';
import { AppStateService } from '../../core/app-state.service';
import { ElectronBridgeService } from '../../core/electron-bridge.service';
import { ConfirmDialogComponent } from '../../core/confirm-dialog.component';

@Component({
  selector: 'app-backups',
  standalone: true,
  imports: [
    CommonModule,
    MatCardModule,
    MatButtonModule,
    MatIconModule,
    MatSnackBarModule,
    MatDialogModule,
  ],
  templateUrl: './backups.component.html',
  styleUrls: ['./backups.component.scss'],
})
export class BackupsComponent {
  public state = inject(AppStateService);
  public bridge = inject(ElectronBridgeService);
  private snackBar = inject(MatSnackBar);
  private dialog = inject(MatDialog);

  public lastBackupPath = signal<string | null>(null);
  public databasePath = signal('Loading...');

  public constructor() {
    void this.bridge.getDatabasePath().then((path) => this.databasePath.set(path));
  }

  public async onOpenDataFolder(): Promise<void> {
    await this.bridge.openDataFolder();
  }

  public async onCreateBackup(): Promise<void> {
    try {
      const backupPath = await this.bridge.createBackup();
      this.lastBackupPath.set(backupPath);
      this.snackBar.open(`Backup saved: ${backupPath}`, 'Close', { duration: 5000 });
    } catch (err: any) {
      this.snackBar.open(`Backup failed: ${err.message || err}`, 'Close', { duration: 5000 });
    }
  }

  public async onRestoreBackup(): Promise<void> {
    try {
      const res = await this.bridge.showOpenDialog({
        title: 'Select SQLite Database Backup to Restore',
        filters: [{ name: 'SQLite Database', extensions: ['db', 'sqlite', 'sqlite3'] }],
        properties: ['openFile'],
      });

      if (!res.canceled && res.filePaths.length > 0) {
        const filePath = res.filePaths[0];
        const confirmed = await firstValueFrom(
          this.dialog
            .open(ConfirmDialogComponent, {
              width: '440px',
              data: {
                title: 'Restore database?',
                message: `Replace the current database with ${filePath.split(/[\\/]/).pop() || filePath}? Dagsverk will create a safety backup first.`,
                confirmLabel: 'Restore database',
                destructive: true,
              },
            })
            .afterClosed(),
        );
        if (!confirmed) return;

        await this.bridge.restoreBackup(filePath);
        await this.state.init();
        this.snackBar.open('Database restored', 'OK', { duration: 4000 });
      }
    } catch (err: any) {
      this.snackBar.open(`Restore failed: ${err.message || err}`, 'Close', { duration: 5000 });
    }
  }

  public async onImportTidverk(): Promise<void> {
    try {
      const result = await this.bridge.showOpenDialog({
        title: 'Select Tidverk Database',
        filters: [{ name: 'Tidverk SQLite Database', extensions: ['db'] }],
        properties: ['openFile'],
      });
      if (result.canceled || result.filePaths.length === 0) return;

      const filePath = result.filePaths[0];
      const confirmed = await firstValueFrom(
        this.dialog
          .open(ConfirmDialogComponent, {
            width: '440px',
            data: {
              title: 'Import Tidverk data?',
              message:
                'Dagsverk will create backups, then import the entries and settings into a workspace. The Tidverk database will not change.',
              confirmLabel: 'Import data',
              destructive: false,
            },
          })
          .afterClosed(),
      );
      if (!confirmed) return;

      const imported = await this.bridge.importTidverkDatabase(filePath);
      await this.state.init();
      this.snackBar.open(
        `Imported ${imported.entryCount} entries into ${imported.workspaceName}`,
        'OK',
        { duration: 5000 },
      );
    } catch (err: any) {
      this.snackBar.open(`Import failed: ${err.message || err}`, 'Close', { duration: 5000 });
    }
  }
}
