import { emit } from "@tauri-apps/api/event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { handleDevMockCommand, resetDevMockStateForTest } from "./devMock";
import type { FanControlConfig, SensorData } from "./types";

vi.mock("@tauri-apps/api/event", () => ({
	emit: vi.fn(),
}));

vi.mock("@tauri-apps/api/mocks", () => ({
	mockIPC: vi.fn(),
	mockWindows: vi.fn(),
}));

describe("devMock fan control", () => {
	beforeEach(() => {
		resetDevMockStateForTest();
		vi.clearAllMocks();
	});

	it("stores custom RPM configs and updates fan telemetry for multiple fans", () => {
		handleDevMockCommand("set_fan_constant_rpm", { fanIndex: 0, rpm: 5000 });
		handleDevMockCommand("set_fan_constant_rpm", { fanIndex: 1, rpm: 5000 });
		handleDevMockCommand("set_fan_constant_rpm", { fanIndex: 2, rpm: 5000 });

		const configs = handleDevMockCommand("get_fan_control_configs") as Record<
			string,
			FanControlConfig
		>;
		const sensors = handleDevMockCommand("get_sensors") as SensorData;

		expect(configs["0"]).toEqual({
			mode: "constant_rpm",
			target_rpm: 5000,
		});
		expect(configs["1"]).toEqual({
			mode: "constant_rpm",
			target_rpm: 5000,
		});
		expect(configs["2"]).toEqual({
			mode: "constant_rpm",
			target_rpm: 5000,
		});
		expect(sensors.fans[0].mode).toBe("forced");
		expect(sensors.fans[0].target).toBe(5000);
		expect(sensors.fans[1].mode).toBe("forced");
		expect(sensors.fans[1].target).toBe(5000);
		expect(sensors.fans[2].mode).toBe("forced");
		expect(sensors.fans[2].target).toBe(5000);
		expect(emit).toHaveBeenCalledWith(
			"sensor_update",
			expect.objectContaining({
				fans: expect.arrayContaining([
					expect.objectContaining({ mode: "forced", target: 5000 }),
				]),
			}),
		);
	});

	it("rejects writes for unknown fans", () => {
		expect(() =>
			handleDevMockCommand("set_fan_constant_rpm", { fanIndex: 9, rpm: 5000 }),
		).toThrow("Fan index 9 not found in development mock");
	});

	it("clamps custom RPM targets to the mock fan bounds", () => {
		handleDevMockCommand("set_fan_constant_rpm", { fanIndex: 1, rpm: 9999 });
		const highSensors = handleDevMockCommand("get_sensors") as SensorData;

		handleDevMockCommand("set_fan_constant_rpm", { fanIndex: 1, rpm: 1 });
		const lowSensors = handleDevMockCommand("get_sensors") as SensorData;

		expect(highSensors.fans[1].target).toBe(6550);
		expect(lowSensors.fans[1].target).toBe(1200);
	});

	it("restores auto config and telemetry", () => {
		handleDevMockCommand("set_fan_constant_rpm", { fanIndex: 0, rpm: 5000 });
		handleDevMockCommand("set_fan_constant_rpm", { fanIndex: 1, rpm: 5000 });
		handleDevMockCommand("set_fan_constant_rpm", { fanIndex: 2, rpm: 5000 });
		handleDevMockCommand("set_fan_auto", { fanIndex: 0 });
		handleDevMockCommand("set_fan_auto", { fanIndex: 1 });
		handleDevMockCommand("set_fan_auto", { fanIndex: 2 });

		const configs = handleDevMockCommand("get_fan_control_configs") as Record<
			string,
			FanControlConfig
		>;
		const sensors = handleDevMockCommand("get_sensors") as SensorData;

		expect(configs["0"]).toEqual({ mode: "auto" });
		expect(configs["1"]).toEqual({ mode: "auto" });
		expect(configs["2"]).toEqual({ mode: "auto" });
		expect(sensors.fans[0].mode).toBe("auto");
		expect(sensors.fans[1].mode).toBe("auto");
		expect(sensors.fans[2].mode).toBe("auto");
		expect(sensors.fans[0].target).toBe(3200);
		expect(sensors.fans[1].target).toBe(3200);
		expect(sensors.fans[2].target).toBe(3200);
	});
});
