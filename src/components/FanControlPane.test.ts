import { render, screen } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import type { FanData, Sensor, SensorData } from "$lib/types";
import FanControlPane from "./FanControlPane.svelte";

vi.mock("$lib/tauriCommands", () => ({
	getFanControlConfigs: vi.fn().mockResolvedValue({}),
	setFanAuto: vi.fn().mockResolvedValue(undefined),
	setFanConstantRpm: vi.fn().mockResolvedValue(undefined),
	requestPrivilegeRestart: vi.fn(),
}));

const autoFan: FanData = {
	index: 0,
	label: "Fan 0",
	actual: 2000,
	min: 1200,
	max: 6550,
	target: 1200,
	mode: "auto",
};

const forcedFan: FanData = {
	...autoFan,
	mode: "forced",
	target: 4000,
};

const sensors: Sensor[] = [
	{
		key: "TC0D",
		name: "CPU Die",
		value: 55,
		unit: "C",
		sensor_type: "Cpu",
	},
];

const toDesignToken = (fan: FanData) => ({
	id: `fan-${fan.index}`,
	fanIndex: fan.index,
	label: fan.label,
	value: Math.round(fan.actual),
	unit: "rpm" as const,
	status: "normal" as const,
	minRpm: Math.round(fan.min),
	maxRpm: Math.round(fan.max),
	targetRpm: Math.round(fan.target),
	controlMode: fan.mode === "forced" ? ("constant" as const) : ("auto" as const),
});

describe("FanControlPane", () => {
	it("shows Auto active when fan mode is auto", () => {
		render(FanControlPane, {
			props: {
				fans: [toDesignToken(autoFan)],
				rawFans: [autoFan],
				sensors,
				hasWriteAccess: true,
				fanControlAvailable: true,
			},
		});

		expect(screen.getByRole("button", { name: "Set Fan 0 to auto mode" })).toHaveClass(
			"text-(--control-active-text)",
		);
		expect(screen.getByRole("button", { name: "Set Fan 0 to custom mode" })).not.toHaveClass(
			"text-(--control-active-text)",
		);
	});

	it("shows Custom active when fan mode is forced", () => {
		render(FanControlPane, {
			props: {
				fans: [toDesignToken(forcedFan)],
				rawFans: [forcedFan],
				sensors,
				hasWriteAccess: true,
				fanControlAvailable: true,
			},
		});

		const customButton = screen.getByRole("button", { name: "Set Fan 0 to custom mode" });
		expect(customButton).toHaveClass("text-(--control-active-text)");
		expect(customButton).toHaveTextContent("Constant value of 4000");
		expect(screen.getByRole("button", { name: "Set Fan 0 to auto mode" })).not.toHaveClass(
			"text-(--control-active-text)",
		);
	});

	it("calls setFanAuto when Auto is clicked", async () => {
		const user = userEvent.setup();
		const { setFanAuto } = await import("$lib/tauriCommands");

		render(FanControlPane, {
			props: {
				fans: [toDesignToken(forcedFan)],
				rawFans: [forcedFan],
				sensors,
				hasWriteAccess: true,
				fanControlAvailable: true,
			},
		});

		await user.click(screen.getByRole("button", { name: "Set Fan 0 to auto mode" }));
		expect(setFanAuto).toHaveBeenCalledWith(0);
	});

	it("hides write controls when fan control is unavailable", () => {
		render(FanControlPane, {
			props: {
				fans: [toDesignToken(autoFan)],
				rawFans: [autoFan],
				sensors,
				hasWriteAccess: false,
				fanControlAvailable: false,
			},
		});

		expect(screen.queryByRole("button", { name: "Set Fan 0 to auto mode" })).not.toBeInTheDocument();
		expect(screen.queryByRole("button", { name: "Set Fan 0 to custom mode" })).not.toBeInTheDocument();
		expect(screen.getByText("Monitoring only")).toBeVisible();
	});
});
