import { afterEach, describe, expect, it, vi } from "vitest";

const check = vi.fn();
const relaunch = vi.fn();

vi.mock("@tauri-apps/plugin-updater", () => ({
	check,
}));

vi.mock("@tauri-apps/plugin-process", () => ({
	relaunch,
}));

describe("checkForUpdate", () => {
	afterEach(() => {
		vi.resetModules();
		vi.clearAllMocks();
		vi.unstubAllEnvs();
	});

	it("does not call the self-updater for App Store distribution", async () => {
		vi.stubEnv("VITE_FANGUARD_DISTRIBUTION", "app-store");
		const { checkForUpdate } = await import("./updater");

		const result = await checkForUpdate();

		expect(result).toEqual({ status: "managed-by-app-store" });
		expect(check).not.toHaveBeenCalled();
	});
});
