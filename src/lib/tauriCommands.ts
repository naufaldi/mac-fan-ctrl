import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type { FanControlConfig, PowerPresetConfig, PowerSource, Preset, SensorData } from "./types";

export const SENSOR_UPDATE_EVENT = "sensor_update";

export async function pingBackend(message: string): Promise<string> {
	return invoke<string>("ping_backend", { message });
}

// ── App info ────────────────────────────────────────────────────────────────

export interface AppInfo {
	name: string;
	version: string;
	identifier: string;
}

export async function getAppInfo(): Promise<AppInfo> {
	return invoke<AppInfo>("get_app_info");
}

export async function listenShowAbout(
	callback: () => void,
): Promise<UnlistenFn> {
	return listen("show-about", () => {
		callback();
	});
}

export async function listenCheckForUpdates(
	callback: () => void,
): Promise<UnlistenFn> {
	return listen("check-for-updates", () => {
		callback();
	});
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

// ── Alert commands ──────────────────────────────────────────────────────────

export interface AlertConfig {
	enabled: boolean;
	cpu_threshold: number;
	cooldown_secs: number;
}

export async function getAlertConfig(): Promise<AlertConfig> {
	return invoke<AlertConfig>("get_alert_config");
}

export async function setAlertConfig(params: Partial<AlertConfig>): Promise<AlertConfig> {
	return invoke<AlertConfig>("set_alert_config", { params });
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

// ── Power preset commands ───────────────────────────────────────────────────

export async function getPowerPresetConfig(): Promise<PowerPresetConfig> {
	return invoke<PowerPresetConfig>("get_power_preset_config");
}

export async function setPowerPresetConfig(params: Partial<PowerPresetConfig>): Promise<PowerPresetConfig> {
	return invoke<PowerPresetConfig>("set_power_preset_config", { params });
}

export async function getCurrentPowerSource(): Promise<PowerSource> {
	return invoke<PowerSource>("get_current_power_source");
}

export async function listenToPowerSourceChanges(
	onUpdate: (source: PowerSource) => void,
): Promise<UnlistenFn> {
	return listen<PowerSource>("power_source_changed", ({ payload }) => {
		onUpdate(payload);
	});
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

export async function hideToMenuBar(): Promise<void> {
	return invoke<void>("hide_to_menu_bar");
}

export async function installHelper(): Promise<string> {
	return invoke<string>("install_helper");
}

export async function reconnectWriter(): Promise<boolean> {
	return invoke<boolean>("reconnect_writer");
}

// ── URL commands ────────────────────────────────────────────────────────────

export async function openUrl(url: string): Promise<void> {
	return invoke<void>("open_url", { url });
}
