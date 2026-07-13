import { expect, test } from "@playwright/test";

test.describe("FanGuard dashboard Liquid Glass layout", () => {
	test("fits the configured 960 by 540 Tauri viewport without clipping controls", async ({
		page,
	}) => {
		// Tauri's configured 960x540 window leaves roughly 486 CSS pixels for
		// the webview after the native macOS title bar.
		await page.setViewportSize({ width: 960, height: 486 });
		await page.goto("/");

		const autoButton = page.getByRole("button", { name: "Auto" });
		const manualButton = page.getByRole("button", { name: "Manual" });
		const targetSlider = page.getByRole("slider", { name: "Target RPM" });
		const telemetryHeader = page.getByText("Name", { exact: true });
		const manualBounds = await manualButton.boundingBox();
		const sliderBounds = await targetSlider.boundingBox();

		expect(manualBounds).not.toBeNull();
		expect(
			(manualBounds?.x ?? 960) + (manualBounds?.width ?? 1),
		).toBeLessThanOrEqual(960);
		await expect(autoButton).toBeInViewport();
		await expect(manualButton).toBeInViewport();
		await expect(targetSlider).toBeInViewport();
		await expect(
			page.locator('[data-rpm-tick="major"]').first(),
		).toBeInViewport();
		await expect(telemetryHeader).toBeInViewport();
		await expect(
			page.getByText("System Fan 1", { exact: true }),
		).toBeInViewport();
		expect(
			(sliderBounds?.x ?? 0) + (sliderBounds?.width ?? 960),
		).toBeLessThanOrEqual(960);
		expect(
			await page.evaluate(() => document.documentElement.scrollWidth),
		).toBeLessThanOrEqual(960);
	});

	test("renders a single-window dashboard without sidebar or fake navigation", async ({
		page,
	}) => {
		await page.goto("/");

		await expect(page.getByRole("heading", { name: "FanGuard" })).toBeVisible();
		await expect(page.getByText("Fan Control", { exact: true })).toBeVisible();
		await expect(
			page.getByRole("button", { name: "More actions" }),
		).toBeVisible();
		await expect(
			page.getByRole("searchbox", { name: "Search sensors" }),
		).toBeVisible();

		await expect(page.getByRole("button", { name: /^Fans\b/ })).toBeVisible();
		await expect(
			page.getByRole("button", { name: /^Temperature Sensors/ }),
		).toBeVisible();
		await expect(
			page.getByRole("button", { name: /^Disk Drives/ }),
		).toBeVisible();

		await expect(page.getByRole("button", { name: /Back/i })).toHaveCount(0);
		await expect(page.getByRole("button", { name: /Forward/i })).toHaveCount(0);
		await expect(
			page.getByRole("button", { name: /to custom mode$/i }),
		).toHaveCount(0);
		await expect(page.getByRole("dialog")).toHaveCount(0);

		await expect(page.getByText("SMC Connected")).toBeVisible();
		await expect(page.getByText("MacBookPro18,3")).toBeVisible();
	});

	test("collapses and expands a disclosure group", async ({ page }) => {
		await page.goto("/");

		const temperatureGroup = page.getByRole("button", {
			name: /^Temperature Sensors/,
		});
		await expect(temperatureGroup).toHaveAttribute("aria-expanded", "true");
		await expect(page.getByText("Battery", { exact: true })).toBeVisible();

		await temperatureGroup.click();
		await expect(temperatureGroup).toHaveAttribute("aria-expanded", "false");
		await expect(page.getByText("Battery", { exact: true })).not.toBeVisible();
	});

	test("filters visible telemetry rows without changing global control", async ({
		page,
	}) => {
		await page.goto("/");

		await page.getByRole("searchbox", { name: "Search sensors" }).fill("gpu");

		await expect(page.getByText("Fan Control", { exact: true })).toBeVisible();
		await expect(page.getByText("GPU Cluster (AGX)")).toBeVisible();
		await expect(page.getByText("Battery", { exact: true })).not.toBeVisible();
	});
});
