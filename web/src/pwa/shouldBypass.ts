/**
 * Service worker URL policy (NFR9.5, BR3.3).
 *
 * The worker is registered at the site root. It bypasses the support console,
 * every gateway call, and every cross-origin request (Supabase Auth and
 * WalletConnect are the only contracted direct calls); it caches same-origin
 * static assets only.
 */

export const PWA_SCOPE = "/";

export function shouldBypass(requestUrl: URL, appOrigin: string): boolean {
  if (requestUrl.origin !== appOrigin) {
    return true;
  }
  const path = requestUrl.pathname;
  if (path === "/support" || path.startsWith("/support/")) {
    return true;
  }
  if (path === "/v1" || path.startsWith("/v1/")) {
    return true;
  }
  return false;
}
