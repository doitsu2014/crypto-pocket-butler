import { describe, expect, it } from "vitest";
import { PWA_SCOPE, shouldBypass } from "./shouldBypass";

const origin = "https://app.cryptopocketbutler.example";

function url(path: string, base = origin) {
  return new URL(path, base);
}

describe("service worker bypass", () => {
  it("bypasses the support console at /support", () => {
    expect(shouldBypass(url("/support/"), origin)).toBe(true);
    expect(shouldBypass(url("/support/assets/index.js"), origin)).toBe(true);
  });

  it("bypasses every gateway call at /v1", () => {
    expect(shouldBypass(url("/v1/portfolios"), origin)).toBe(true);
    expect(shouldBypass(url("/v1"), origin)).toBe(true);
  });

  it("bypasses cross-origin Supabase and WalletConnect requests", () => {
    expect(
      shouldBypass(url("https://project.supabase.co/auth/v1/token"), origin),
    ).toBe(true);
    expect(
      shouldBypass(url("https://relay.walletconnect.com/"), origin),
    ).toBe(true);
    expect(
      shouldBypass(url("https://verify.walletconnect.org/"), origin),
    ).toBe(true);
  });

  it("does not bypass same-origin static assets or app navigation", () => {
    expect(shouldBypass(url("/assets/app-123.js"), origin)).toBe(false);
    expect(shouldBypass(url("/dashboard"), origin)).toBe(false);
  });

  it("registers at root scope because the SPA is served at the site root", () => {
    expect(PWA_SCOPE).toBe("/");
  });
});
