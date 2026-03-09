import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type { FanControlConfig, Preset, SensorData } from "./types";

export const SENSOR_UPDATE_EVENT = "sensor_update";

export async function pingBackend(message: string): Promise<string> {
	return invoke<string>("ping_backend", { message });
}

export async function getSensors(): Promise<SensorData> {
	return invoke<SensorData>("get_sensors");
}

export async function listenToSensorUpdates(
	onUpdate: (payload: SensorData) => void,
): Promise<UnlistenFn> {
	return listen<SensorData>(SENSOR_UPDATE_EVENT, ({ payload }) => {
		onUpdate(payload);
	});
}

// ── Fan control commands ─────────────────────────────────────────────────────

export async function setFanConstantRpm(
	fanIndex: number,
	rpm: number,
): Promise<void> {
	return invoke<void>("set_fan_constant_rpm", {
		fanIndex,
		rpm,
	});
}

export async function setFanSensorControl(
	fanIndex: number,
	sensorKey: string,
	tempLow: number,
	tempHigh: number,
): Promise<void> {
	return invoke<void>("set_fan_sensor_control", {
		fanIndex,
		sensorKey,
		tempLow,
		tempHigh,
	});
}

export async function setFanAuto(fanIndex: number): Promise<void> {
	return invoke<void>("set_fan_auto", { fanIndex });
}

export async function getFanControlConfigs(): Promise<
	Record<string, FanControlConfig>
> {
	return invoke<Record<string, FanControlConfig>>("get_fan_control_configs");
}

// ── Preset commands ──────────────────────────────────────────────────────────

export async function getPresets(): Promise<Preset[]> {
	return invoke<Preset[]>("get_presets");
}

export async function getActivePreset(): Promise<string | null> {
	return invoke<string | null>("get_active_preset");
}

export async function applyPreset(name: string): Promise<void> {
	return invoke<void>("apply_preset", { name });
}

export async function savePreset(name: string): Promise<void> {
	return invoke<void>("save_preset", { name });
}

export async function deletePreset(name: string): Promise<void> {
	return invoke<void>("delete_preset", { name });
}

// ── Privilege commands ──────────────────────────────────────────────────────

export interface PrivilegeStatus {
	has_write_access: boolean;
}

export async function getPrivilegeStatus(): Promise<PrivilegeStatus> {
	return invoke<PrivilegeStatus>("get_privilege_status");
}

export async function requestPrivilegeRestart(): Promise<void> {
	return invoke<void>("request_privilege_restart");
}

// ── Tray display commands ────────────────────────────────────────────────────

export async function setTrayDisplayMode(mode: number): Promise<void> {
	return invoke<void>("set_tray_display_mode", { mode });
}

export async function getTrayDisplayMode(): Promise<number> {
	return invoke<number>("get_tray_display_mode");
}

// ── URL commands ────────────────────────────────────────────────────────────

export async function openUrl(url: string): Promise<void> {
	return invoke<void>("open_url", { url });
}
