import type { FanData } from "./types";

export type GlobalFanControlMode = "auto" | "manual";

export type SharedFanRange =
	| { readonly kind: "none"; readonly reason: string }
	| {
			readonly kind: "ready";
			readonly minRpm: number;
			readonly maxRpm: number;
	  }
	| {
			readonly kind: "invalid";
			readonly minRpm: number;
			readonly maxRpm: number;
			readonly reason: string;
	  };

export type GlobalFanControlState = {
	readonly fanCount: number;
	readonly mode: GlobalFanControlMode;
	readonly currentRpm: number | null;
	readonly targetRpm: number;
	readonly range: SharedFanRange;
	readonly canControl: boolean;
	readonly disabledReason: string | null;
};

export type RpmScaleTick = {
	readonly rpm: number;
	readonly percent: number;
	readonly kind: "major" | "minor";
};

export type ManualControlConstraint =
	| { readonly kind: "blocked"; readonly reason: string }
	| {
			readonly kind: "ready";
			readonly minRpm: number;
			readonly maxRpm: number;
			readonly reason: string | null;
	  };

const roundRpm = (value: number): number => Math.round(value);

const average = (values: readonly number[]): number | null =>
	values.length === 0
		? null
		: values.reduce((sum, value) => sum + value, 0) / values.length;

const fanMinimums = (fans: readonly FanData[]): number[] =>
	fans.map((fan) => roundRpm(fan.min));

const fanMaximums = (fans: readonly FanData[]): number[] =>
	fans.map((fan) => roundRpm(fan.max));

export const getSharedFanRange = (fans: readonly FanData[]): SharedFanRange => {
	if (fans.length === 0) {
		return { kind: "none", reason: "No fans detected." };
	}

	const minRpm = Math.max(...fanMinimums(fans));
	const maxRpm = Math.min(...fanMaximums(fans));

	return minRpm <= maxRpm
		? { kind: "ready", minRpm, maxRpm }
		: {
				kind: "invalid",
				minRpm,
				maxRpm,
				reason: "Detected fans do not share a safe RPM range.",
			};
};

export const clampRpmTarget = (
	targetRpm: number,
	range: SharedFanRange,
): number => {
	if (range.kind !== "ready") {
		return roundRpm(targetRpm);
	}

	return Math.max(range.minRpm, Math.min(range.maxRpm, roundRpm(targetRpm)));
};

export const buildRpmScale = (
	minRpm: number,
	maxRpm: number,
): RpmScaleTick[] => {
	if (maxRpm <= minRpm) {
		return [];
	}

	const firstTick = Math.ceil(minRpm / 100) * 100;
	const count = Math.max(0, Math.floor((maxRpm - firstTick) / 100) + 1);

	return Array.from(
		{ length: count },
		(_, index) => firstTick + index * 100,
	).map((rpm) => ({
		rpm,
		percent: ((rpm - minRpm) / (maxRpm - minRpm)) * 100,
		kind: rpm % 500 === 0 ? "major" : "minor",
	}));
};

export const getManualControlConstraint = (
	fans: readonly FanData[],
	range: SharedFanRange,
	maxTemperatureC: number | null,
): ManualControlConstraint => {
	if (range.kind !== "ready") {
		return { kind: "blocked", reason: range.reason };
	}

	if (maxTemperatureC === null) {
		return { kind: "blocked", reason: "Temperature telemetry is unavailable." };
	}

	if (maxTemperatureC >= 95) {
		return {
			kind: "blocked",
			reason: `Emergency cooling is active at ${Math.round(maxTemperatureC)}°C.`,
		};
	}

	const highestActualRpm = Math.max(...fans.map((fan) => fan.actual));
	const thermalMinimum = Math.ceil(highestActualRpm / 50) * 50;
	const minRpm =
		maxTemperatureC >= 85
			? Math.max(range.minRpm, thermalMinimum)
			: range.minRpm;

	return minRpm <= range.maxRpm
		? {
				kind: "ready",
				minRpm,
				maxRpm: range.maxRpm,
				reason:
					maxTemperatureC >= 85
						? `Hot system: minimum target ${new Intl.NumberFormat("en-US").format(minRpm)} RPM.`
						: null,
			}
		: {
				kind: "blocked",
				reason: "Current fan speed exceeds the shared safe RPM range.",
			};
};

export const deriveGlobalFanMode = (
	fans: readonly FanData[],
): GlobalFanControlMode =>
	fans.some((fan) => fan.mode === "forced") ? "manual" : "auto";

const forcedTargets = (fans: readonly FanData[]): number[] =>
	fans
		.filter((fan) => fan.mode === "forced")
		.map((fan) => roundRpm(fan.target));

const currentRpms = (fans: readonly FanData[]): number[] =>
	fans.map((fan) => roundRpm(fan.actual));

const defaultTargetRpm = (
	fans: readonly FanData[],
	range: SharedFanRange,
): number => {
	const preferredTarget =
		average(forcedTargets(fans)) ?? average(currentRpms(fans)) ?? 0;
	return clampRpmTarget(preferredTarget, range);
};

const disabledReasonForRange = (range: SharedFanRange): string | null =>
	range.kind === "ready" ? null : range.reason;

export const toGlobalFanControlState = (
	fans: readonly FanData[],
): GlobalFanControlState => {
	const range = getSharedFanRange(fans);
	const disabledReason = disabledReasonForRange(range);

	return {
		fanCount: fans.length,
		mode: deriveGlobalFanMode(fans),
		currentRpm: average(currentRpms(fans)),
		targetRpm: defaultTargetRpm(fans, range),
		range,
		canControl: range.kind === "ready",
		disabledReason,
	};
};
