import { useEffect, useSyncExternalStore } from "react";
import { supabase } from "./client";

/**
 * The browser session (BR3.1, NFR9.1).
 *
 * Supabase Auth is the single source of truth. The access token is exposed to
 * the API client in memory only; the existing setter is kept for tests and for
 * the standalone asset-resolver surface.
 */

export interface AuthSession {
  access_token: string;
  refresh_token?: string;
  user?: { id: string; email?: string };
}

export interface AuthSubscription {
  data: { subscription: { unsubscribe(): void } };
}

export interface AuthClientLike {
  getSession(): Promise<{ data: { session: AuthSession | null } }>;
  onAuthStateChange(
    callback: (event: string, session: AuthSession | null) => void,
  ): AuthSubscription;
  signInWithPassword(input: {
    email: string;
    password: string;
  }): Promise<{ error: { message: string } | null }>;
  signUp(input: {
    email: string;
    password: string;
    options?: { data?: Record<string, string> };
  }): Promise<{ error: { message: string } | null }>;
  signOut(): Promise<unknown>;
}

export type SessionStatus = "loading" | "signed-out" | "signed-in" | "unavailable";

export interface SessionState {
  status: SessionStatus;
  session: AuthSession | null;
}

export interface SessionStore {
  getSnapshot(): SessionState;
  subscribe(listener: () => void): () => void;
  start(): void;
  signIn(email: string, password: string): Promise<void>;
  signUp(email: string, password: string, gateToken?: string): Promise<void>;
  signOut(): Promise<void>;
  /** Test hook: forget the subscription and return to the initial state. */
  reset(): void;
}

let overrideToken: string | null = null;

/** Compatibility setter for tests and the standalone asset resolver. */
export function setAccessToken(token: string | null): void {
  overrideToken = token;
}

export function createSessionStore(client: AuthClientLike | null): SessionStore {
  let state: SessionState = {
    status: client ? "loading" : "unavailable",
    session: null,
  };
  let started = false;
  let unsubscribe: (() => void) | null = null;
  const listeners = new Set<() => void>();

  function emit(next: SessionState): void {
    state = next;
    for (const listener of listeners) {
      listener();
    }
  }

  function apply(session: AuthSession | null): void {
    emit(
      session
        ? { status: "signed-in", session }
        : { status: "signed-out", session: null },
    );
  }

  function start(): void {
    if (started) {
      return;
    }
    started = true;
    if (!client) {
      return;
    }
    client
      .getSession()
      .then(({ data }) => apply(data.session))
      .catch(() => apply(null));
    const subscription = client.onAuthStateChange((_event, session) => {
      apply(session);
    });
    unsubscribe = () => subscription.data.subscription.unsubscribe();
  }

  async function signIn(email: string, password: string): Promise<void> {
    if (!client) {
      throw new Error("Supabase Auth is not configured");
    }
    const { error } = await client.signInWithPassword({ email, password });
    if (error) {
      throw new Error(error.message);
    }
  }

  async function signUp(
    email: string,
    password: string,
    gateToken?: string,
  ): Promise<void> {
    if (!client) {
      throw new Error("Supabase Auth is not configured");
    }
    const { error } = await client.signUp({
      email,
      password,
      options: gateToken ? { data: { gate_token: gateToken } } : undefined,
    });
    if (error) {
      throw new Error(error.message);
    }
  }

  async function signOut(): Promise<void> {
    if (client) {
      await client.signOut();
    }
    overrideToken = null;
    apply(null);
  }

  function reset(): void {
    started = false;
    unsubscribe?.();
    unsubscribe = null;
    state = { status: client ? "loading" : "unavailable", session: null };
    listeners.clear();
  }

  return {
    getSnapshot: () => state,
    subscribe(listener) {
      listeners.add(listener);
      return () => {
        listeners.delete(listener);
      };
    },
    start,
    signIn,
    signUp,
    signOut,
    reset,
  };
}

export const sessionStore = createSessionStore(
  supabase as unknown as AuthClientLike | null,
);

/** The API client reads this per request; it never touches web storage. */
export function getAccessToken(): string | null {
  return overrideToken ?? sessionStore.getSnapshot().session?.access_token ?? null;
}

export function useSession(): SessionState {
  useEffect(() => {
    sessionStore.start();
  }, []);
  return useSyncExternalStore(
    sessionStore.subscribe,
    sessionStore.getSnapshot,
    sessionStore.getSnapshot,
  );
}

export async function signOut(): Promise<void> {
  await sessionStore.signOut();
}
