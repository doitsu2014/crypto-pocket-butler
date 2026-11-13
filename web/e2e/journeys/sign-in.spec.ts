import { expect, test } from "@playwright/test";
import { capture } from "../support/capture";
import { mockSupabaseAuth, submitSignIn } from "../support/session";

/**
 * The unauthenticated surface: what a visitor sees before any session exists.
 *
 * BR3.1 — sign-in is the only way in, and a failed attempt keeps the visitor on
 * the form rather than pretending a session was created.
 */

test("a visitor sees the sign-in form", async ({ page }, testInfo) => {
  await mockSupabaseAuth(page);
  await page.goto("/");

  await expect(page.locator('input[type="email"]')).toBeVisible();
  await expect(page.locator('input[type="password"]')).toBeVisible();
  await expect(page.getByRole("button", { name: /sign in/i })).toBeVisible();

  await capture(page, testInfo, "01-sign-in-form");
});

test("invalid credentials keep the visitor on the form", async ({ page }, testInfo) => {
  await mockSupabaseAuth(page, { acceptCredentials: false });
  await submitSignIn(page);

  // Still the form, no dashboard: the session store never reached "signed-in".
  await expect(page).not.toHaveURL(/\/dashboard/);
  await expect(page.locator('input[type="email"]')).toBeVisible();

  await capture(page, testInfo, "02-invalid-credentials");
});
