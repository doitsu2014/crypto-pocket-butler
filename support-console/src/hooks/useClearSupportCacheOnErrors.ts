import { useQueryClient } from "@tanstack/react-query";
import { useEffect } from "react";
import { clearSupportCache, isForbiddenError, isUnauthorizedError } from "../api/queryClient";

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
