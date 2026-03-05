import type { Sensor, SensorData, SensorType } from "./types";

export const SUMMARY_SENSOR_LIMIT = 4;

export function shouldShowReadMore(detailsCount: number): boolean {
	return detailsCount > SUMMARY_SENSOR_LIMIT;
}

export function getReadMoreLabel(
	expanded: boolean,
	detailsCount: number,
): string {
	if (expanded) {
		return "Show Less ▲";
	}

	return `Read More (${Math.max(detailsCount - SUMMARY_SENSOR_LIMIT, 0)}) ▼`;
}

const SENSOR_TYPE_ORDER: Record<SensorType, number> = {
	Cpu: 0,
	Gpu: 1,
	Memory: 2,
	Storage: 3,
	Battery: 4,
	Power: 5,
	Trackpad: 6,
	Other: 7,
};

function compareSensors(left: Sensor, right: Sensor): number {
	const typeDelta =
		SENSOR_TYPE_ORDER[left.sensor_type] - SENSOR_TYPE_ORDER[right.sensor_type];
	if (typeDelta !== 0) {
		return typeDelta;
	}

	return left.name.localeCompare(right.name);
}

function exists<T>(value: T | null): value is T {
	return value !== null;
}

function average(values: number[]): number | null {
	if (values.length === 0) {
		return null;
	}

	return values.reduce((sum, value) => sum + value, 0) / values.length;
}

function buildCpuAverageSensor(details: Sensor[]): Sensor {
	const cpuCoreValues = details
		.filter(
			(sensor) =>
				sensor.sensor_type === "Cpu" &&
				sensor.value !== null &&
				sensor.name.toLowerCase().includes("core"),
		)
		.map((sensor) => sensor.value)
		.filter((value): value is number => value !== null);

	const cpuValues = details
		.filter((sensor) => sensor.sensor_type === "Cpu" && sensor.value !== null)
		.map((sensor) => sensor.value)
		.filter((value): value is number => value !== null);

	return {
		key: "cpu-core-average",
		name: "CPU Core Average",
		value: average(cpuCoreValues.length > 0 ? cpuCoreValues : cpuValues),
		unit: "C",
		sensor_type: "Cpu",
	};
}

function buildBatterySensor(details: Sensor[]): Sensor {
	const exactBattery = details.find(
		(sensor) => sensor.sensor_type === "Battery" && sensor.name === "Battery",
	);
	const fallbackBattery = details.find(
		(sensor) => sensor.sensor_type === "Battery",
	);
	const selectedBattery = exactBattery ?? fallbackBattery ?? null;

	return {
		key: selectedBattery?.key ?? "battery",
		name: "Battery",
		value: selectedBattery?.value ?? null,
		unit: "C",
		sensor_type: "Battery",
	};
}

function buildGpuAverageSensor(details: Sensor[]): Sensor {
	const explicitAverage = details.find(
		(sensor) =>
			sensor.sensor_type === "Gpu" &&
			sensor.value !== null &&
			(sensor.name.toLowerCase().includes("average") ||
				sensor.name.toLowerCase().includes("avg")),
	);

	const gpuValues = details
		.filter((sensor) => sensor.sensor_type === "Gpu" && sensor.value !== null)
		.map((sensor) => sensor.value)
		.filter((value): value is number => value !== null);

	return {
		key: explicitAverage?.key ?? "gpu-average",
		name: "GPU Average",
		value: explicitAverage?.value ?? average(gpuValues),
		unit: "C",
		sensor_type: "Gpu",
	};
}

export function getSummarySensorsForDisplay(sensorData: SensorData): Sensor[] {
	const details = sensorData.details;

	if (details.length === 0) {
		return [
			buildCpuAverageSensor(details),
			buildBatterySensor(details),
			buildGpuAverageSensor(details),
		];
	}

	return [
		buildCpuAverageSensor(details),
		buildBatterySensor(details),
		buildGpuAverageSensor(details),
	];
}

export function getDetailSensorsInDisplayOrder(sensorData: SensorData): Sensor[] {
	return [...sensorData.details].sort(compareSensors);
}
