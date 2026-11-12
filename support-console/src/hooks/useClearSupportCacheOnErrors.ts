import { useQueryClient } from "@tanstack/react-query";
import { useEffect } from "react";
import {
  clearSupportCache,
  clearSupportQueries,
  isForbiddenError,
  isUnauthorizedError,
} from "../api/queryClient";

/**
 * Any 401 or 403 clears the query cache (security design: cache clearing on
 * sign-out, grant expiry or revocation, and any 403).
 */
export function useClearSupportCacheOnErrors(error: unknown): void {
  const queryClient = useQueryClient();

  useEffect(() => {
    if (isForbiddenError(error) || isUnauthorizedError(error)) {
      clearSupportCache(queryClient);
    }
  }, [error, queryClient]);
}

/**
 * The support-route variant: a 401 means the session is gone, so everything
 * goes; a 403 means this grant ended, so only the support entries go.
 */
export function useClearSupportQueriesOnRouteErrors(error: unknown): void {
  const queryClient = useQueryClient();

  useEffect(() => {
    if (isUnauthorizedError(error)) {
      clearSupportCache(queryClient);
    } else if (isForbiddenError(error)) {
      clearSupportQueries(queryClient);
    }
  }, [error, queryClient]);
}
