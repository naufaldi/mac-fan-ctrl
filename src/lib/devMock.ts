import { emit } from "@tauri-apps/api/event";
import { mockIPC, mockWindows } from "@tauri-apps/api/mocks";
import type { FanControlConfig, Preset, SensorData } from "./types";

/**
 * Browser-only dev mock. Installs a Tauri IPC shim so the dashboard renders
 * with sample telemetry outside the native webview. Never loaded in
 * production (see src/main.ts) and never used inside Tauri.
 */

let tick = 0;

const sampleFans = [
	{
		index: 0,
		label: "System Fan 1",
		min: 1200,
		max: 6550,
		actual: 2532,
		target: 3200,
	},
	{
		index: 1,
		label: "System Fan 2",
		min: 1200,
		max: 6550,
		actual: 2200,
		target: 3200,
	},
	{
		index: 2,
		label: "System Fan 3",
		min: 1200,
		max: 6550,
		actual: 2000,
		target: 3200,
	},
] as const;

const defaultFanConfigs = (): Record<string, FanControlConfig> =>
	Object.fromEntries(
		sampleFans.map((fan) => [
			String(fan.index),
			{ mode: "constant_rpm", target_rpm: fan.target },
		]),
	);

let fanConfigs: Record<string, FanControlConfig> = defaultFanConfigs();

function wobble(): number {
  tick += 1;
  return 0;
}

function clampRpm(rpm: number, min: number, max: number): number {
	return Math.max(min, Math.min(max, Math.round(rpm)));
}

function getCommandNumber(args: unknown, key: string): number {
	if (!args || typeof args !== "object") {
		throw new Error(`Missing command arguments for ${key}`);
	}

	const value = (args as Record<string, unknown>)[key];
	if (typeof value !== "number" || !Number.isFinite(value)) {
		throw new Error(`Invalid ${key}: ${String(value)}`);
	}

	return value;
}

function getCommandString(args: unknown, key: string): string {
	if (!args || typeof args !== "object") {
		throw new Error(`Missing command arguments for ${key}`);
	}

	const value = (args as Record<string, unknown>)[key];
	if (typeof value !== "string" || value.trim() === "") {
		throw new Error(`Invalid ${key}: ${String(value)}`);
	}

	return value;
}

function getSampleFan(fanIndex: number): (typeof sampleFans)[number] {
	const fan = sampleFans.find((sampleFan) => sampleFan.index === fanIndex);
	if (!fan) {
		throw new Error(`Fan index ${fanIndex} not found in development mock`);
	}
	return fan;
}

function fanData(o: number): SensorData["fans"] {
	return sampleFans.map((fan) => {
		const config = fanConfigs[String(fan.index)] ?? { mode: "auto" };
		const target =
			config.mode === "constant_rpm"
				? config.target_rpm
				: config.mode === "sensor_based"
					? fan.min
					: fan.target;

		return {
			index: fan.index,
			label: fan.label,
			actual: fan.actual + o * 20,
			min: fan.min,
			max: fan.max,
			target,
			mode: config.mode === "auto" ? "auto" : "forced",
		};
	});
}

function sampleSensorData(): SensorData {
  const o = wobble();
  return {
    summary: { cpu_package: null, gpu: null, ram: null, ssd: null },
    details: [
      { key: "TB0T", name: "Battery", value: 37 + o, unit: "C", sensor_type: "Battery", source: "smc" },
      { key: "TCPUAVG", name: "CPU Core Average", value: 56 + o, unit: "C", sensor_type: "Cpu", source: "derived" },
      { key: "TP0b", name: "CPU Efficiency Cores Average", value: 49 + o, unit: "C", sensor_type: "Cpu", source: "iohid_iokit" },
      { key: "TP2b", name: "CPU Performance Cores Average", value: 61 + o, unit: "C", sensor_type: "Cpu", source: "iohid_iokit" },
      { key: "TG0D", name: "GPU Cluster (AGX)", value: 68 + o, unit: "C", sensor_type: "Gpu", source: "iohid_iokit" },
      { key: "TPCD", name: "Power Management Unit", value: 52 + o, unit: "C", sensor_type: "Power", source: "iohid_iokit" },
      { key: "TN0n", name: "APPLE SSD AP0512Q", value: 38 + o, unit: "C", sensor_type: "Storage", source: "smc", null_reason: null },
      { key: "TN1n", name: "SanDisk Extreme Pro", value: 36 + o, unit: "C", sensor_type: "Storage", source: "smc", null_reason: null },
    ],
    diagnostics: {
      model_id: "MacBookPro18,3",
      diagnostics_enabled: false,
      active_providers: ["smc", "iohid_iokit"],
      unresolved: [],
      per_core_cpu_temp_available: true,
    },
    fans: fanData(o),
  };
}

