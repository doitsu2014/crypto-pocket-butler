/**
 * TanStack Query hooks for portfolio recommendation data fetching
 */

import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { apiClient } from "@/lib/api-client";
import type { 
  ListRecommendationsResponse, 
  RecommendationsQueryParams,
  RecommendationDetailResponse,
  Recommendation
} from "@/types/api";

// ============================================================================
// Query Keys
// ============================================================================

export const recommendationKeys = {
  all: ["recommendations"] as const,
  lists: () => [...recommendationKeys.all, "list"] as const,
  list: (portfolioId: string, params?: RecommendationsQueryParams) => 
    [...recommendationKeys.lists(), portfolioId, params] as const,
  details: () => [...recommendationKeys.all, "detail"] as const,
  detail: (portfolioId: string, recommendationId: string) => 
    [...recommendationKeys.details(), portfolioId, recommendationId] as const,
};

// ============================================================================
// Query Hooks
// ============================================================================

/**
 * Fetch recommendations for a portfolio with optional filters
 */
export function useRecommendations(portfolioId: string, params?: RecommendationsQueryParams) {
  return useQuery({
    queryKey: recommendationKeys.list(portfolioId, params),
    queryFn: async () => {
      // Build query string from params
      const queryParams = new URLSearchParams();
      if (params?.status) {
        queryParams.append("status", params.status);
      }
      if (params?.recommendation_type) {
        queryParams.append("recommendation_type", params.recommendation_type);
      }
      if (params?.limit) {
        queryParams.append("limit", params.limit.toString());
      }

      const queryString = queryParams.toString();
      const endpoint = `/v1/portfolios/${portfolioId}/recommendations${queryString ? `?${queryString}` : ""}`;

      return apiClient<ListRecommendationsResponse>(endpoint);
    },
    enabled: !!portfolioId,
    // Recommendations update less frequently
    staleTime: 60 * 1000, // 1 minute
  });
}

/**
 * Fetch a single recommendation detail
 */
export function useRecommendation(portfolioId: string, recommendationId: string) {
  return useQuery({
    queryKey: recommendationKeys.detail(portfolioId, recommendationId),
    queryFn: async () => {
      return apiClient<RecommendationDetailResponse>(
        `/v1/portfolios/${portfolioId}/recommendations/${recommendationId}`
      );
    },
    enabled: !!portfolioId && !!recommendationId,
  });
}

// ============================================================================
// Mutation Hooks
// ============================================================================

/**
 * Generate new recommendations for a portfolio
 */
export function useGenerateRecommendations() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: async (portfolioId: string) => {
      return apiClient<Recommendation>(`/v1/portfolios/${portfolioId}/recommendations/generate`, {
        method: "POST",
      });
    },
    onSuccess: (_, portfolioId) => {
      // Invalidate recommendations list for this portfolio
      queryClient.invalidateQueries({ 
        queryKey: recommendationKeys.lists(),
        predicate: (query) => {
          const [, , pid] = query.queryKey;
          return pid === portfolioId;
        }
      });
    },
  });
}

/**
 * Approve a recommendation
 */
export function useApproveRecommendation() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: async ({ portfolioId, recommendationId }: { portfolioId: string; recommendationId: string }) => {
      return apiClient<Recommendation>(
        `/v1/portfolios/${portfolioId}/recommendations/${recommendationId}/approve`,
        { method: "POST" }
      );
    },
    onSuccess: (_, { portfolioId, recommendationId }) => {
      // Invalidate both list and detail
      queryClient.invalidateQueries({ queryKey: recommendationKeys.lists() });
      queryClient.invalidateQueries({ 
        queryKey: recommendationKeys.detail(portfolioId, recommendationId) 
      });
    },
  });
}

/**
 * Reject a recommendation
 */
export function useRejectRecommendation() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: async ({ portfolioId, recommendationId }: { portfolioId: string; recommendationId: string }) => {
      return apiClient<Recommendation>(
        `/v1/portfolios/${portfolioId}/recommendations/${recommendationId}/reject`,
        { method: "POST" }
      );
    },
    onSuccess: (_, { portfolioId, recommendationId }) => {
      // Invalidate both list and detail
      queryClient.invalidateQueries({ queryKey: recommendationKeys.lists() });
      queryClient.invalidateQueries({ 
        queryKey: recommendationKeys.detail(portfolioId, recommendationId) 
      });
    },
  });
}

/**
 * Execute an approved recommendation
 */
export function useExecuteRecommendation() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: async ({ portfolioId, recommendationId }: { portfolioId: string; recommendationId: string }) => {
      return apiClient<Recommendation>(
        `/v1/portfolios/${portfolioId}/recommendations/${recommendationId}/execute`,
        { method: "POST" }
      );
    },
    onSuccess: (_, { portfolioId, recommendationId }) => {
      // Invalidate recommendations, portfolio holdings, and accounts
      queryClient.invalidateQueries({ queryKey: recommendationKeys.lists() });
      queryClient.invalidateQueries({ 
        queryKey: recommendationKeys.detail(portfolioId, recommendationId) 
      });
      // Portfolio holdings will change after execution
      queryClient.invalidateQueries({ queryKey: ["portfolios"] });
    },
  });
}
