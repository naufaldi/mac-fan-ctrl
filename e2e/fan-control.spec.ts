import { expect, test } from "@playwright/test";

test.describe("FanGuard global fan control", () => {
	test("applies one manual RPM target to all detected fans", async ({ page }) => {
		await page.goto("/");

		await expect(page.getByText("1200", { exact: true })).toBeVisible();
		await expect(page.getByText("6550", { exact: true })).toBeVisible();
		await expect(page.getByText(/Current average: [\d,]+ RPM/)).toBeVisible();
		await page.getByRole("button", { name: "Manual" }).click();
		await page.getByLabel("Target RPM").fill("5000");
		await expect(page.getByText("5000 RPM", { exact: true })).toBeVisible();
		await page.getByRole("button", { name: "Apply" }).click();

		await expect(page.getByRole("alert")).toHaveCount(0);
		await expect(page.getByText("Applied 5000 RPM to 3 fans.")).toBeVisible();
		await expect(page.getByText("System Fan 1")).toBeVisible();
		await expect(page.getByText("System Fan 2")).toBeVisible();
		await expect(page.getByText("System Fan 3")).toBeVisible();
		await expect(page.getByText("Synced")).toHaveCount(3);
	});

	test("restores automatic control for all detected fans", async ({ page }) => {
		await page.goto("/");

		await page.getByRole("button", { name: "Manual" }).click();
		await page.getByLabel("Target RPM").fill("5000");
		await page.getByRole("button", { name: "Apply" }).click();
		await page.getByRole("button", { name: "Restore Auto" }).click();

		await expect(page.getByText("Restored automatic control for 3 fans.")).toBeVisible();
		await expect(
			page.locator('[role="row"]').filter({ hasText: "System Fan 1" }).getByText("Auto", { exact: true }),
		).toBeVisible();
		await expect(
			page.locator('[role="row"]').filter({ hasText: "System Fan 2" }).getByText("Auto", { exact: true }),
		).toBeVisible();
		await expect(
			page.locator('[role="row"]').filter({ hasText: "System Fan 3" }).getByText("Auto", { exact: true }),
		).toBeVisible();
	});
});
