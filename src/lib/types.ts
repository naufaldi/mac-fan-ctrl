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

export interface Sensor {
  key: string;
  name: string;
  value: number | null;
  unit: SensorUnit;
  sensor_type: SensorType;
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
}
