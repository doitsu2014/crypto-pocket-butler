/**
 * TanStack Query hooks for Solana token management
 */

import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { apiClient } from "@/lib/api-client";
import type { SolanaToken, CreateSolanaTokenInput, UpdateSolanaTokenInput } from "@/types/api";

// ============================================================================
// Query Keys
// ============================================================================

export const solanaTokenKeys = {
  all: ["solana-tokens"] as const,
  lists: () => [...solanaTokenKeys.all, "list"] as const,
  list: () => [...solanaTokenKeys.lists()] as const,
  details: () => [...solanaTokenKeys.all, "detail"] as const,
  detail: (id: string) => [...solanaTokenKeys.details(), id] as const,
};

// ============================================================================
// Query Hooks
// ============================================================================

/**
 * Fetch all configured Solana SPL tokens
 */
export function useSolanaTokens(isActive?: boolean) {
  const params = isActive !== undefined ? `?is_active=${isActive}` : "";
  return useQuery({
    queryKey: [...solanaTokenKeys.list(), { isActive }],
    queryFn: async () => {
      return apiClient<SolanaToken[]>(`/api/v1/solana-tokens${params}`);
    },
  });
}

// ============================================================================
// Mutation Hooks
// ============================================================================

/**
 * Create a new Solana SPL token entry
 */
export function useCreateSolanaToken() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: async (input: CreateSolanaTokenInput) => {
      return apiClient<SolanaToken>("/api/v1/solana-tokens", {
        method: "POST",
        body: input,
      });
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: solanaTokenKeys.lists() });
    },
  });
}

/**
 * Update an existing Solana SPL token entry
 */
export function useUpdateSolanaToken(tokenUuid: string) {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: async (input: UpdateSolanaTokenInput) => {
      return apiClient<SolanaToken>(`/api/v1/solana-tokens/${tokenUuid}`, {
        method: "PUT",
        body: input,
      });
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: solanaTokenKeys.lists() });
      queryClient.invalidateQueries({ queryKey: solanaTokenKeys.detail(tokenUuid) });
    },
  });
}

/**
 * Delete a Solana SPL token entry
 */
export function useDeleteSolanaToken() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: async (tokenUuid: string) => {
      return apiClient<void>(`/api/v1/solana-tokens/${tokenUuid}`, {
        method: "DELETE",
      });
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: solanaTokenKeys.lists() });
    },
  });
}
