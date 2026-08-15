import { Component, EventEmitter, Input, Output } from '@angular/core';
import { FormsModule } from '@angular/forms';
import { MatButtonModule } from '@angular/material/button';
import { MatIconModule } from '@angular/material/icon';
import { MatInputModule } from '@angular/material/input';
import { MatMenuModule } from '@angular/material/menu';
import { MatTooltipModule } from '@angular/material/tooltip';

@Component({
  selector: 'app-color-picker',
  standalone: true,
  imports: [
    FormsModule,
    MatButtonModule,
    MatIconModule,
    MatInputModule,
    MatMenuModule,
    MatTooltipModule,
  ],
  templateUrl: './color-picker.component.html',
  styleUrl: './color-picker.component.scss',
})
export class ColorPickerComponent {
  @Input() public color = '#5F875F';
  @Input() public label = 'Change color';
  @Output() public readonly colorChange = new EventEmitter<string>();

  public readonly colors = [
    '#5F875F',
    '#1E8E3E',
    '#00897B',
    '#039BE5',
    '#3F51B5',
    '#7986CB',
    '#8E24AA',
    '#D81B60',
    '#E67C73',
    '#F6BF26',
    '#F4511E',
    '#616161',
  ];

  public select(color: string): void {
    this.color = color.toUpperCase();
    this.colorChange.emit(this.color);
  }

  public applyHex(value: string): void {
    const normalized = value.trim().replace(/^#?/, '#');
    if (/^#[0-9a-f]{6}$/i.test(normalized)) this.select(normalized);
  }
}
