import type { Sensor, SensorData, SensorType } from "./types";

export const SUMMARY_SENSOR_LIMIT = 4;

type CatalogRow = {
	key: string;
	name: string;
	unit: Sensor["unit"];
	sensor_type: SensorType;
};

const TARGET_CATALOG: ReadonlyArray<CatalogRow> = [
	{ key: "TB0T", name: "Battery", unit: "C", sensor_type: "Battery" },
	{ key: "TCPUAVG", name: "CPU Core Average", unit: "C", sensor_type: "Cpu" },
	{ key: "TP0b", name: "CPU Efficiency Cluster 1", unit: "C", sensor_type: "Cpu" },
	{ key: "TP1b", name: "CPU Efficiency Cluster 2", unit: "C", sensor_type: "Cpu" },
	{ key: "TP2b", name: "CPU Performance Cluster", unit: "C", sensor_type: "Cpu" },
	{ key: "TG0D", name: "GPU Cluster 1", unit: "C", sensor_type: "Gpu" },
	{ key: "TG0P", name: "GPU Cluster 2", unit: "C", sensor_type: "Gpu" },
	{ key: "TGAVG", name: "GPU Cluster Average", unit: "C", sensor_type: "Gpu" },
	{ key: "Tm0P", name: "Mainboard", unit: "C", sensor_type: "Other" },
	{ key: "TM0P", name: "Memory Bank 1", unit: "C", sensor_type: "Memory" },
	{ key: "TM1P", name: "Memory Bank 2", unit: "C", sensor_type: "Memory" },
	{ key: "TPCD", name: "Power Manager Die Average", unit: "C", sensor_type: "Power" },
	{ key: "PDTR", name: "Power Supply Proximity", unit: "W", sensor_type: "Power" },
	{ key: "Ts0P", name: "Trackpad", unit: "C", sensor_type: "Trackpad" },
	{ key: "DISK_SECTION", name: "Disk Drives:", unit: "C", sensor_type: "Storage" },
	{ key: "TN0n", name: "APPLE SSD", unit: "C", sensor_type: "Storage" },
];

/** Sensor keys relevant for fan control decisions (heat sources, not peripherals). */
const FAN_CONTROL_SENSOR_KEYS: ReadonlySet<string> = new Set([
	"TCPUAVG", // CPU Core Average
	"TP0b",    // CPU Efficiency Cluster 1
	"TP1b",    // CPU Efficiency Cluster 2
	"TP2b",    // CPU Performance Cluster
	"TGAVG",   // GPU Cluster Average
	"TG0D",    // GPU Cluster 1
	"TG0P",    // GPU Cluster 2
	"TPCD",    // Power Manager Die Average
	"TB0T",    // Battery
]);

export function getFanControlSensors(sensors: Sensor[]): Sensor[] {
	return sensors.filter(
		(s) => s.unit === "C" && s.value !== null && FAN_CONTROL_SENSOR_KEYS.has(s.key),
	);
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

function isCpuAverageSensor(sensor: Sensor): boolean {
	return sensor.key === "TCPUAVG" || sensor.name === "CPU Core Average";
}

type CpuCoreKind = "efficiency" | "performance" | "generic";

type CpuCoreDescriptor = {
	kind: CpuCoreKind;
	index: number;
};

function toCpuCoreDescriptor(sensor: Sensor): CpuCoreDescriptor | null {
	if (sensor.sensor_type !== "Cpu" || isCpuAverageSensor(sensor)) {
		return null;
	}

	const normalizedName = sensor.name.trim();
	if (/^dynamic cpu /i.test(normalizedName)) {
		return null;
	}

	const efficiencyNameMatch = normalizedName.match(/^CPU Efficiency Core (\d+)$/i);
	if (efficiencyNameMatch) {
		return { kind: "efficiency", index: Number.parseInt(efficiencyNameMatch[1], 10) };
	}

	const performanceNameMatch = normalizedName.match(/^CPU Performance Core (\d+)$/i);
	if (performanceNameMatch) {
		return { kind: "performance", index: Number.parseInt(performanceNameMatch[1], 10) };
	}

	const genericNameMatch = normalizedName.match(/^CPU Core (\d+)$/i);
	if (genericNameMatch) {
		return { kind: "generic", index: Number.parseInt(genericNameMatch[1], 10) };
	}

	const normalizedKey = sensor.key.toUpperCase();
	const efficiencyKeyMatch = normalizedKey.match(/^TCE(\d+)$/);
	if (efficiencyKeyMatch) {
		return { kind: "efficiency", index: Number.parseInt(efficiencyKeyMatch[1], 10) };
	}

	const performanceKeyMatch = normalizedKey.match(/^TCP(\d+)$/);
	if (performanceKeyMatch) {
		return { kind: "performance", index: Number.parseInt(performanceKeyMatch[1], 10) };
	}

	const genericKeyMatch = normalizedKey.match(/^TC(\d+)C$/);
	if (genericKeyMatch) {
		return { kind: "generic", index: Number.parseInt(genericKeyMatch[1], 10) + 1 };
	}

	return null;
}

const CPU_CORE_KIND_ORDER: Record<CpuCoreKind, number> = {
	efficiency: 0,
	performance: 1,
	generic: 2,
};

function compareCpuCoreDescriptors(
	left: CpuCoreDescriptor,
	right: CpuCoreDescriptor,
): number {
	const kindDelta = CPU_CORE_KIND_ORDER[left.kind] - CPU_CORE_KIND_ORDER[right.kind];
	if (kindDelta !== 0) {
		return kindDelta;
	}

	return left.index - right.index;
}

function getCpuSensorsForDisplay(details: Sensor[]): Sensor[] {
	const averageSensor = details.find(isCpuAverageSensor) ?? buildCpuAverageSensor(details);
	const displayCpuCores = details
		.filter((sensor) => sensor.sensor_type === "Cpu")
		.map((sensor) => ({ sensor, descriptor: toCpuCoreDescriptor(sensor) }))
		.filter(
			(
				entry,
			): entry is {
				sensor: Sensor;
				descriptor: CpuCoreDescriptor;
			} => entry.descriptor !== null,
		)
		.sort((left, right) =>
			compareCpuCoreDescriptors(left.descriptor, right.descriptor),
		)
		.map((entry) => entry.sensor);

	const uniqueByIdentity = new Map(
		[averageSensor, ...displayCpuCores].map((sensor) => [
			`${sensor.key}::${sensor.name}`,
			sensor,
		]),
	);

	return [...uniqueByIdentity.values()];
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
	const cpuSensors = getCpuSensorsForDisplay(completeDetails);
	const byKey = new Map(completeDetails.map((sensor) => [sensor.key, sensor]));
	const byName = new Map(completeDetails.map((sensor) => [sensor.name, sensor]));
	const orderedCatalog = TARGET_CATALOG.flatMap((row) => {
		if (row.key === "TCPUAVG") {
			return cpuSensors;
		}
		return [byKey.get(row.key) ?? byName.get(row.name) ?? toPlaceholderSensor(row)];
	});
	return orderedCatalog.filter(
		(s) => s.unit !== "W" && (s.source !== "placeholder" || s.sensor_type === "Storage"),
	);
}

export function isPerCoreTemperatureUnavailable(
	sensorData: SensorData | null,
): boolean {
	return sensorData?.diagnostics?.per_core_cpu_temp_available === false;
}
