import type { FanControlMode, SensorData } from "./designTokens";
import { formatTemperature } from "./format";

export type SensorPartitions = {
	fanSensors: SensorData[];
	temperatureSensors: SensorData[];
};

export type FanRowData = {
	id: string;
	fanIndex: number;
	label: string;
	minRpm: number;
	currentRpm: number | null;
	maxRpm: number;
	controlMode: FanControlMode;
	targetRpm: number;
};

const defaultFanBounds: Record<string, { minRpm: number; maxRpm: number }> = {
	"fan-left": { minRpm: 1200, maxRpm: 5800 },
	"fan-right": { minRpm: 1200, maxRpm: 6200 },
};

const defaultBounds = { minRpm: 1000, maxRpm: 6000 };

export const partitionSensors = (sensors: SensorData[]): SensorPartitions => ({
	fanSensors: sensors.filter((sensor) => sensor.unit === "rpm"),
	temperatureSensors: sensors.filter((sensor) => sensor.unit === "celsius"),
});

export const toFanRowData = (fan: SensorData): FanRowData => {
	const bounds = defaultFanBounds[fan.id] ?? defaultBounds;
	const currentRpm = fan.value === null ? null : Math.round(fan.value);
	const targetRpm = fan.targetRpm ?? currentRpm ?? bounds.minRpm;

	return {
		id: fan.id,
		fanIndex: fan.fanIndex ?? 0,
		label: fan.label,
		minRpm: fan.minRpm ?? bounds.minRpm,
		currentRpm,
		maxRpm: fan.maxRpm ?? bounds.maxRpm,
		controlMode: fan.controlMode ?? "auto",
		targetRpm,
	};
};

export const toFanRows = (fans: SensorData[]): FanRowData[] =>
	fans.map(toFanRowData);

export const formatSensorValue = (sensor: SensorData): string =>
	sensor.value === null ? "N/A" : formatTemperature(sensor.value);
