import type { Sensor, SensorData, SensorType } from "./types";

export const SUMMARY_SENSOR_LIMIT = 4;

type CatalogRow = {
	key: string;
	name: string;
	unit: Sensor["unit"];
	sensor_type: SensorType;
};

const TARGET_CATALOG: ReadonlyArray<CatalogRow> = [
	{ key: "TW0P", name: "Airport Proximity", unit: "C", sensor_type: "Other" },
	{ key: "TB0T", name: "Battery", unit: "C", sensor_type: "Battery" },
	{ key: "TB1T", name: "Battery Gas Gauge", unit: "C", sensor_type: "Battery" },
	{ key: "TCPUAVG", name: "CPU Core Average", unit: "C", sensor_type: "Cpu" },
	{ key: "TCE1", name: "CPU Efficiency Core 1", unit: "C", sensor_type: "Cpu" },
	{ key: "TCE2", name: "CPU Efficiency Core 2", unit: "C", sensor_type: "Cpu" },
	{ key: "TCP1", name: "CPU Performance Core 1", unit: "C", sensor_type: "Cpu" },
	{ key: "TCP2", name: "CPU Performance Core 2", unit: "C", sensor_type: "Cpu" },
	{ key: "TCP3", name: "CPU Performance Core 3", unit: "C", sensor_type: "Cpu" },
	{ key: "TCP4", name: "CPU Performance Core 4", unit: "C", sensor_type: "Cpu" },
	{ key: "TCP5", name: "CPU Performance Core 5", unit: "C", sensor_type: "Cpu" },
	{ key: "TCP6", name: "CPU Performance Core 6", unit: "C", sensor_type: "Cpu" },
	{ key: "TG0D", name: "GPU Cluster 1", unit: "C", sensor_type: "Gpu" },
	{ key: "TG0P", name: "GPU Cluster 2", unit: "C", sensor_type: "Gpu" },
	{ key: "TGAVG", name: "GPU Cluster Average", unit: "C", sensor_type: "Gpu" },
	{ key: "TM0P", name: "Memory Bank 1", unit: "C", sensor_type: "Memory" },
	{ key: "TM1P", name: "Memory Bank 2", unit: "C", sensor_type: "Memory" },
	{ key: "TPCD", name: "Power Manager Die Average", unit: "C", sensor_type: "Power" },
	{ key: "PDTR", name: "Power Supply Proximity", unit: "W", sensor_type: "Power" },
	{ key: "Ts0P", name: "Trackpad", unit: "C", sensor_type: "Trackpad" },
	{ key: "Ts1P", name: "Trackpad Actuator", unit: "C", sensor_type: "Trackpad" },
	{ key: "DISK_SECTION", name: "Disk Drives:", unit: "C", sensor_type: "Storage" },
	{ key: "SSD", name: "APPLE SSD", unit: "C", sensor_type: "Storage" },
];

const TARGET_SENSOR_NAMES = new Set(TARGET_CATALOG.map((row) => row.name));
const TARGET_SENSOR_KEYS = new Set(TARGET_CATALOG.map((row) => row.key));

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

export function shouldShowReadMore(detailsCount: number): boolean {
	return detailsCount > SUMMARY_SENSOR_LIMIT;
}

export function getReadMoreLabel(expanded: boolean, detailsCount: number): string {
	if (expanded) {
		return "Show Less ▲";
	}

	return `Read More (${Math.max(detailsCount - SUMMARY_SENSOR_LIMIT, 0)}) ▼`;
}

function compareSensors(left: Sensor, right: Sensor): number {
	const typeDelta =
		SENSOR_TYPE_ORDER[left.sensor_type] - SENSOR_TYPE_ORDER[right.sensor_type];
	if (typeDelta !== 0) {
		return typeDelta;
	}

	return left.name.localeCompare(right.name);
}

function average(values: number[]): number | null {
	if (values.length === 0) {
		return null;
	}

	return values.reduce((sum, value) => sum + value, 0) / values.length;
}

