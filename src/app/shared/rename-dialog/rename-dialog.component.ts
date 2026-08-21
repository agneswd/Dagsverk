import { Component, inject } from '@angular/core';
import { FormsModule } from '@angular/forms';
import { A11yModule } from '@angular/cdk/a11y';
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
  imports: [
    FormsModule,
    A11yModule,
    MatButtonModule,
    MatDialogModule,
    MatFormFieldModule,
    MatInputModule,
  ],
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
  styles: [
    `
      .full-width {
        width: 100%;
      }

      :host ::ng-deep .mat-mdc-dialog-content {
        overflow: visible;
        padding-top: 8px;
      }

      :host ::ng-deep .mat-mdc-text-field-wrapper {
        min-height: 56px;
      }

      :host ::ng-deep .mat-mdc-form-field-infix {
        min-height: 56px;
        padding-top: 16px !important;
        padding-bottom: 16px !important;
      }

      :host ::ng-deep input.mat-mdc-input-element {
        height: 24px;
        line-height: 24px;
      }

      :host ::ng-deep .mdc-notched-outline__notch {
        overflow: visible;
      }
    `,
  ],
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
