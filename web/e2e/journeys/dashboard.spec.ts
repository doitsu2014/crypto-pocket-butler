import { expect, test } from "@playwright/test";
import { capture } from "../support/capture";
import { expectNoUnmatched, fixtures, mockGateway } from "../support/gateway";
import { mockSupabaseAuth, submitSignIn } from "../support/session";

/**
 * The signed-in shell: sign-in through the form, the gate check, and the
 * portfolio list with its valuations.
 *
 * The point of this journey is that the browser reaches a real dashboard state
 * from a blank page — not that a component renders in isolation.
 */

test("a signed-in user lands on the dashboard with their portfolios", async ({
  page,
}, testInfo) => {
  await mockSupabaseAuth(page);
  const unmatched = await mockGateway(page);

  await submitSignIn(page);

  await expect(page).toHaveURL(/\/dashboard/);
  await expect(page.getByRole("heading", { name: "Long term" })).toBeVisible();
  await expect(page.getByRole("heading", { name: "Play" })).toBeVisible();

  // The priced lines come from the valuation fixtures; the second portfolio is
  // deliberately left unpriced so its partial state is visible too.
  await expect(page.getByText("BTC")).toBeVisible();
  await expect(page.getByText("ETH")).toBeVisible();

  await capture(page, testInfo, "01-dashboard");

  expectNoUnmatched(unmatched);
});

test("an empty portfolio list shows the start-here state", async ({ page }, testInfo) => {
  await mockSupabaseAuth(page);
  const unmatched = await mockGateway(page, { portfolios: [], valuations: {} });

  await submitSignIn(page);
  await expect(page).toHaveURL(/\/dashboard/);

  await capture(page, testInfo, "02-dashboard-empty");

  expectNoUnmatched(unmatched);
});

test("signing out ends the session and returns to the form", async ({ page }, testInfo) => {
  await mockSupabaseAuth(page);
  const unmatched = await mockGateway(page);

  await submitSignIn(page);
  await expect(page).toHaveURL(/\/dashboard/);

  await page.getByRole("button", { name: "Sign out" }).click();

  await expect(page.locator('input[type="email"]')).toBeVisible();
  await expect(page).not.toHaveURL(/\/dashboard/);

  await capture(page, testInfo, "03-signed-out");

  expectNoUnmatched(unmatched);
});

// The fixture is the contract these assertions rest on; a drift in it should be
// visible here rather than as a confusing UI failure.
test("the dashboard fixture describes two portfolios", async () => {
  expect(fixtures.portfolios).toHaveLength(2);
  expect(fixtures.portfolios[0]?.name).toBe("Long term");
});
