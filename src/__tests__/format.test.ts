import { describe, expect, it } from 'vitest';
import { formatRpm, formatTemperature } from '$lib/format';

describe('formatRpm', () => {
  it('returns plain number string below 1000', () => {
    expect(formatRpm(0)).toBe('0');
    expect(formatRpm(500)).toBe('500');
    expect(formatRpm(999)).toBe('999');
  });

  it('returns K notation at exactly 1000', () => {
    expect(formatRpm(1000)).toBe('1.0K');
  });

  it('returns K notation above 1000', () => {
    expect(formatRpm(1500)).toBe('1.5K');
    expect(formatRpm(2450)).toBe('2.5K');
    expect(formatRpm(5800)).toBe('5.8K');
  });
});

describe('formatTemperature', () => {
  it('formats integer temperatures', () => {
    expect(formatTemperature(72)).toBe('72°C');
    expect(formatTemperature(0)).toBe('0°C');
  });

  it('rounds decimal temperatures', () => {
    expect(formatTemperature(72.4)).toBe('72°C');
    expect(formatTemperature(72.5)).toBe('73°C');
    expect(formatTemperature(72.9)).toBe('73°C');
  });
});
