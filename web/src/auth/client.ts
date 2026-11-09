import { createClient, type SupabaseClient } from "@supabase/supabase-js";
import { createMemoryStorage } from "./storage";

const supabaseUrl = (import.meta.env.VITE_SUPABASE_URL as string | undefined) ?? "";
const supabaseAnonKey =
  (import.meta.env.VITE_SUPABASE_ANON_KEY as string | undefined) ?? "";

/**
 * The storage key Supabase derives from a project URL. It is not a secret; it
 * names the in-memory slot and the one sessionStorage key.
 */
export function storageKeyFor(url: string): string {
  if (!url) {
    return "sb-crypto-pocket-butler-auth-token";
  }
  try {
    const host = new URL(url).hostname;
    const ref = host.split(".")[0];
    return `sb-${ref}-auth-token`;
  } catch {
    return "sb-crypto-pocket-butler-auth-token";
  }
}

export const storageKey = storageKeyFor(supabaseUrl);

/**
 * The single Supabase client (NFR9.1). `persistSession` stays true with the
 * custom adapter; `persistSession: false` would replace the adapter and lose
 * the PKCE verifier.
 */
export const supabase: SupabaseClient | null =
  supabaseUrl && supabaseAnonKey
    ? createClient(supabaseUrl, supabaseAnonKey, {
        auth: {
          persistSession: true,
          autoRefreshToken: true,
          detectSessionInUrl: true,
          flowType: "pkce",
          storageKey,
          storage: createMemoryStorage(storageKey, globalThis.sessionStorage),
        },
      })
    : null;
