import { render, screen } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
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

const renderPane = (hasWriteAccess = true) =>
	render(FanControlPane, {
		props: { rawFans: [autoFan], hasWriteAccess },
	});

describe("FanControlPane", () => {
	it("shows the shared numeric RPM range and disables Apply in Auto mode", () => {
		renderPane();

		expect(screen.getByText("1200")).toBeVisible();
		expect(screen.getByText("6550")).toBeVisible();
		expect(screen.getByText("2000 RPM")).toBeVisible();
		expect(screen.getByRole("button", { name: "Apply" })).toBeDisabled();
	});

	it("applies one manual target through the global backend command", async () => {
		const user = userEvent.setup();
		renderPane();

		await user.click(screen.getByRole("button", { name: "Manual" }));
		await user.click(screen.getByRole("button", { name: "Apply" }));

		expect(setAllFansConstantRpm).toHaveBeenCalledWith(2000);
		expect(screen.getByText("Applied 2000 RPM to 1 fan.")).toBeVisible();
	});

	it("restores all fans to automatic control", async () => {
		const user = userEvent.setup();
		renderPane();

		await user.click(screen.getByRole("button", { name: "Restore Auto" }));

		expect(restoreAllFansAuto).toHaveBeenCalledTimes(1);
		expect(screen.getByText("Restored automatic control for 1 fan.")).toBeVisible();
	});

	it("disables fan writes without administrator access", () => {
		renderPane(false);

		expect(screen.getByRole("slider", { name: "Target RPM" })).toBeDisabled();
		expect(screen.getByRole("button", { name: "Apply" })).toBeDisabled();
		expect(screen.getByRole("button", { name: "Restore Auto" })).toBeDisabled();
		expect(screen.getByText("Fan writes need administrator access.")).toBeVisible();
	});
});
