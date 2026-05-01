/**
 * TanStack Query client configuration with centralized error handling and retry logic
 */

import { QueryClient } from "@tanstack/react-query";
import { ApiError } from "./api-client";

/**
 * Default query options for all queries
 */
const defaultQueryOptions = {
  queries: {
    // Stale time: 30 seconds - data is considered fresh for this duration
    staleTime: 30 * 1000,
    
    // Retry logic based on error type
    retry: (failureCount: number, error: unknown) => {
      // Don't retry on auth errors
      if (error instanceof ApiError && error.type === "auth") {
        return false;
      }
      
      // Don't retry on validation errors
      if (error instanceof ApiError && error.type === "validation") {
        return false;
      }
      
      // Retry network and server errors up to 2 times
      if (error instanceof ApiError && 
          (error.type === "network" || error.type === "server")) {
        return failureCount < 2;
      }
      
      // Default: retry once for unknown errors
      return failureCount < 1;
    },
    
    // Retry delay: exponential backoff
    retryDelay: (attemptIndex: number) => Math.min(1000 * 2 ** attemptIndex, 30000),
    
    // Refetch on window focus for data freshness
    refetchOnWindowFocus: true,
    
    // Don't refetch on reconnect by default (can be overridden per query)
    refetchOnReconnect: false,
  },
  mutations: {
    // Retry mutations once on network errors only
    retry: (failureCount: number, error: unknown) => {
      if (error instanceof ApiError && error.type === "network") {
        return failureCount < 1;
      }
      return false;
    },
  },
};

/**
 * Create a new QueryClient instance with default configuration
 */
export function createQueryClient() {
  return new QueryClient({
    defaultOptions: defaultQueryOptions,
  });
}

/**
 * Singleton query client for client-side usage
 */
let browserQueryClient: QueryClient | undefined = undefined;

export function getQueryClient() {
  if (typeof window === "undefined") {
    // Server: always create a new query client
    return createQueryClient();
  } else {
    // Browser: use singleton pattern
    if (!browserQueryClient) {
      browserQueryClient = createQueryClient();
    }
    return browserQueryClient;
  }
}
