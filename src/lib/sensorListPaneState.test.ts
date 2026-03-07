import { describe, expect, it } from "vitest";
import {
	getAllSensorsForDisplay,
	getDetailSensorsInDisplayOrder,
	getReadMoreLabel,
	getSummarySensorsForDisplay,
	isPerCoreTemperatureUnavailable,
	SUMMARY_SENSOR_LIMIT,
	shouldShowReadMore,
} from "./sensorListPaneState";
import type { SensorData } from "./types";

const emptyFans = { fans: [] as SensorData["fans"] };

describe("sensorListPaneState", () => {
	it("enables Read More only above summary limit", () => {
		expect(shouldShowReadMore(SUMMARY_SENSOR_LIMIT)).toBe(false);
		expect(shouldShowReadMore(SUMMARY_SENSOR_LIMIT + 1)).toBe(true);
	});

	it("builds read more/show less labels", () => {
		expect(getReadMoreLabel(false, 6)).toBe("Read More (2) ▼");
		expect(getReadMoreLabel(true, 6)).toBe("Show Less ▲");
		expect(getReadMoreLabel(false, 1)).toBe("Read More (0) ▼");
	});

	it("builds fixed top list with CPU average, Battery, and GPU average", () => {
		const sensorData: SensorData = {
			...emptyFans,
			summary: {
				cpu_package: null,
				gpu: null,
				ram: null,
				ssd: null,
			},
			details: [
				{
					key: "TC1C",
					name: "CPU Core 1",
					value: 80,
					unit: "C",
					sensor_type: "Cpu",
				},
				{
					key: "TC2C",
					name: "CPU Core 2",
					value: 100,
					unit: "C",
					sensor_type: "Cpu",
				},
				{
					key: "TG0D",
					name: "GPU Cluster 1",
					value: 70,
					unit: "C",
					sensor_type: "Gpu",
				},
				{
					key: "TG0P",
					name: "GPU Cluster 2",
					value: 90,
					unit: "C",
					sensor_type: "Gpu",
				},
				{
					key: "TB1T",
					name: "Battery",
					value: 41,
					unit: "C",
					sensor_type: "Battery",
				},
			],
		};

		const summarySensors = getSummarySensorsForDisplay(sensorData);

		expect(summarySensors.map((sensor) => sensor.name)).toEqual([
			"CPU Core Average",
			"Battery",
			"GPU Cluster Average",
		]);
		expect(summarySensors.map((sensor) => sensor.value)).toEqual([90, 41, 80]);
	});

	it("orders detailed sensors by category for read more", () => {
		const sensorData: SensorData = {
			...emptyFans,
			summary: {
				cpu_package: null,
				gpu: null,
				ram: null,
				ssd: null,
			},
			details: [
				{
					key: "PPTR",
					name: "Power Supply",
					value: 30,
					unit: "W",
					sensor_type: "Power",
				},
				{
					key: "TG0D",
					name: "GPU Die",
					value: 73,
					unit: "C",
					sensor_type: "Gpu",
				},
				{
					key: "TM0P",
					name: "Memory Bank 1",
					value: 52,
					unit: "C",
					sensor_type: "Memory",
				},
			],
		};

		expect(getDetailSensorsInDisplayOrder(sensorData).map((sensor) => sensor.sensor_type)).toEqual([
			"Gpu",
			"Memory",
			"Power",
		]);
	});

	it("combines and orders all sensors for flat display", () => {
		const sensorData: SensorData = {
			...emptyFans,
			summary: {
				cpu_package: null,
				gpu: null,
				ram: null,
				ssd: null,
			},
			details: [
				{
					key: "TW0P",
					name: "Airport Proximity",
					value: 42,
					unit: "C",
					sensor_type: "Other",
				},
				{
					key: "TB0T",
					name: "Battery",
					value: 31,
					unit: "C",
					sensor_type: "Battery",
				},
				{
					key: "TB1T",
					name: "Battery Gas Gauge",
					value: 31,
					unit: "C",
					sensor_type: "Battery",
				},
				{
					key: "TC1C",
					name: "CPU Efficiency Core 1",
					value: 80,
					unit: "C",
					sensor_type: "Cpu",
				},
				{
					key: "TC2C",
					name: "CPU Efficiency Core 2",
					value: 82,
					unit: "C",
					sensor_type: "Cpu",
				},
				{
					key: "TC3C",
					name: "CPU Performance Core 1",
					value: 90,
					unit: "C",
					sensor_type: "Cpu",
				},
				{
					key: "TG0D",
					name: "GPU Cluster 1",
					value: 60,
					unit: "C",
					sensor_type: "Gpu",
				},
				{
					key: "TG0P",
					name: "GPU Cluster 2",
					value: 58,
					unit: "C",
					sensor_type: "Gpu",
				},
			],
		};

		const allSensors = getAllSensorsForDisplay(sensorData);

		// Temperature sensors with no data on this hardware are hidden; storage sensors remain
		expect(allSensors.map((sensor) => sensor.name)).toEqual([
			"Battery",
			"CPU Core Average",
			"CPU Efficiency Core 1",
			"CPU Efficiency Core 2",
			"CPU Performance Core 1",
			"GPU Cluster 1",
			"GPU Cluster 2",
			"GPU Cluster Average",
			"Disk Drives:",
			"APPLE SSD",
		]);

		expect(allSensors.find((sensor) => sensor.name === "Disk Drives:")?.value).toBeNull();
		expect(allSensors.find((sensor) => sensor.name === "APPLE SSD")?.value).toBeNull();
		expect(allSensors.find((sensor) => sensor.name === "GPU Cluster Average")?.value).toBe(59);
	});

	it("prefers key match over name and hides sensors with no data on this hardware", () => {
		const sensorData: SensorData = {
			...emptyFans,
			summary: {
				cpu_package: null,
				gpu: null,
				ram: null,
				ssd: null,
			},
			details: [
				{
					key: "TM0P",
					name: "Memory Bank Primary",
					value: 54,
					unit: "C",
					sensor_type: "Memory",
					source: "smc",
					null_reason: null,
				},
			],
		};

		const allSensors = getAllSensorsForDisplay(sensorData);
		const memoryBank = allSensors.find((sensor) => sensor.key === "TM0P");
		const missingTrackpad = allSensors.find((sensor) => sensor.key === "Ts0P");

		expect(memoryBank?.name).toBe("Memory Bank Primary");
		expect(memoryBank?.value).toBe(54);
		// Trackpad not present on this hardware — hidden rather than shown as N/A
		expect(missingTrackpad).toBeUndefined();
	});

	it("filters noisy dynamic cpu labels from flat display", () => {
		const sensorData: SensorData = {
			...emptyFans,
			summary: {
				cpu_package: null,
				gpu: null,
				ram: null,
				ssd: null,
			},
			details: [
				{
					key: "TCPUAVG",
					name: "CPU Core Average",
					value: 65,
					unit: "C",
					sensor_type: "Cpu",
				},
				{
					key: "TC0C",
					name: "CPU Efficiency Core 1",
					value: 63,
					unit: "C",
					sensor_type: "Cpu",
				},
				{
					key: "TPD1",
					name: "Dynamic CPU TPD1",
					value: 61,
					unit: "C",
					sensor_type: "Cpu",
				},
			],
		};

		const allSensors = getAllSensorsForDisplay(sensorData);
		const names = allSensors.map((sensor) => sensor.name);

		expect(names).toContain("CPU Core Average");
		expect(names).toContain("CPU Efficiency Core 1");
		expect(names).not.toContain("Dynamic CPU TPD1");
	});

	it("uses detected cpu core rows without fixed placeholder core catalog", () => {
		const sensorData: SensorData = {
			...emptyFans,
			summary: {
				cpu_package: null,
				gpu: null,
				ram: null,
				ssd: null,
			},
			details: [
				{
					key: "TCPUAVG",
					name: "CPU Core Average",
					value: 71,
					unit: "C",
					sensor_type: "Cpu",
				},
				{
					key: "TC0C",
					name: "CPU Efficiency Core 1",
					value: 70,
					unit: "C",
					sensor_type: "Cpu",
				},
			],
		};

		const allSensors = getAllSensorsForDisplay(sensorData);
		const cpuNames = allSensors
			.filter((sensor) => sensor.sensor_type === "Cpu")
			.map((sensor) => sensor.name);

		expect(cpuNames).toEqual(["CPU Core Average", "CPU Efficiency Core 1"]);
	});

	it("reports per-core temperature unavailable when diagnostics flag is false", () => {
		const sensorData: SensorData = {
			...emptyFans,
			summary: {
				cpu_package: null,
				gpu: null,
				ram: null,
				ssd: null,
			},
			details: [],
			diagnostics: {
				model_id: "MacBookPro18,3",
				diagnostics_enabled: false,
				active_providers: ["smc"],
				unresolved: [],
				per_core_cpu_temp_available: false,
			},
		};

		expect(isPerCoreTemperatureUnavailable(sensorData)).toBe(true);
	});

	it("hides catalog sensors whose value is null (not available on this hardware)", () => {
		const sensorData: SensorData = {
			...emptyFans,
			summary: { cpu_package: null, gpu: null, ram: null, ssd: null },
			// TM0P and TM1P not present → will become null placeholders
			details: [
				{ key: "TG0D", name: "GPU Cluster 1", value: 55, unit: "C", sensor_type: "Gpu" },
			],
		};

		const names = getAllSensorsForDisplay(sensorData).map((s) => s.name);

		expect(names).not.toContain("Memory Bank 1");
		expect(names).not.toContain("Memory Bank 2");
		expect(names).toContain("GPU Cluster 1");
	});

	it("excludes uncataloged non-CPU sensors from flat display", () => {
		const sensorData: SensorData = {
			...emptyFans,
			summary: { cpu_package: null, gpu: null, ram: null, ssd: null },
			details: [
				{ key: "Tg04", name: "Dynamic GPU Tg04", value: 45, unit: "C", sensor_type: "Gpu" },
				{ key: "Tm00", name: "Dynamic Memory Tm00", value: 38, unit: "C", sensor_type: "Memory" },
				{ key: "TG0D", name: "GPU Cluster 1", value: 60, unit: "C", sensor_type: "Gpu" },
			],
		};

		const allSensors = getAllSensorsForDisplay(sensorData);
		const names = allSensors.map((s) => s.name);

		expect(names).not.toContain("Dynamic GPU Tg04");
		expect(names).not.toContain("Dynamic Memory Tm00");
		expect(names).toContain("GPU Cluster 1");
	});

	it("does not report per-core unavailable when diagnostics are missing", () => {
		const sensorData: SensorData = {
			...emptyFans,
			summary: {
				cpu_package: null,
				gpu: null,
				ram: null,
				ssd: null,
			},
			details: [],
		};

		expect(isPerCoreTemperatureUnavailable(sensorData)).toBe(false);
		expect(isPerCoreTemperatureUnavailable(null)).toBe(false);
	});
});
