import { Component, inject } from '@angular/core';
import { FormsModule } from '@angular/forms';
import { MatButtonModule } from '@angular/material/button';
import { MatDialogModule, MatDialogRef } from '@angular/material/dialog';
import { MAT_DIALOG_DATA } from '@angular/material/dialog';
import { MatFormFieldModule } from '@angular/material/form-field';
import { MatInputModule } from '@angular/material/input';

export interface RenameDialogData {
  title: string;
  label: string;
  initialName: string;
}

@Component({
  selector: 'app-rename-dialog',
  standalone: true,
  imports: [FormsModule, MatButtonModule, MatDialogModule, MatFormFieldModule, MatInputModule],
  template: `
    <h2 mat-dialog-title>{{ data.title }}</h2>
    <mat-dialog-content>
      <mat-form-field appearance="outline" subscriptSizing="dynamic" class="full-width">
        <mat-label>{{ data.label }}</mat-label>
        <input
          matInput
          [(ngModel)]="name"
          (keydown.enter)="confirm()"
          cdkFocusInitial
          maxlength="120"
        />
      </mat-form-field>
    </mat-dialog-content>
    <mat-dialog-actions align="end">
      <button mat-button type="button" (click)="dialogRef.close()">Cancel</button>
      <button
        mat-flat-button
        type="button"
        color="primary"
        [disabled]="!name.trim()"
        (click)="confirm()"
      >
        Rename
      </button>
    </mat-dialog-actions>
  `,
})
export class RenameDialogComponent {
  public dialogRef = inject(MatDialogRef<RenameDialogComponent, string | undefined>);
  public data = inject<RenameDialogData>(MAT_DIALOG_DATA);
  public name = this.data.initialName;

  public confirm(): void {
    const trimmed = this.name.trim();
    if (!trimmed) return;
    this.dialogRef.close(trimmed);
  }
}
