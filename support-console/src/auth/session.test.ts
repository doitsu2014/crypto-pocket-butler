import { afterEach, describe, expect, it, vi } from "vitest";
import {
  clearSupportSession,
  getSupportSession,
  isSupportRole,
  resolveSupportRole,
  sessionFromSupabaseUser,
  setSupportSession,
  subscribeSupportSession,
  type SupportSession,
} from "./session";

const session: SupportSession = {
  accessToken: "token-1",
  user: {
    id: "agent-1",
    email: "agent@example.test",
    appMetadata: { role: "support" },
  },
};

describe("role resolution", () => {
  it("accepts support and admin roles", () => {
    expect(resolveSupportRole({ id: "a", appMetadata: { role: "support" } })).toBe("support");
    expect(resolveSupportRole({ id: "b", appMetadata: { role: "admin" } })).toBe("admin");
    expect(isSupportRole("support")).toBe(true);
    expect(isSupportRole("admin")).toBe(true);
  });

  it("refuses a customer, a missing role, and a non-string role", () => {
    expect(resolveSupportRole({ id: "c", appMetadata: { role: "customer" } })).toBeNull();
    expect(resolveSupportRole({ id: "d", appMetadata: {} })).toBeNull();
    expect(resolveSupportRole({ id: "e" })).toBeNull();
    expect(resolveSupportRole({ id: "f", appMetadata: { role: 7 } })).toBeNull();
    expect(resolveSupportRole(null)).toBeNull();
    expect(isSupportRole("customer")).toBe(false);
  });
});

describe("in-memory session", () => {
  afterEach(() => {
    clearSupportSession();
  });

  it("stores the token and user in memory only", () => {
    setSupportSession(session);

    expect(getSupportSession()).toEqual(session);
    expect(window.localStorage.length).toBe(0);
    expect(window.sessionStorage.length).toBe(0);
  });

  it("clears the session", () => {
    setSupportSession(session);
    clearSupportSession();
    expect(getSupportSession()).toBeNull();
  });

  it("notifies subscribers on set and clear, and stops after unsubscribe", () => {
    const listener = vi.fn();
    const unsubscribe = subscribeSupportSession(listener);

    setSupportSession(session);
    clearSupportSession();
    expect(listener).toHaveBeenCalledTimes(2);

    unsubscribe();
    setSupportSession(session);
    expect(listener).toHaveBeenCalledTimes(2);
  });
});

describe("Supabase user mapping", () => {
  it("maps the app_metadata role and access token", () => {
    const mapped = sessionFromSupabaseUser(
      {
        id: "agent-9",
        email: "agent9@example.test",
        app_metadata: { role: "admin", provider: "email" },
      },
      "token-9",
    );

    expect(mapped).toEqual({
      accessToken: "token-9",
      user: {
        id: "agent-9",
        email: "agent9@example.test",
        appMetadata: { role: "admin", provider: "email" },
      },
    });
    expect(resolveSupportRole(mapped.user)).toBe("admin");
  });

  it("keeps an absent email and metadata object safe", () => {
    const mapped = sessionFromSupabaseUser({ id: "agent-10" }, "token-10");
    expect(mapped.user.email).toBeUndefined();
    expect(mapped.user.appMetadata).toEqual({});
    expect(resolveSupportRole(mapped.user)).toBeNull();
  });
});
