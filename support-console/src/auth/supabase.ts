import { createClient, type SupabaseClient } from "@supabase/supabase-js";
import {
  sessionFromSupabaseUser,
  type SupportSession,
  type SupabaseUserLike,
} from "./session";

export interface SupportSupabaseConfig {
  url: string;
  anonKey: string;
}

export interface SupportedStorageLike {
  getItem(key: string): string | null;
  setItem(key: string, value: string): void;
  removeItem(key: string): void;
  clear(): void;
}

/**
 * Supabase Auth normally persists its session to localStorage. The console
 * supplies an in-memory adapter instead, so a token never outlives the tab.
 */
export function createInMemoryStorage(): SupportedStorageLike {
  const values = new Map<string, string>();
  return {
    getItem(key) {
      return values.get(key) ?? null;
    },
    setItem(key, value) {
      values.set(key, value);
    },
    removeItem(key) {
      values.delete(key);
    },
    clear() {
      values.clear();
    },
  };
}

export function supportAuthOptions(storage: SupportedStorageLike) {
  return {
    persistSession: false,
    autoRefreshToken: true,
    detectSessionInUrl: false,
    storage,
  } as const;
}

export function createSupportSupabaseClient(config: SupportSupabaseConfig): SupabaseClient {
  return createClient(config.url, config.anonKey, {
    auth: supportAuthOptions(createInMemoryStorage()),
  });
}

export interface SupabaseAuthLike {
  auth: {
    signInWithPassword(credentials: {
      email: string;
      password: string;
    }): Promise<{
      data: {
        session: { access_token: string } | null;
        user: SupabaseUserLike | null;
      };
      error: { message: string } | null;
    }>;
    signOut(): Promise<{ error: { message: string } | null }>;
  };
}

export interface SupportAuthClient {
  signIn(email: string, password: string): Promise<SupportSession>;
  signOut(): Promise<void>;
}

export function createSupabaseAuthClient(client: SupabaseAuthLike): SupportAuthClient {
  return {
    async signIn(email: string, password: string): Promise<SupportSession> {
      const { data, error } = await client.auth.signInWithPassword({ email, password });
      if (error) {
        throw new Error(error.message);
      }
      if (!data.session || !data.user) {
        throw new Error("Supabase Auth returned no session.");
      }
      return sessionFromSupabaseUser(data.user, data.session.access_token);
    },

    async signOut(): Promise<void> {
      const { error } = await client.auth.signOut();
      if (error) {
        throw new Error(error.message);
      }
    },
  };
}

export function configFromEnv(
  env: { VITE_SUPABASE_URL?: string; VITE_SUPABASE_ANON_KEY?: string } = import.meta.env,
): SupportSupabaseConfig | null {
  const url = env.VITE_SUPABASE_URL?.trim();
  const anonKey = env.VITE_SUPABASE_ANON_KEY?.trim();
  if (!url || !anonKey) {
    return null;
  }
  return { url, anonKey };
}
