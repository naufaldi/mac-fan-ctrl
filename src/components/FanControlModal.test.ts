import { render, screen } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import FanControlModal from "./FanControlModal.svelte";

vi.mock("$lib/tauriCommands", () => ({
	setFanConstantRpm: vi.fn().mockResolvedValue(undefined),
	setFanSensorControl: vi.fn().mockResolvedValue(undefined),
	installHelper: vi.fn(),
	reconnectWriter: vi.fn(),
}));

import { setFanConstantRpm } from "$lib/tauriCommands";

const fan = {
	index: 0,
	label: "Fan 0",
	actual: 2000,
	min: 1200,
	max: 6550,
	target: 2000,
	mode: "auto" as const,
};

const sensors = [
	{
		key: "TC0D",
		name: "CPU Die",
		value: 55,
		unit: "C" as const,
		sensor_type: "Cpu" as const,
	},
];

describe("FanControlModal", () => {
	it("submits clamped constant RPM when OK is clicked", async () => {
		const user = userEvent.setup();
		const onclose = vi.fn();

		render(FanControlModal, {
			props: {
				fan,
				sensors,
				onclose,
			},
		});

		const rpmInput = screen.getByRole("spinbutton", {
			name: /Target RPM for Fan 0/i,
		});
		await user.clear(rpmInput);
		await user.type(rpmInput, "4000");
		await user.click(screen.getByRole("button", { name: "OK" }));

		expect(setFanConstantRpm).toHaveBeenCalledWith(0, 4000);
		expect(onclose).toHaveBeenCalledTimes(1);
	});
});
