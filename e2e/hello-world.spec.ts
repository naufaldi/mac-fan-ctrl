import { expect, test } from "@playwright/test";

test("hello world ping flow", async ({ page }) => {
	await page.goto("/");
	await expect(page.getByRole("heading", { level: 1 })).toHaveText(
		"Hello from Svelte",
	);
	await page.getByRole("button", { name: "Ping backend" }).click();
	await expect(page.getByTestId("ping-response")).toBeVisible();
});
