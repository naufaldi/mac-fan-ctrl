import { describe, expect, it } from "vitest";
import {
	getDetailSensorsInDisplayOrder,
	getReadMoreLabel,
	getSummarySensorsForDisplay,
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
			"GPU Average",
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
});
