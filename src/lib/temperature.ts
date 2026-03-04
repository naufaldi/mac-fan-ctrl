import { TemperatureThresholds, type TemperatureStatus } from './designTokens';

/**
 * Determine temperature status based on thresholds
 * @param temp - Temperature in Celsius, or null if unavailable
 * @returns Status: 'normal' | 'warm' | 'hot' | 'unknown'
 */
export function getTemperatureStatus(temp: number | null): TemperatureStatus {
  if (temp === null) return 'unknown';
  if (temp >= TemperatureThresholds.warm) return 'hot';
  if (temp >= TemperatureThresholds.normal) return 'warm';
  return 'normal';
}
