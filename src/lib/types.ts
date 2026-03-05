export type SensorType =
  | "Cpu"
  | "Gpu"
  | "Memory"
  | "Storage"
  | "Battery"
  | "Power"
  | "Trackpad"
  | "Other";

export type SensorUnit = "C" | "W";
export type SensorSource = "smc" | "iohid_iokit" | "derived" | "placeholder";
export type NullReason = "placeholder" | "unsupported" | "read_error";

export interface UnresolvedSensor {
  provider: string;
  raw_name: string;
  reason: string;
}

export interface SensorDiagnostics {
  model_id: string | null;
  diagnostics_enabled: boolean;
  active_providers: string[];
  unresolved: UnresolvedSensor[];
}

export interface Sensor {
  key: string;
  name: string;
  value: number | null;
  unit: SensorUnit;
  sensor_type: SensorType;
  source?: SensorSource;
  null_reason?: NullReason | null;
}

export interface SummarySensors {
  cpu_package: Sensor | null;
  gpu: Sensor | null;
  ram: Sensor | null;
  ssd: Sensor | null;
}

export interface SensorData {
  summary: SummarySensors;
  details: Sensor[];
  diagnostics?: SensorDiagnostics;
}
