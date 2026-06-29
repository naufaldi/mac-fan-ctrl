import { afterEach, describe, expect, it, vi } from "vitest";
import { getDistribution, isAppStoreDistribution } from "./distribution";

describe("distribution", () => {
	afterEach(() => {
		vi.unstubAllEnvs();
	});

	it("defaults to direct distribution", () => {
		expect(getDistribution()).toBe("direct");
		expect(isAppStoreDistribution()).toBe(false);
	});

	it("detects App Store distribution from Vite env", () => {
		vi.stubEnv("VITE_FANGUARD_DISTRIBUTION", "app-store");

		expect(getDistribution()).toBe("app-store");
		expect(isAppStoreDistribution()).toBe(true);
	});
});
