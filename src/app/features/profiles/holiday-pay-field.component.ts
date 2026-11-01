import { Component, computed, model, signal } from '@angular/core';
import { FormsModule } from '@angular/forms';
import { MatFormFieldModule } from '@angular/material/form-field';
import { MatInputModule } from '@angular/material/input';
import { MatSelectModule } from '@angular/material/select';

const PRESETS = [
  { value: 0, label: 'None (included in the hourly rate)' },
  { value: 12, label: '12 % - legal minimum' },
  { value: 12.72, label: '12.72 % - hotel and restaurant agreement' },
];

// Picks the holiday pay percent for hourly pay: a common preset or a custom value.
// Usage: <app-holiday-pay-field [(percent)]="settings.salary.holidayPayPercent" />
@Component({
  selector: 'app-holiday-pay-field',
  standalone: true,
  imports: [FormsModule, MatFormFieldModule, MatInputModule, MatSelectModule],
  template: `
    <mat-form-field appearance="outline" subscriptSizing="dynamic">
      <mat-label>Holiday pay</mat-label>
      <mat-select [ngModel]="choice()" (ngModelChange)="choose($event)">
        @for (preset of presets; track preset.value) {
          <mat-option [value]="preset.value">{{ preset.label }}</mat-option>
        }
        <mat-option value="custom">Other percent</mat-option>
      </mat-select>
    </mat-form-field>
    @if (choice() === 'custom') {
      <mat-form-field appearance="outline" subscriptSizing="dynamic">
        <mat-label>Holiday pay percent</mat-label>
        <input
          matInput
          type="number"
          min="0"
          max="100"
          step="0.01"
          class="numeric"
          [ngModel]="percent()"
          (ngModelChange)="percent.set(clamp($event))"
        />
        <span matTextSuffix>%</span>
      </mat-form-field>
    }
  `,
  styles: [
    `
      :host {
        display: contents;
      }
      mat-form-field {
        flex: 1 1 260px;
      }
    `,
  ],
})
export class HolidayPayFieldComponent {
  public readonly percent = model.required<number>();
  public readonly presets = PRESETS;
  private customSelected = signal(false);

  public readonly choice = computed<number | 'custom'>(() => {
    const value = this.percent() ?? 0;
    if (this.customSelected()) return 'custom';
    return PRESETS.some((preset) => preset.value === value) ? value : 'custom';
  });

  public choose(choice: number | 'custom'): void {
    this.customSelected.set(choice === 'custom');
    if (choice !== 'custom') this.percent.set(choice);
  }

  public clamp(value: number | null): number {
    return Math.min(100, Math.max(0, Number(value) || 0));
  }
}
