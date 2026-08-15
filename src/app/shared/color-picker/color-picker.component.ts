import { Component, EventEmitter, Input, OnChanges, Output } from '@angular/core';
import { MatButtonModule } from '@angular/material/button';
import { MatIconModule } from '@angular/material/icon';
import { MatMenuModule } from '@angular/material/menu';
import { MatTooltipModule } from '@angular/material/tooltip';

@Component({
  selector: 'app-color-picker',
  standalone: true,
  imports: [MatButtonModule, MatIconModule, MatMenuModule, MatTooltipModule],
  templateUrl: './color-picker.component.html',
  styleUrl: './color-picker.component.scss',
})
export class ColorPickerComponent implements OnChanges {
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
  public hue = 120;
  public saturation = 25;
  public lightness = 45;

  public ngOnChanges(): void {
    const [r, g, b] = this.color
      .replace('#', '')
      .match(/.{2}/g)!
      .map((part) => parseInt(part, 16) / 255);
    const max = Math.max(r, g, b);
    const min = Math.min(r, g, b);
    const delta = max - min;
    this.lightness = Math.round(((max + min) / 2) * 100);
    this.saturation =
      delta === 0 ? 0 : Math.round((delta / (1 - Math.abs((2 * (max + min)) / 2 - 1))) * 100);
    this.hue = Math.round(
      delta === 0
        ? 0
        : max === r
          ? 60 * (((g - b) / delta) % 6)
          : max === g
            ? 60 * ((b - r) / delta + 2)
            : 60 * ((r - g) / delta + 4),
    );
    if (this.hue < 0) this.hue += 360;
  }

  public select(color: string): void {
    this.color = color.toUpperCase();
    this.colorChange.emit(this.color);
    this.ngOnChanges();
  }

  public updateCustomColor(channel: 'hue' | 'saturation' | 'lightness', value: number): void {
    this[channel] = value;
    const saturation = this.saturation / 100;
    const lightness = this.lightness / 100;
    const chroma = (1 - Math.abs(2 * lightness - 1)) * saturation;
    const section = this.hue / 60;
    const x = chroma * (1 - Math.abs((section % 2) - 1));
    const [red, green, blue] =
      section < 1
        ? [chroma, x, 0]
        : section < 2
          ? [x, chroma, 0]
          : section < 3
            ? [0, chroma, x]
            : section < 4
              ? [0, x, chroma]
              : section < 5
                ? [x, 0, chroma]
                : [chroma, 0, x];
    const match = lightness - chroma / 2;
    this.color =
      '#' +
      [red, green, blue]
        .map((component) =>
          Math.round((component + match) * 255)
            .toString(16)
            .padStart(2, '0'),
        )
        .join('')
        .toUpperCase();
    this.colorChange.emit(this.color);
  }
}
