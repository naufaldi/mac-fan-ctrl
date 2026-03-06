import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { describe, expect, it, vi } from "vitest";
import {
	getSensors,
	listenToSensorUpdates,
	SENSOR_UPDATE_EVENT,
} from "./tauriCommands";
import type { SensorData } from "./types";

vi.mock("@tauri-apps/api/core", () => ({
	invoke: vi.fn(),
}));

vi.mock("@tauri-apps/api/event", () => ({
	listen: vi.fn(),
}));

const sampleSensors: SensorData = {
	summary: {
		cpu_package: {
			key: "TC0D",
			name: "CPU Die",
			value: 70,
			unit: "C",
			sensor_type: "Cpu",
		},
		gpu: null,
		ram: null,
		ssd: null,
	},
	details: [
		{
			key: "TC0D",
			name: "CPU Die",
			value: 70,
			unit: "C",
			sensor_type: "Cpu",
		},
	],
	fans: [],
};

describe("tauriCommands", () => {
	it("maps getSensors to get_sensors invoke command", async () => {
		const mockedInvoke = vi.mocked(invoke);
		mockedInvoke.mockResolvedValue(sampleSensors);

		const result = await getSensors();

		expect(result).toEqual(sampleSensors);
		expect(mockedInvoke).toHaveBeenCalledWith("get_sensors");
	});

	it("subscribes to sensor_update and forwards payload", async () => {
		const mockedListen = vi.mocked(listen);
		let forwardedPayload: SensorData | null = null;
		const unlisten = vi.fn();
		mockedListen.mockImplementation(async (_event, callback) => {
			callback({ payload: sampleSensors } as { payload: SensorData });
			return unlisten;
		});

		const cleanup = await listenToSensorUpdates((payload) => {
			forwardedPayload = payload;
		});

		expect(mockedListen).toHaveBeenCalledWith(
			SENSOR_UPDATE_EVENT,
			expect.any(Function),
		);
		expect(forwardedPayload).toEqual(sampleSensors);
		cleanup();
		expect(unlisten).toHaveBeenCalledTimes(1);
	});
});
