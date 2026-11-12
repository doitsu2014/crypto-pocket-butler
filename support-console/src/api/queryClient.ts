import { QueryClient } from "@tanstack/react-query";
import { ApiError } from "./client";

/**
 * Server state for the support console. Search results live for 30 s and
 * metadata reads for 10 s (performance design); the cache is cleared on
 * sign-out, grant expiry or revocation, and any 401/403.
 */
export function createSupportQueryClient(): QueryClient {
  return new QueryClient({
    defaultOptions: {
      queries: {
        retry: false,
        staleTime: 10_000,
        gcTime: 300_000,
      },
      mutations: {
        retry: false,
      },
    },
  });
}

export function clearSupportCache(client: QueryClient): void {
  void client.cancelQueries();
  client.clear();
}

/**
 * Drops only the `["support", …]` entries. A support-route 403 ends support
 * access; it must not throw away unrelated admin state the same session holds.
 */
export function clearSupportQueries(client: QueryClient): void {
  void client.cancelQueries({ queryKey: ["support"] });
  client.removeQueries({ queryKey: ["support"] });
}

export function isForbiddenError(error: unknown): boolean {
  return error instanceof ApiError && error.status === 403;
}

export function isUnauthorizedError(error: unknown): boolean {
  return error instanceof ApiError && error.status === 401;
}
