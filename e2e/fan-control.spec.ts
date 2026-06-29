import { expect, test } from "@playwright/test";

test("fan control auto to custom rpm to auto flow", async ({ page }) => {
	await page.goto("/");

	await expect(page.getByRole("button", { name: "Set Fan 0 to auto mode" })).toBeVisible({
		timeout: 15_000,
	});

	const autoButton = page.getByRole("button", { name: "Set Fan 0 to auto mode" });
	const customButton = page.getByRole("button", { name: "Set Fan 0 to custom mode" });

	await expect(autoButton).toHaveClass(/control-active-text/);
	await expect(customButton).not.toHaveClass(/control-active-text/);

	await customButton.click();
	await expect(page.getByRole("dialog")).toBeVisible();

	const rpmInput = page.getByRole("spinbutton", {
		name: /Target RPM for Fan 0/i,
	});
	await rpmInput.fill("5000");
	await page.getByRole("button", { name: "OK" }).click();

	await expect(page.getByRole("button", { name: "Set Fan 0 to custom mode" })).toHaveClass(/control-active-text/, {
		timeout: 10_000,
	});
	await expect(page.getByRole("button", { name: "Set Fan 0 to custom mode" })).toContainText(
		"Constant value of 5000",
	);
	await expect(page.getByRole("button", { name: "Set Fan 0 to auto mode" })).not.toHaveClass(/control-active-text/);

	await page.getByRole("button", { name: "Set Fan 0 to auto mode" }).click();
	await expect(page.getByRole("button", { name: "Set Fan 0 to custom mode" })).toHaveText(
		"Custom...",
		{ timeout: 10_000 },
	);
	await expect(page.getByRole("button", { name: "Set Fan 0 to auto mode" })).toHaveClass(/control-active-text/);
	await expect(page.getByRole("button", { name: "Set Fan 0 to custom mode" })).not.toHaveClass(
		/control-active-text/,
	);
});
