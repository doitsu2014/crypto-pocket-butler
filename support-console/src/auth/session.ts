/**
 * The support session lives in memory only (BRD-012 NFR-SEC-10). The access
 * token is never written to localStorage, sessionStorage, IndexedDB, or a
 * cookie; closing the tab ends the session.
 */

export type SupportRole = "support" | "admin";

export interface AuthUser {
  id: string;
  email?: string;
  appMetadata?: Record<string, unknown>;
}

export interface SupportSession {
  accessToken: string;
  user: AuthUser;
}

export interface SupabaseUserLike {
  id: string;
  email?: string | null;
  app_metadata?: Record<string, unknown> | null;
}

export function isSupportRole(value: unknown): value is SupportRole {
  return value === "support" || value === "admin";
}

/** BR1.1: only a support or admin role in app_metadata reaches the console. */
export function resolveSupportRole(user: AuthUser | null | undefined): SupportRole | null {
  const role = user?.appMetadata?.role;
  return isSupportRole(role) ? role : null;
}

let currentSession: SupportSession | null = null;
const listeners = new Set<(session: SupportSession | null) => void>();

export function getSupportSession(): SupportSession | null {
  return currentSession;
}

export function setSupportSession(session: SupportSession | null): void {
  currentSession = session;
  for (const listener of listeners) {
    listener(currentSession);
  }
}

export function clearSupportSession(): void {
  setSupportSession(null);
}

export function subscribeSupportSession(
  listener: (session: SupportSession | null) => void,
): () => void {
  listeners.add(listener);
  return () => {
    listeners.delete(listener);
  };
}

export function sessionFromSupabaseUser(
  user: SupabaseUserLike,
  accessToken: string,
): SupportSession {
  return {
    accessToken,
    user: {
      id: user.id,
      email: user.email ?? undefined,
      appMetadata: user.app_metadata ?? {},
    },
  };
}
