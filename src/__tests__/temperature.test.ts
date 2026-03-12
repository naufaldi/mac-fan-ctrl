import { describe, expect, it } from 'vitest';
import { getTemperatureStatus } from '$lib/temperature';

describe('getTemperatureStatus', () => {
  it('returns "unknown" for null', () => {
    expect(getTemperatureStatus(null)).toBe('unknown');
  });

  it('returns "normal" below normal threshold (70°C)', () => {
    expect(getTemperatureStatus(0)).toBe('normal');
    expect(getTemperatureStatus(50)).toBe('normal');
    expect(getTemperatureStatus(69)).toBe('normal');
  });

  it('returns "warm" at exactly the normal threshold (70°C)', () => {
    expect(getTemperatureStatus(70)).toBe('warm');
  });

  it('returns "warm" between normal and warm thresholds', () => {
    expect(getTemperatureStatus(75)).toBe('warm');
    expect(getTemperatureStatus(84)).toBe('warm');
  });

  it('returns "hot" at exactly the warm threshold (85°C)', () => {
    expect(getTemperatureStatus(85)).toBe('hot');
  });

  it('returns "hot" above the warm threshold', () => {
    expect(getTemperatureStatus(90)).toBe('hot');
    expect(getTemperatureStatus(100)).toBe('hot');
  });
});
