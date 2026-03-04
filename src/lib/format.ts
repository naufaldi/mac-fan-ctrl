/**
 * Format RPM value with K notation for values >= 1000
 * @param rpm - RPM value
 * @returns Formatted string (e.g., 2450 -> "2.4K")
 */
export function formatRpm(rpm: number): string {
  if (rpm >= 1000) {
    return `${(rpm / 1000).toFixed(1)}K`;
  }
  return rpm.toString();
}

/**
 * Format temperature with degree symbol
 * @param temp - Temperature in Celsius
 * @returns Formatted string (e.g., 72 -> "72°C")
 */
export function formatTemperature(temp: number): string {
  return `${Math.round(temp)}°C`;
}
