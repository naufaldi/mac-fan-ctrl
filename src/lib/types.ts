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
	perf_level_core_counts?: [number, number] | null;
	per_core_cpu_temp_available?: boolean;
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

export type FanMode = "auto" | "forced";

export interface FanData {
	index: number;
	label: string;
	actual: number;
	min: number;
	max: number;
	target: number;
	mode: FanMode;
}

export interface SensorData {
	summary: SummarySensors;
	details: Sensor[];
	diagnostics?: SensorDiagnostics;
	fans: FanData[];
}

// ── Fan control configuration (mirrors Rust FanControlConfig) ────────────────

export type FanControlConfigMode = "auto" | "constant_rpm" | "sensor_based";

export interface FanControlConfigAuto {
	readonly mode: "auto";
}

export interface FanControlConfigConstantRpm {
	readonly mode: "constant_rpm";
	readonly target_rpm: number;
}

export interface FanControlConfigSensorBased {
	readonly mode: "sensor_based";
	readonly sensor_key: string;
	readonly temp_low: number;
	readonly temp_high: number;
}

export type FanControlConfig =
	| FanControlConfigAuto
	| FanControlConfigConstantRpm
	| FanControlConfigSensorBased;

// ── Presets ──────────────────────────────────────────────────────────────────

export interface Preset {
	readonly name: string;
	readonly builtin: boolean;
	readonly configs: Record<string, FanControlConfig>;
}
