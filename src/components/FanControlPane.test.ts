import { cleanup, fireEvent, render, screen } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { FanData } from "$lib/types";
import FanControlPane from "./FanControlPane.svelte";

const { setAllFansConstantRpm, restoreAllFansAuto } = vi.hoisted(() => ({
	setAllFansConstantRpm: vi.fn().mockResolvedValue(undefined),
	restoreAllFansAuto: vi.fn().mockResolvedValue(undefined),
}));

vi.mock("$lib/tauriCommands", () => ({
	requestPrivilegeRestart: vi.fn(),
	restoreAllFansAuto,
	setAllFansConstantRpm,
}));

const autoFan: FanData = {
	index: 0,
	label: "Fan 0",
	actual: 2000,
	min: 1200,
	max: 6550,
	target: 2000,
	mode: "auto",
};

const renderPane = (
	hasWriteAccess = true,
	maxTemperatureC: number | null = 60,
) =>
	render(FanControlPane, {
		props: { rawFans: [autoFan], hasWriteAccess, maxTemperatureC },
	});

describe("FanControlPane", () => {
	beforeEach(() => {
		setAllFansConstantRpm.mockClear();
		restoreAllFansAuto.mockClear();
	});

	afterEach(() => {
		cleanup();
		vi.clearAllTimers();
		vi.useRealTimers();
	});

	it("shows the shared numeric RPM range and disables the slider in Auto mode", () => {
		renderPane();

		expect(screen.getByText("1200")).toBeVisible();
		expect(screen.getByText("6550")).toBeVisible();
		expect(screen.getByText("2000 RPM")).toBeVisible();
		expect(screen.getByRole("slider", { name: "Target RPM" })).toBeDisabled();
	});

	it("keeps a Manual draft selected when telemetry changes", async () => {
		const user = userEvent.setup();
		const view = renderPane();

		await user.click(screen.getByRole("button", { name: "Manual" }));
		await view.rerender({
			rawFans: [{ ...autoFan, actual: 2050, target: 2050 }],
			hasWriteAccess: true,
		});

		expect(screen.getByRole("button", { name: "Manual" })).toHaveAttribute(
			"aria-pressed",
			"true",
		);
	});

	it("auto-applies a stable Manual target after two seconds", async () => {
		vi.useFakeTimers();
		const user = userEvent.setup({ advanceTimers: vi.advanceTimersByTime });
		renderPane();

		await user.click(screen.getByRole("button", { name: "Manual" }));
		await fireEvent.input(screen.getByRole("slider", { name: "Target RPM" }), {
			target: { value: "5000" },
		});

		expect(screen.getByText("Applying in 2.0s")).toBeVisible();
		await vi.advanceTimersByTimeAsync(1999);
		expect(setAllFansConstantRpm).not.toHaveBeenCalled();
		await vi.advanceTimersByTimeAsync(1);
		expect(setAllFansConstantRpm).toHaveBeenCalledWith(5000);
	});

	it("restarts the two-second delay when the target changes", async () => {
		vi.useFakeTimers();
		const user = userEvent.setup({ advanceTimers: vi.advanceTimersByTime });
		renderPane();

		await user.click(screen.getByRole("button", { name: "Manual" }));
		await fireEvent.input(screen.getByRole("slider", { name: "Target RPM" }), {
			target: { value: "4000" },
		});
		await vi.advanceTimersByTimeAsync(1500);
		await fireEvent.input(screen.getByRole("slider", { name: "Target RPM" }), {
			target: { value: "5000" },
		});
		await vi.advanceTimersByTimeAsync(1999);

		expect(setAllFansConstantRpm).not.toHaveBeenCalled();
		await vi.advanceTimersByTimeAsync(1);
		expect(setAllFansConstantRpm).toHaveBeenCalledTimes(1);
		expect(setAllFansConstantRpm).toHaveBeenCalledWith(5000);
	});

	it("returns to Auto and reports a rejected hardware write", async () => {
		vi.useFakeTimers();
		setAllFansConstantRpm.mockRejectedValueOnce(
			new Error("SMC rejected target read-back"),
		);
		const user = userEvent.setup({ advanceTimers: vi.advanceTimersByTime });
		renderPane();

		await user.click(screen.getByRole("button", { name: "Manual" }));
		await fireEvent.input(screen.getByRole("slider", { name: "Target RPM" }), {
			target: { value: "5000" },
		});
		await vi.advanceTimersByTimeAsync(2000);

		expect(screen.getByRole("button", { name: "Auto" })).toHaveAttribute(
			"aria-pressed",
			"true",
		);
		expect(screen.getByText("SMC rejected target read-back")).toBeVisible();
	});

	it("cancels a pending Manual target when Auto is selected", async () => {
		vi.useFakeTimers();
		const user = userEvent.setup({ advanceTimers: vi.advanceTimersByTime });
		renderPane();

		await user.click(screen.getByRole("button", { name: "Manual" }));
		await fireEvent.input(screen.getByRole("slider", { name: "Target RPM" }), {
			target: { value: "5000" },
		});
		await user.click(screen.getByRole("button", { name: "Auto" }));
		await vi.advanceTimersByTimeAsync(2000);

		expect(restoreAllFansAuto).toHaveBeenCalledTimes(1);
		expect(setAllFansConstantRpm).not.toHaveBeenCalled();
	});

	it("restores all fans when Auto is selected", async () => {
		const user = userEvent.setup();
		renderPane();

		await user.click(screen.getByRole("button", { name: "Manual" }));
		await user.click(screen.getByRole("button", { name: "Auto" }));

		expect(restoreAllFansAuto).toHaveBeenCalledTimes(1);
		expect(
			screen.getByText("Restored automatic control for 1 fan."),
		).toBeVisible();
	});

	it("disables fan writes without administrator access", () => {
		renderPane(false);

		expect(screen.getByRole("slider", { name: "Target RPM" })).toBeDisabled();
		expect(
			screen.getByText("Fan writes need administrator access."),
		).toBeVisible();
	});

	it("shows safe 500 RPM labels and 100 RPM minor ticks", () => {
		render(FanControlPane, {
			props: {
				rawFans: [{ ...autoFan, actual: 2503, min: 2317, max: 6550 }],
				hasWriteAccess: true,
				maxTemperatureC: 60,
			},
		});

		expect(screen.getByText("2317", { exact: true })).toBeVisible();
		expect(screen.getByText("6550", { exact: true })).toBeVisible();
		expect(screen.getByText("2500", { exact: true })).toBeVisible();
		expect(screen.getByText("6000", { exact: true })).toBeVisible();
		expect(screen.queryByText("2000", { exact: true })).not.toBeInTheDocument();
		expect(
			document.querySelectorAll('[data-rpm-tick="minor"]'),
		).not.toHaveLength(0);
	});

	it("raises the Manual minimum while the system is hot", async () => {
		const user = userEvent.setup();
		render(FanControlPane, {
			props: {
				rawFans: [{ ...autoFan, actual: 3201 }],
				hasWriteAccess: true,
				maxTemperatureC: 86,
			},
		});

		await user.click(screen.getByRole("button", { name: "Manual" }));

		expect(screen.getByRole("slider", { name: "Target RPM" })).toHaveAttribute(
			"min",
			"3250",
		);
		expect(
			screen.getByText("Hot system: minimum target 3,250 RPM."),
		).toBeVisible();
	});

	it("blocks Manual when emergency cooling is active", () => {
		renderPane(true, 95);

		expect(screen.getByRole("button", { name: "Manual" })).toBeDisabled();
		expect(
			screen.getByText("Emergency cooling is active at 95°C."),
		).toBeVisible();
	});

	it("blocks Manual when temperature telemetry is unavailable", () => {
		renderPane(true, null);

		expect(screen.getByRole("button", { name: "Manual" })).toBeDisabled();
		expect(
			screen.getByText("Temperature telemetry is unavailable."),
		).toBeVisible();
	});
});
