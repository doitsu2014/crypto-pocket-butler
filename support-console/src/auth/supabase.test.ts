import { describe, expect, it, vi } from "vitest";
import {
  configFromEnv,
  createInMemoryStorage,
  createSupabaseAuthClient,
  createSupportSupabaseClient,
  supportAuthOptions,
  type SupabaseAuthLike,
} from "./supabase";

describe("in-memory Supabase storage", () => {
  it("stores values in memory and removes them", () => {
    const storage = createInMemoryStorage();

    expect(storage.getItem("session")).toBeNull();
    storage.setItem("session", "value");
    expect(storage.getItem("session")).toBe("value");
    storage.removeItem("session");
    expect(storage.getItem("session")).toBeNull();
  });

  it("clears every value and never touches browser storage", () => {
    const storage = createInMemoryStorage();
    storage.setItem("a", "1");
    storage.setItem("b", "2");
    storage.clear();

    expect(storage.getItem("a")).toBeNull();
    expect(storage.getItem("b")).toBeNull();
    expect(window.localStorage.length).toBe(0);
    expect(window.sessionStorage.length).toBe(0);
  });

  it("never writes to localStorage even when one exists", () => {
    const storage = createInMemoryStorage();
    storage.setItem("token", "secret-token");

    expect(window.localStorage.getItem("token")).toBeNull();
    expect(window.sessionStorage.getItem("token")).toBeNull();
  });
});

describe("Supabase auth options", () => {
  it("disables persistence and URL session detection", () => {
    const storage = createInMemoryStorage();
    const options = supportAuthOptions(storage);

    expect(options.persistSession).toBe(false);
    expect(options.autoRefreshToken).toBe(true);
    expect(options.detectSessionInUrl).toBe(false);
    expect(options.storage).toBe(storage);
  });

  it("creates a client with an in-memory storage adapter", () => {
    const client = createSupportSupabaseClient({
      url: "https://project.example.supabase.co",
      anonKey: "public-anon-key",
    });

    expect(client).not.toBeNull();
    expect(typeof client?.auth.getSession).toBe("function");
  });
});

describe("auth client adapter", () => {
  function fakeClient(overrides: Partial<SupabaseAuthLike["auth"]> = {}): SupabaseAuthLike {
    return {
      auth: {
        signInWithPassword: vi.fn().mockResolvedValue({
          data: {
            session: { access_token: "token-1" },
            user: {
              id: "agent-1",
              email: "agent@example.test",
              app_metadata: { role: "support" },
            },
          },
          error: null,
        }),
        signOut: vi.fn().mockResolvedValue({ error: null }),
        ...overrides,
      },
    };
  }

  it("maps a successful sign-in to an in-memory support session", async () => {
    const client = fakeClient();
    const auth = createSupabaseAuthClient(client);

    const session = await auth.signIn("agent@example.test", "password");

    expect(client.auth.signInWithPassword).toHaveBeenCalledWith({
      email: "agent@example.test",
      password: "password",
    });
    expect(session.accessToken).toBe("token-1");
    expect(session.user.appMetadata?.role).toBe("support");
    expect(window.localStorage.length).toBe(0);
  });

  it("throws the Supabase error message on a failed sign-in", async () => {
    const client = fakeClient({
      signInWithPassword: vi.fn().mockResolvedValue({
        data: { session: null, user: null },
        error: { message: "Invalid login credentials" },
      }),
    });
    const auth = createSupabaseAuthClient(client);

    await expect(auth.signIn("agent@example.test", "wrong")).rejects.toThrow(
      /invalid login credentials/i,
    );
  });

  it("refuses a session without a user or access token", async () => {
    const client = fakeClient({
      signInWithPassword: vi.fn().mockResolvedValue({
        data: { session: null, user: null },
        error: null,
      }),
    });
    const auth = createSupabaseAuthClient(client);

    await expect(auth.signIn("agent@example.test", "password")).rejects.toThrow(/session/i);
  });

  it("signs out through the Supabase client", async () => {
    const client = fakeClient();
    const auth = createSupabaseAuthClient(client);

    await auth.signOut();

    expect(client.auth.signOut).toHaveBeenCalled();
  });
});

describe("environment configuration", () => {
  it("returns null unless both Supabase values are present", () => {
    expect(configFromEnv({})).toBeNull();
    expect(configFromEnv({ VITE_SUPABASE_URL: "https://project.example.supabase.co" })).toBeNull();
    expect(
      configFromEnv({ VITE_SUPABASE_ANON_KEY: "public-anon-key" }),
    ).toBeNull();
  });

  it("reads the public URL and anon key", () => {
    expect(
      configFromEnv({
        VITE_SUPABASE_URL: "https://project.example.supabase.co",
        VITE_SUPABASE_ANON_KEY: "public-anon-key",
      }),
    ).toEqual({
      url: "https://project.example.supabase.co",
      anonKey: "public-anon-key",
    });
  });
});
