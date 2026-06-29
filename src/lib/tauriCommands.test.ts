import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { describe, expect, it, vi } from "vitest";
import {
	getFanControlConfigs,
	getPrivilegeStatus,
	getSensors,
	listenToSensorUpdates,
	SENSOR_UPDATE_EVENT,
	setFanAuto,
	setFanConstantRpm,
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

	it("maps setFanConstantRpm to set_fan_constant_rpm invoke command", async () => {
		const mockedInvoke = vi.mocked(invoke);
		mockedInvoke.mockResolvedValue(undefined);

		await setFanConstantRpm(0, 4000);

		expect(mockedInvoke).toHaveBeenCalledWith("set_fan_constant_rpm", {
			fanIndex: 0,
			rpm: 4000,
		});
	});

	it("maps setFanAuto to set_fan_auto invoke command", async () => {
		const mockedInvoke = vi.mocked(invoke);
		mockedInvoke.mockResolvedValue(undefined);

		await setFanAuto(0);

		expect(mockedInvoke).toHaveBeenCalledWith("set_fan_auto", {
			fanIndex: 0,
		});
	});

	it("maps getFanControlConfigs to get_fan_control_configs invoke command", async () => {
		const mockedInvoke = vi.mocked(invoke);
		mockedInvoke.mockResolvedValue({});

		await getFanControlConfigs();

		expect(mockedInvoke).toHaveBeenCalledWith("get_fan_control_configs");
	});

	it("maps getPrivilegeStatus to get_privilege_status invoke command", async () => {
		const mockedInvoke = vi.mocked(invoke);
		mockedInvoke.mockResolvedValue({
			has_write_access: false,
			fan_control_available: false,
			reason: "Fan control is disabled in TestFlight builds.",
		});

		const result = await getPrivilegeStatus();

		expect(result).toEqual({
			has_write_access: false,
			fan_control_available: false,
			reason: "Fan control is disabled in TestFlight builds.",
		});
		expect(mockedInvoke).toHaveBeenCalledWith("get_privilege_status");
	});
});
