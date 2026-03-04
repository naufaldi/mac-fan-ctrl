/// <reference types="vitest/config" />
import path from "node:path";
import { svelte } from "@sveltejs/vite-plugin-svelte";
import tailwindcss from "@tailwindcss/vite";
import type { PluginOption } from "vite";
import { defineConfig } from "vite";

export default defineConfig({
	plugins: [tailwindcss(), svelte() as unknown as PluginOption],
	server: {
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
		alias: {
			"@": path.resolve(__dirname, "src"),
			$lib: path.resolve(__dirname, "src/lib"),
			$components: path.resolve(__dirname, "src/components"),
			$stores: path.resolve(__dirname, "src/stores"),
			$types: path.resolve(__dirname, "src/types"),
		},
	},
	base: "./",
});
