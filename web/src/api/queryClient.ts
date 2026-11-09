import { MutationCache, QueryCache, QueryClient } from "@tanstack/react-query";
import { ApiError } from "./client";

/**
 * The app-wide 401 rule (NFR9.1, security design): a 401 signs out, clears the
 * query cache, and returns to `/`. A 5xx retries once; a 4xx never retries.
 */

export interface QueryClientPolicy {
  onUnauthorized?: () => void;
}

export function shouldRetry(failureCount: number, error: unknown): boolean {
  if (failureCount >= 1) {
    return false;
  }
  return error instanceof ApiError && error.problem.status >= 500;
}

function isUnauthorized(error: unknown): boolean {
  return error instanceof ApiError && error.problem.status === 401;
}

export function createQueryClient(policy: QueryClientPolicy = {}): QueryClient {
  return new QueryClient({
    queryCache: new QueryCache({
      onError: (error) => {
        if (isUnauthorized(error)) {
          policy.onUnauthorized?.();
        }
      },
    }),
    mutationCache: new MutationCache({
      onError: (error) => {
        if (isUnauthorized(error)) {
          policy.onUnauthorized?.();
        }
      },
    }),
    defaultOptions: {
      queries: {
        retry: shouldRetry,
        staleTime: 30_000,
        refetchOnWindowFocus: false,
      },
      mutations: { retry: false },
    },
  });
}
