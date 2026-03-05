import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type { SensorData } from "./types";

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
