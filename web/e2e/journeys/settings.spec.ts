import { expect, test } from "@playwright/test";
import { capture } from "../support/capture";
import { expectNoUnmatched, mockGateway } from "../support/gateway";
import { mockSupabaseAuth, submitSignIn } from "../support/session";

/**
 * Settings: the surface a person uses to see and correct what the product
 * knows about them.
 *
 * Navigation is by clicking, never by `page.goto`: the access token lives in
 * memory only (NFR9.1), so a full page load ends the session by design. A
 * journey that reloaded would be testing the sign-out path, not this one.
 */

test("the profile panel shows the stored name and email", async ({ page }, testInfo) => {
  await mockSupabaseAuth(page);
  const unmatched = await mockGateway(page);

  await submitSignIn(page);
  await expect(page).toHaveURL(/\/dashboard/);

  await page
    .getByRole("navigation", { name: "Main" })
    .getByRole("link", { name: "Settings" })
    .click();

  await expect(page.getByRole("heading", { name: "Profile" })).toBeVisible();
  await expect(page.getByText("ada@example.com")).toBeVisible();

  await capture(page, testInfo, "01-settings-profile");

  expectNoUnmatched(unmatched);
});

test("the settings navigation reaches preferences and consent", async ({ page }, testInfo) => {
  await mockSupabaseAuth(page);
  const unmatched = await mockGateway(page);

  await submitSignIn(page);
  await expect(page).toHaveURL(/\/dashboard/);

  await page
    .getByRole("navigation", { name: "Main" })
    .getByRole("link", { name: "Settings" })
    .click();
  await expect(page.getByRole("heading", { name: "Profile" })).toBeVisible();

  await page.getByTestId("settings-nav-preferences").click();
  await expect(page).toHaveURL(/\/settings\/preferences$/);
  await capture(page, testInfo, "02-settings-preferences");

  await page.getByTestId("settings-nav-consent").click();
  await expect(page).toHaveURL(/\/settings\/consent$/);
  await capture(page, testInfo, "03-settings-consent");

  expectNoUnmatched(unmatched);
});
