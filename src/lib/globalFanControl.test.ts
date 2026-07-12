import { describe, expect, it } from "vitest";
import {
	clampRpmTarget,
	deriveGlobalFanMode,
	getSharedFanRange,
	toGlobalFanControlState,
} from "./globalFanControl";
import type { FanData } from "./types";

const fan = (overrides: Partial<FanData>): FanData => ({
	index: 0,
	label: "System Fan",
	actual: 2400,
	min: 1200,
	max: 6500,
	target: 2400,
	mode: "auto",
	...overrides,
});

describe("globalFanControl", () => {
	it("uses the shared RPM range across all detected fans", () => {
		const range = getSharedFanRange([
			fan({ index: 0, min: 1200, max: 6500 }),
			fan({ index: 1, min: 1600, max: 6200 }),
		]);

		expect(range).toEqual({ kind: "ready", minRpm: 1600, maxRpm: 6200 });
	});

	it("disables control when fans do not share a safe range", () => {
		const state = toGlobalFanControlState([
			fan({ index: 0, min: 4000, max: 4500 }),
			fan({ index: 1, min: 1200, max: 3000 }),
		]);

		expect(state.canControl).toBe(false);
		expect(state.disabledReason).toBe(
			"Detected fans do not share a safe RPM range.",
		);
	});

	it("clamps the target to the shared range", () => {
		const range = getSharedFanRange([
			fan({ index: 0, min: 1200, max: 6500 }),
			fan({ index: 1, min: 1600, max: 6200 }),
		]);

		expect(clampRpmTarget(1000, range)).toBe(1600);
		expect(clampRpmTarget(7000, range)).toBe(6200);
		expect(clampRpmTarget(3200, range)).toBe(3200);
	});

	it("reports manual mode when any fan is already forced", () => {
		expect(
			deriveGlobalFanMode([
				fan({ index: 0, mode: "auto" }),
				fan({ index: 1, mode: "forced" }),
			]),
		).toBe("manual");
	});

	it("uses the average current RPM as the default automatic target", () => {
		const state = toGlobalFanControlState([
			fan({ index: 0, actual: 2400 }),
			fan({ index: 1, actual: 2600 }),
		]);

		expect(state.currentRpm).toBe(2500);
		expect(state.targetRpm).toBe(2500);
		expect(state.mode).toBe("auto");
	});
});
