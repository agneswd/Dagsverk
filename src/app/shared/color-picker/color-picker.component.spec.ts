import { describe, expect, it } from 'vitest';
import { ColorPickerComponent } from './color-picker.component';

describe('ColorPickerComponent', () => {
  it('converts visual picker values to a hex color', () => {
    const picker = new ColorPickerComponent();
    picker.color = '#FF0000';
    picker.ngOnChanges();

    picker.updateCustomColor('hue', 120);

    expect(picker.color).toBe('#00FF00');
  });
});
