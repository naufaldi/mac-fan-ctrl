/**
 * Design Token Types for mac-fan-ctrl
 * Mirrors CSS variables defined in src/app.css @theme block
 */

/** Temperature status variants */
export type TemperatureStatus = 'normal' | 'warm' | 'hot' | 'unknown';

/** CSS variable references for status colors */
export const StatusColors: Record<TemperatureStatus, string> = {
  normal: 'var(--color-status-normal)',
  warm: 'var(--color-status-warm)',
  hot: 'var(--color-status-hot)',
  unknown: 'var(--color-status-unknown)',
};

/** Temperature thresholds in Celsius */
export const TemperatureThresholds = {
  normal: 70,
  warm: 85,
} as const;

/** CSS variable references for spacing */
export const Spacing = {
  1: 'var(--spacing-1)',
  2: 'var(--spacing-2)',
  3: 'var(--spacing-3)',
  4: 'var(--spacing-4)',
  6: 'var(--spacing-6)',
  8: 'var(--spacing-8)',
} as const;

/** CSS variable references for surface colors */
export const SurfaceColors = {
  card: 'var(--color-surface-card)',
  hover: 'var(--color-surface-hover)',
} as const;

/** CSS variable references for border radius */
export const Radius = {
  card: 'var(--radius-card)',
  dot: 'var(--radius-dot)',
} as const;

/** CSS variable reference for mono font */
export const FontMono = 'var(--font-mono)';
export const FontUi = 'var(--font-ui)';

export type FanControlMode = 'auto' | 'constant';

/** Sensor data interface for components */
export interface SensorData {
  id: string;
  fanIndex?: number;
  label: string;
  value: number | null;
  unit: 'celsius' | 'rpm';
  status: TemperatureStatus;
  minRpm?: number;
  maxRpm?: number;
  controlMode?: FanControlMode;
  targetRpm?: number;
  sparklineData?: number[];
}
