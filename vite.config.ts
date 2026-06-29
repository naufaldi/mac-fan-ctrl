/// <reference types="vitest/config" />
import path from "node:path";
import { svelte } from "@sveltejs/vite-plugin-svelte";
import tailwindcss from "@tailwindcss/vite";
import type { PluginOption } from "vite";
import { defineConfig } from "vite";

const e2eMock = process.env.VITE_E2E_MOCK === "true";

export default defineConfig({
	plugins: [
		tailwindcss(),
		svelte() as unknown as PluginOption,
	],
	server: {
		port: 1420,
		strictPort: true,
		hmr: {
			overlay: true,
		},
	},
	test: {
		environment: "happy-dom",
		setupFiles: ["./vitest-setup.ts"],
		include: ["src/**/*.test.ts", "src/**/*.test.tsx"],
	},
	resolve: {
		conditions: ["browser"],
		alias: {
			...(e2eMock
				? {
						"@tauri-apps/api/core": path.resolve(__dirname, "src/e2e/tauriMockCore.ts"),
						"@tauri-apps/api/event": path.resolve(__dirname, "src/e2e/tauriMockCore.ts"),
						"@tauri-apps/api/window": path.resolve(__dirname, "src/e2e/mockWindow.ts"),
					}
				: {}),
			"@": path.resolve(__dirname, "src"),
			$lib: path.resolve(__dirname, "src/lib"),
			$components: path.resolve(__dirname, "src/components"),
			$stores: path.resolve(__dirname, "src/stores"),
			$types: path.resolve(__dirname, "src/types"),
		},
	},
	base: "./",
});
