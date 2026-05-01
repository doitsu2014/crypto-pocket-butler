/**
 * TanStack Query hooks for portfolio data fetching
 */

import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { apiClient } from "@/lib/api-client";
import type { Portfolio, PortfolioHoldingsResponse, CreatePortfolioInput } from "@/types/api";

// ============================================================================
// Query Keys
// ============================================================================

export const portfolioKeys = {
  all: ["portfolios"] as const,
  lists: () => [...portfolioKeys.all, "list"] as const,
  list: () => [...portfolioKeys.lists()] as const,
  details: () => [...portfolioKeys.all, "detail"] as const,
  detail: (id: string) => [...portfolioKeys.details(), id] as const,
  holdings: (id: string) => [...portfolioKeys.detail(id), "holdings"] as const,
};

// ============================================================================
// Query Hooks
// ============================================================================

/**
 * Fetch list of all portfolios for the current user
 */
export function usePortfolios() {
  return useQuery({
    queryKey: portfolioKeys.list(),
    queryFn: async () => {
      return apiClient<Portfolio[]>("/v1/portfolios");
    },
  });
}

/**
 * Fetch a single portfolio by ID
 */
export function usePortfolio(portfolioId: string) {
  return useQuery({
    queryKey: portfolioKeys.detail(portfolioId),
    queryFn: async () => {
      return apiClient<Portfolio>(`/v1/portfolios/${portfolioId}`);
    },
    enabled: !!portfolioId,
  });
}

/**
 * Fetch portfolio holdings with current values
 */
export function usePortfolioHoldings(portfolioId: string) {
  return useQuery({
    queryKey: portfolioKeys.holdings(portfolioId),
    queryFn: async () => {
      return apiClient<PortfolioHoldingsResponse>(`/v1/portfolios/${portfolioId}/holdings`);
    },
    enabled: !!portfolioId,
    // Holdings data changes frequently, refetch on window focus
    refetchOnWindowFocus: true,
  });
}

// ============================================================================
// Mutation Hooks
// ============================================================================

/**
 * Create a new portfolio
 */
export function useCreatePortfolio() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: async (input: CreatePortfolioInput) => {
      return apiClient<Portfolio>("/v1/portfolios", {
        method: "POST",
        body: input,
      });
    },
    onSuccess: () => {
      // Invalidate and refetch portfolios list
      queryClient.invalidateQueries({ queryKey: portfolioKeys.lists() });
    },
  });
}

/**
 * Update an existing portfolio
 */
export function useUpdatePortfolio(portfolioId: string) {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: async (input: Partial<CreatePortfolioInput>) => {
      return apiClient<Portfolio>(`/v1/portfolios/${portfolioId}`, {
        method: "PUT",
        body: input,
      });
    },
    onSuccess: () => {
      // Invalidate both list and specific portfolio
      queryClient.invalidateQueries({ queryKey: portfolioKeys.lists() });
      queryClient.invalidateQueries({ queryKey: portfolioKeys.detail(portfolioId) });
    },
  });
}

/**
 * Delete a portfolio
 */
export function useDeletePortfolio() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: async (portfolioId: string) => {
      return apiClient<void>(`/v1/portfolios/${portfolioId}`, {
        method: "DELETE",
      });
    },
    onSuccess: () => {
      // Invalidate portfolios list
      queryClient.invalidateQueries({ queryKey: portfolioKeys.lists() });
    },
  });
}
