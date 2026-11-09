import { describe, expect, it } from "vitest";
import { createMemoryStorage } from "./storage";

function fakeSessionStorage() {
  const values = new Map<string, string>();
  return {
    values,
    getItem: (key: string) => values.get(key) ?? null,
    setItem: (key: string, value: string) => {
      values.set(key, value);
    },
    removeItem: (key: string) => {
      values.delete(key);
    },
  };
}

describe("supabase storage adapter", () => {
  it("keeps access and refresh token keys in memory, not in sessionStorage", () => {
    const sessionStorage = fakeSessionStorage();
    const storage = createMemoryStorage("sb-test-auth-token", sessionStorage);

    storage.setItem("sb-test-auth-token", "{\"access_token\":\"a\"}");

    expect(storage.getItem("sb-test-auth-token")).toContain("access_token");
    expect(sessionStorage.values.size).toBe(0);
  });

  it("allows only the PKCE code verifier into sessionStorage", () => {
    const sessionStorage = fakeSessionStorage();
    const storage = createMemoryStorage("sb-test-auth-token", sessionStorage);

    storage.setItem("sb-test-auth-token-code-verifier", "verifier-1");

    expect(sessionStorage.values.get("sb-test-auth-token-code-verifier")).toBe(
      "verifier-1",
    );
    expect(storage.getItem("sb-test-auth-token-code-verifier")).toBe("verifier-1");
  });

  it("clears the code verifier after the callback exchange", () => {
    const sessionStorage = fakeSessionStorage();
    const storage = createMemoryStorage("sb-test-auth-token", sessionStorage);
    storage.setItem("sb-test-auth-token-code-verifier", "verifier-1");

    storage.removeItem("sb-test-auth-token-code-verifier");

    expect(sessionStorage.values.size).toBe(0);
    expect(storage.getItem("sb-test-auth-token-code-verifier")).toBeNull();
  });
});
