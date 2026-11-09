import { describe, expect, it, vi } from "vitest";
import { createSessionStore, type AuthClientLike } from "./session";

function fakeAuth(initial: { access_token: string } | null) {
  let listener: ((event: string, session: unknown) => void) | null = null;
  const client = {
    getSession: vi.fn().mockResolvedValue({ data: { session: initial } }),
    onAuthStateChange: vi.fn((callback: (event: string, session: unknown) => void) => {
      listener = callback;
      return { data: { subscription: { unsubscribe: vi.fn() } } };
    }),
    signInWithPassword: vi.fn().mockResolvedValue({ error: null }),
    signUp: vi.fn().mockResolvedValue({ error: null }),
    signOut: vi.fn().mockResolvedValue(undefined),
  };
  return {
    client: client as unknown as AuthClientLike,
    emit: (session: unknown) => listener?.("SIGNED_IN", session),
  };
}

describe("session store", () => {
  it("starts loading and becomes signed-in from the stored session", async () => {
    const auth = fakeAuth({ access_token: "token-1" });
    const store = createSessionStore(auth.client);

    expect(store.getSnapshot().status).toBe("loading");
    store.start();
    await vi.waitFor(() => expect(store.getSnapshot().status).toBe("signed-in"));
    expect(store.getSnapshot().session?.access_token).toBe("token-1");
  });

  it("clears the session when Supabase signs out", async () => {
    const auth = fakeAuth({ access_token: "token-1" });
    const store = createSessionStore(auth.client);
    store.start();
    await vi.waitFor(() => expect(store.getSnapshot().status).toBe("signed-in"));

    auth.emit(null);

    expect(store.getSnapshot().status).toBe("signed-out");
    expect(store.getSnapshot().session).toBeNull();
  });

  it("calls Supabase sign-out and clears the token", async () => {
    const auth = fakeAuth({ access_token: "token-1" });
    const store = createSessionStore(auth.client);
    store.start();
    await vi.waitFor(() => expect(store.getSnapshot().status).toBe("signed-in"));

    await store.signOut();

    expect(auth.client.signOut).toHaveBeenCalledOnce();
    expect(store.getSnapshot().status).toBe("signed-out");
  });

  it("reports unavailable when Supabase Auth is not configured", () => {
    const store = createSessionStore(null);
    expect(store.getSnapshot().status).toBe("unavailable");
    store.start();
    expect(store.getSnapshot().status).toBe("unavailable");
  });

  it("passes the signup gate token to Supabase metadata", async () => {
    const auth = fakeAuth(null);
    const store = createSessionStore(auth.client);

    await store.signUp("mai@example.com", "correct-horse", "gate-1");

    expect(auth.client.signUp).toHaveBeenCalledWith({
      email: "mai@example.com",
      password: "correct-horse",
      options: { data: { gate_token: "gate-1" } },
    });
  });
});