function toPlaceholderSensor(row: CatalogRow): Sensor {
	return {
		key: row.key,
		name: row.name,
		value: null,
		unit: row.unit,
		sensor_type: row.sensor_type,
		source: "placeholder",
		null_reason: "placeholder",
	};
}

export function buildCpuAverageSensor(details: Sensor[]): Sensor {
	const cpuCoreValues = details
		.filter(
			(sensor) =>
				sensor.sensor_type === "Cpu" &&
				sensor.value !== null &&
				sensor.name.toLowerCase().includes("core") &&
				!sensor.name.toLowerCase().includes("average"),
		)
		.map((sensor) => sensor.value)
		.filter((value): value is number => value !== null);

	const cpuValues = details
		.filter((sensor) => sensor.sensor_type === "Cpu" && sensor.value !== null)
		.map((sensor) => sensor.value)
		.filter((value): value is number => value !== null);

	return {
		key: "TCPUAVG",
		name: "CPU Core Average",
		value: average(cpuCoreValues.length > 0 ? cpuCoreValues : cpuValues),
		unit: "C",
		sensor_type: "Cpu",
		source: "derived",
		null_reason: cpuValues.length === 0 ? "unsupported" : null,
	};
}

export function buildBatterySensor(details: Sensor[]): Sensor {
	const exactBattery = details.find(
		(sensor) => sensor.sensor_type === "Battery" && sensor.name === "Battery",
	);
	const fallbackBattery = details.find(
		(sensor) => sensor.sensor_type === "Battery",
	);
	const selectedBattery = exactBattery ?? fallbackBattery ?? null;

	return {
		key: selectedBattery?.key ?? "TB0T",
		name: "Battery",
		value: selectedBattery?.value ?? null,
		unit: "C",
		sensor_type: "Battery",
		source: selectedBattery?.source ?? "placeholder",
		null_reason: selectedBattery?.null_reason ?? "placeholder",
	};
}

export function buildGpuAverageSensor(details: Sensor[]): Sensor {
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
		key: explicitAverage?.key ?? "TGAVG",
		name: "GPU Cluster Average",
		value: explicitAverage?.value ?? average(gpuValues),
		unit: "C",
		sensor_type: "Gpu",
		source: explicitAverage?.source ?? "derived",
		null_reason: gpuValues.length === 0 ? "unsupported" : null,
	};
}

function ensureComputedRows(details: Sensor[]): Sensor[] {
	const withCpuAverage = details.some((sensor) => sensor.name === "CPU Core Average")
		? details
		: [...details, buildCpuAverageSensor(details)];

	return withCpuAverage.some((sensor) => sensor.name === "GPU Cluster Average")
		? withCpuAverage
		: [...withCpuAverage, buildGpuAverageSensor(withCpuAverage)];
}

export function getSummarySensorsForDisplay(sensorData: SensorData): Sensor[] {
	const details = sensorData.details;
	return [
		buildCpuAverageSensor(details),
		buildBatterySensor(details),
		buildGpuAverageSensor(details),
	];
}

export function getDetailSensorsInDisplayOrder(sensorData: SensorData): Sensor[] {
	return [...sensorData.details].sort(compareSensors);
}

export function getAllSensorsForDisplay(sensorData: SensorData): Sensor[] {
	const completeDetails = ensureComputedRows(sensorData.details);
	const byKey = new Map(completeDetails.map((sensor) => [sensor.key, sensor]));
	const byName = new Map(completeDetails.map((sensor) => [sensor.name, sensor]));
	const orderedCatalog = TARGET_CATALOG.map(
		(row) => byKey.get(row.key) ?? byName.get(row.name) ?? toPlaceholderSensor(row),
	);
	const additionalSensors = completeDetails
		.filter(
			(sensor) =>
				!TARGET_SENSOR_KEYS.has(sensor.key) && !TARGET_SENSOR_NAMES.has(sensor.name),
		)
		.sort(compareSensors);

	return [...orderedCatalog, ...additionalSensors];
}
