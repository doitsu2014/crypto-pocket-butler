/**
 * TanStack Query hooks for chains data fetching
 */

import { useQuery } from "@tanstack/react-query";
import { apiClient } from "@/lib/api-client";
import type { Chain } from "@/types/api";

// ============================================================================
// Query Keys
// ============================================================================

export const chainKeys = {
  all: ["chains"] as const,
  list: () => [...chainKeys.all, "list"] as const,
};

// ============================================================================
// Query Hooks
// ============================================================================

/**
 * Fetch list of all supported EVM chains
 */
export function useChains() {
  return useQuery({
    queryKey: chainKeys.list(),
    queryFn: async () => {
      const response = await apiClient<{ chains: Chain[] }>(
        "/v1/chains"
      );
      return response.chains;
    },
    staleTime: 1000 * 60 * 60, // Cache for 1 hour - chains rarely change
  });
}
