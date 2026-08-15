import { Component, inject, signal } from '@angular/core';
import { CommonModule } from '@angular/common';
import { MatCardModule } from '@angular/material/card';
import { MatButtonModule } from '@angular/material/button';
import { MatIconModule } from '@angular/material/icon';
import { MatSnackBar, MatSnackBarModule } from '@angular/material/snack-bar';
import { MatDialog, MatDialogModule, MatDialogRef, MAT_DIALOG_DATA } from '@angular/material/dialog';
import { firstValueFrom } from 'rxjs';
import { AppStateService } from '../../core/app-state.service';
import { ElectronBridgeService } from '../../core/electron-bridge.service';

@Component({
  selector: 'app-confirm-restore-dialog',
  standalone: true,
  imports: [MatButtonModule, MatDialogModule],
  template: `
    <h2 mat-dialog-title>Restore database?</h2>
    <mat-dialog-content>
      Dagsverk will replace the current database with <strong>{{ data.fileName }}</strong>. A safety backup will be created first.
    </mat-dialog-content>
    <mat-dialog-actions align="end">
      <button mat-button type="button" (click)="dialogRef.close(false)">Cancel</button>
      <button mat-flat-button type="button" (click)="dialogRef.close(true)">Restore database</button>
    </mat-dialog-actions>
  `
})
export class ConfirmRestoreDialogComponent {
  public dialogRef = inject(MatDialogRef<ConfirmRestoreDialogComponent>);
  public data = inject<{ fileName: string }>(MAT_DIALOG_DATA);
}

@Component({
  selector: 'app-backups',
  standalone: true,
  imports: [CommonModule, MatCardModule, MatButtonModule, MatIconModule, MatSnackBarModule, MatDialogModule],
  templateUrl: './backups.component.html',
  styleUrls: ['./backups.component.scss']
})
export class BackupsComponent {
  public state = inject(AppStateService);
  public bridge = inject(ElectronBridgeService);
  private snackBar = inject(MatSnackBar);
  private dialog = inject(MatDialog);

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
        const filePath = res.filePaths[0];
        const confirmed = await firstValueFrom(this.dialog.open(ConfirmRestoreDialogComponent, {
          width: '440px',
          data: { fileName: filePath.split(/[\\/]/).pop() || filePath }
        }).afterClosed());
        if (!confirmed) return;

        await this.bridge.restoreBackup(filePath);
        await this.state.init();
        this.snackBar.open('Database restored', 'OK', { duration: 4000 });
      }
    } catch (err: any) {
      this.snackBar.open(`Restore failed: ${err.message || err}`, 'Close', { duration: 5000 });
    }
  }
}
