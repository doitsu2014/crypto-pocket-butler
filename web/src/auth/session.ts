/**
 * The browser session token lives in memory only (BRD-012 NFR-SEC-10).
 * It is never written to localStorage, sessionStorage, IndexedDB, or a cookie.
 * Supabase Auth sign-in (unit u3-identity) will set this token.
 */
let accessToken: string | null = null;

export function setAccessToken(token: string | null): void {
  accessToken = token;
}

export function getAccessToken(): string | null {
  return accessToken;
}