function samplePresets(): Preset[] {
  return [
    { name: "Automatic", builtin: true, configs: {} },
    { name: "High Cooling", builtin: true, configs: {} },
    { name: "Quiet", builtin: true, configs: {} },
  ];
}

function sampleFanConfigs(): Record<string, FanControlConfig> {
  return fanConfigs;
}

function emitSensorUpdate(): void {
	void emit("sensor_update", sampleSensorData());
}

function setConstantRpm(args: unknown): void {
	const fanIndex = getCommandNumber(args, "fanIndex");
	const rpm = getCommandNumber(args, "rpm");
	const fan = getSampleFan(fanIndex);
	fanConfigs = {
		...fanConfigs,
		[String(fanIndex)]: {
			mode: "constant_rpm",
			target_rpm: clampRpm(rpm, fan.min, fan.max),
		},
	};
	emitSensorUpdate();
}

function setSensorControl(args: unknown): void {
	const fanIndex = getCommandNumber(args, "fanIndex");
	const sensorKey = getCommandString(args, "sensorKey");
	const tempLow = getCommandNumber(args, "tempLow");
	const tempHigh = getCommandNumber(args, "tempHigh");
	const sensorExists = sampleSensorData().details.some((sensor) => sensor.key === sensorKey);
	getSampleFan(fanIndex);
	if (!sensorExists) {
		throw new Error(`Sensor key '${sensorKey}' not found in development mock`);
	}
	fanConfigs = {
		...fanConfigs,
		[String(fanIndex)]: {
			mode: "sensor_based",
			sensor_key: sensorKey,
			temp_low: tempLow,
			temp_high: tempHigh,
		},
	};
	emitSensorUpdate();
}

function setAuto(args: unknown): void {
	const fanIndex = getCommandNumber(args, "fanIndex");
	getSampleFan(fanIndex);
	fanConfigs = {
		...fanConfigs,
		[String(fanIndex)]: { mode: "auto" },
	};
	emitSensorUpdate();
}

function setAllConstantRpm(args: unknown): void {
	const rpm = getCommandNumber(args, "rpm");
	fanConfigs = Object.fromEntries(
		sampleFans.map((fan) => [
			String(fan.index),
			{
				mode: "constant_rpm" as const,
				target_rpm: clampRpm(rpm, fan.min, fan.max),
			},
		]),
	);
	emitSensorUpdate();
}

function restoreAllAuto(): void {
	fanConfigs = Object.fromEntries(
		sampleFans.map((fan) => [String(fan.index), { mode: "auto" as const }]),
	);
	emitSensorUpdate();
}

export function handleDevMockCommand(cmd: string, args?: unknown): unknown {
  switch (cmd) {
    case "get_sensors":
      return sampleSensorData();
    case "get_fan_control_configs":
      return sampleFanConfigs();
    case "set_fan_constant_rpm":
      setConstantRpm(args);
      return undefined;
    case "set_fan_sensor_control":
      setSensorControl(args);
      return undefined;
    case "set_fan_auto":
      setAuto(args);
      return undefined;
	case "set_all_fans_constant_rpm":
		setAllConstantRpm(args);
		return undefined;
	case "restore_all_fans_auto":
		restoreAllAuto();
		return undefined;
    case "get_presets":
      return samplePresets();
    case "get_active_preset":
      return "Automatic";
    case "get_privilege_status":
      return { has_write_access: true, fan_control_available: true, reason: null };
    case "get_tray_display_mode":
      return 0;
    case "get_app_info":
      return { name: "FanGuard", version: "0.1.0-beta.5", identifier: "io.github.naufaldi.fanguard" };
    case "get_power_preset_config":
      return { enabled: false, ac_preset: null, battery_preset: null };
    case "get_current_power_source":
      return "ac";
    case "get_alert_config":
      return { enabled: false, cpu_threshold: 85, cooldown_secs: 300 };
    default:
      return undefined;
  }
}

export function resetDevMockStateForTest(): void {
	tick = 0;
	fanConfigs = defaultFanConfigs();
}

let interval: ReturnType<typeof setInterval> | null = null;

export function installDevMock(): void {
  mockWindows("main");
  mockIPC((cmd, args) => handleDevMockCommand(cmd, args), { shouldMockEvents: true });
  interval = setInterval(() => {
    void emit("sensor_update", sampleSensorData());
  }, 2000);
}

export function stopDevMock(): void {
  if (interval) {
    clearInterval(interval);
    interval = null;
  }
}
