import { describe, expect, it } from "vitest";
import {
	getDetailSensorsInDisplayOrder,
	getReadMoreLabel,
	getSummarySensorsForDisplay,
	getAllSensorsForDisplay,
	shouldShowReadMore,
	SUMMARY_SENSOR_LIMIT,
} from "./sensorListPaneState";
import type { SensorData } from "./types";

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

		expect(allSensors.slice(0, 15).map((sensor) => sensor.name)).toEqual([
			"Airport Proximity",
			"Battery",
			"Battery Gas Gauge",
			"CPU Core Average",
			"CPU Efficiency Core 1",
			"CPU Efficiency Core 2",
			"CPU Performance Core 1",
			"CPU Performance Core 2",
			"CPU Performance Core 3",
			"CPU Performance Core 4",
			"CPU Performance Core 5",
			"CPU Performance Core 6",
			"GPU Cluster 1",
			"GPU Cluster 2",
			"GPU Cluster Average",
		]);

		expect(allSensors.find((sensor) => sensor.name === "Disk Drives:")?.value).toBeNull();
		expect(allSensors.find((sensor) => sensor.name === "APPLE SSD")?.value).toBeNull();
		expect(allSensors.find((sensor) => sensor.name === "GPU Cluster Average")?.value).toBe(59);
	});

	it("prefers key match over name and keeps placeholder diagnostics", () => {
		const sensorData: SensorData = {
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
		expect(missingTrackpad?.source).toBe("placeholder");
		expect(missingTrackpad?.null_reason).toBe("placeholder");
	});
});
