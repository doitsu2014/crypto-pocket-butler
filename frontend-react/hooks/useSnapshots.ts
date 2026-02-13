/**
 * TanStack Query hooks for portfolio snapshot data fetching
 */

import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { apiClient } from "@/lib/api-client";
import type { 
  ListSnapshotsResponse, 
  SnapshotsQueryParams,
  CreateSnapshotInput,
  SnapshotResult 
} from "@/types/api";

// ============================================================================
// Query Keys
// ============================================================================

export const snapshotKeys = {
  all: ["snapshots"] as const,
  lists: () => [...snapshotKeys.all, "list"] as const,
  list: (portfolioId: string, params?: SnapshotsQueryParams) => 
    [...snapshotKeys.lists(), portfolioId, params] as const,
};

// ============================================================================
// Query Hooks
// ============================================================================

/**
 * Fetch snapshots for a portfolio with optional filters
 */
export function useSnapshots(portfolioId: string, params?: SnapshotsQueryParams) {
  return useQuery({
    queryKey: snapshotKeys.list(portfolioId, params),
    queryFn: async () => {
      // Build query string from params
      const queryParams = new URLSearchParams();
      if (params?.snapshot_type) {
        queryParams.append("snapshot_type", params.snapshot_type);
      }
      if (params?.start_date) {
        queryParams.append("start_date", params.start_date);
      }
      if (params?.end_date) {
        queryParams.append("end_date", params.end_date);
      }
      if (params?.limit) {
        queryParams.append("limit", params.limit.toString());
      }

      const queryString = queryParams.toString();
      const endpoint = `/v1/portfolios/${portfolioId}/snapshots${queryString ? `?${queryString}` : ""}`;

      return apiClient<ListSnapshotsResponse>(endpoint);
    },
    enabled: !!portfolioId,
    // Snapshots are historical data, they don't change frequently
    staleTime: 5 * 60 * 1000, // 5 minutes
    refetchOnWindowFocus: false,
  });
}

// ============================================================================
// Mutation Hooks
// ============================================================================

/**
 * Create a new snapshot for a portfolio
 */
export function useCreateSnapshot(portfolioId: string) {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: async (input: CreateSnapshotInput = {}) => {
      return apiClient<SnapshotResult>(`/v1/portfolios/${portfolioId}/snapshots`, {
        method: "POST",
        body: JSON.stringify({
          snapshot_type: input.snapshot_type || "manual",
          snapshot_date: input.snapshot_date,
        }),
      });
    },
    onSuccess: () => {
      // Invalidate all snapshot queries for this portfolio
      queryClient.invalidateQueries({
        queryKey: snapshotKeys.list(portfolioId),
      });
    },
  });
}
