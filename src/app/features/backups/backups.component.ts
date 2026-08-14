import { Component, inject, signal } from '@angular/core';
import { CommonModule } from '@angular/common';
import { MatCardModule } from '@angular/material/card';
import { MatButtonModule } from '@angular/material/button';
import { MatIconModule } from '@angular/material/icon';
import { MatSnackBar, MatSnackBarModule } from '@angular/material/snack-bar';
import { AppStateService } from '../../core/app-state.service';
import { ElectronBridgeService } from '../../core/electron-bridge.service';

@Component({
  selector: 'app-backups',
  standalone: true,
  imports: [CommonModule, MatCardModule, MatButtonModule, MatIconModule, MatSnackBarModule],
  templateUrl: './backups.component.html',
  styleUrls: ['./backups.component.scss']
})
export class BackupsComponent {
  public state = inject(AppStateService);
  public bridge = inject(ElectronBridgeService);
  private snackBar = inject(MatSnackBar);

  public lastBackupPath = signal<string | null>(null);

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
        properties: ['openFile']
      });

      if (!res.canceled && res.filePaths.length > 0) {
        await this.bridge.restoreBackup(res.filePaths[0]);
        await this.state.init();
        this.snackBar.open('Database successfully restored!', 'OK', { duration: 4000 });
      }
    } catch (err: any) {
      this.snackBar.open(`Restore failed: ${err.message || err}`, 'Close', { duration: 5000 });
    }
  }
}
