import { expect, type Page } from "@playwright/test";

/**
 * A fake Supabase Auth, answered at the network layer.
 *
 * The SPA keeps the access token in memory only (NFR9.1), so a signed-in state
 * is reached the same way a person reaches it: the sign-in form posts to the
 * auth endpoint and the client stores the answer. Nothing here injects a
 * session into the app, which is why the journey proves the real sign-in path.
 */

export const E2E_USER_ID = "3f0c2b8e-5b1a-4f2d-9c77-1a2b3c4d5e6f";
export const E2E_EMAIL = "ada@example.com";
export const E2E_PASSWORD = "correct-horse-battery";
export const E2E_ACCESS_TOKEN = "e2e-access-token";
export const E2E_REFRESH_TOKEN = "e2e-refresh-token";

export interface AuthMockOptions {
  /** When false, the token endpoint answers with invalid credentials. */
  acceptCredentials?: boolean;
}

function sessionBody() {
  const now = Math.floor(Date.now() / 1000);
  return {
    access_token: E2E_ACCESS_TOKEN,
    token_type: "bearer",
    expires_in: 3600,
    expires_at: now + 3600,
    refresh_token: E2E_REFRESH_TOKEN,
    user: {
      id: E2E_USER_ID,
      aud: "authenticated",
      role: "authenticated",
      email: E2E_EMAIL,
      email_confirmed_at: "2026-01-04T09:00:00Z",
      phone: "",
      confirmed_at: "2026-01-04T09:00:00Z",
      last_sign_in_at: "2026-11-12T08:00:00Z",
      app_metadata: { provider: "email", providers: ["email"] },
      user_metadata: {},
      identities: [],
      created_at: "2026-01-04T09:00:00Z",
      updated_at: "2026-11-12T08:00:00Z",
      is_anonymous: false,
    },
  };
}

export async function mockSupabaseAuth(page: Page, options: AuthMockOptions = {}): Promise<void> {
  const acceptCredentials = options.acceptCredentials ?? true;

  // Matched by path, not by a `**/v1/**` glob: the gateway mock answers `/v1/`
  // and a glob would also swallow `/auth/v1/token`.
  await page.route(
    (url) => url.pathname.startsWith("/auth/v1/"),
    async (route) => {
      const request = route.request();
      const url = new URL(request.url());

      if (url.pathname.endsWith("/token")) {
        if (!acceptCredentials) {
          await route.fulfill({
            status: 400,
            contentType: "application/json",
            body: JSON.stringify({
              code: 400,
              error_code: "invalid_credentials",
              msg: "Invalid login credentials",
            }),
          });
          return;
        }
        await route.fulfill({
          status: 200,
          contentType: "application/json",
          body: JSON.stringify(sessionBody()),
        });
        return;
      }

      if (url.pathname.endsWith("/user")) {
        await route.fulfill({
          status: 200,
          contentType: "application/json",
          body: JSON.stringify(sessionBody().user),
        });
        return;
      }

      if (url.pathname.endsWith("/logout")) {
        await route.fulfill({ status: 204, body: "" });
        return;
      }

      // Nothing else is expected; an empty object keeps an unexpected call from
      // looking like a network outage in the console.
      await route.fulfill({
        status: 200,
        contentType: "application/json",
        body: "{}",
      });
    },
  );
}

/** Fills and submits the sign-in form; the caller asserts the landing state. */
export async function submitSignIn(page: Page): Promise<void> {
  await page.goto("/");
  await page.locator('input[type="email"]').fill(E2E_EMAIL);
  await page.locator('input[type="password"]').fill(E2E_PASSWORD);
  await page.getByRole("button", { name: /sign in/i }).click();
}

/** The bearer token every `/v1` request must carry, unchanged. */
export function bearer(): string {
  return `Bearer ${E2E_ACCESS_TOKEN}`;
}

/** Asserts a `/v1` request carried the session token (JWT forwarded unchanged). */
export function expectForwardedToken(authorization: string | undefined): void {
  expect(authorization, "the gateway call must carry the user's token").toBe(bearer());
}
