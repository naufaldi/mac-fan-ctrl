import { defineConfig } from "@playwright/test";

export default defineConfig({
	testDir: "e2e",
	timeout: 30_000,
	use: {
		baseURL: "http://localhost:1420",
		trace: "off",
	},
	webServer: {
		command: "VITE_E2E_MOCK=true pnpm dev",
		url: "http://localhost:1420",
		timeout: 120_000,
		reuseExistingServer: !process.env.CI,
	},
});
