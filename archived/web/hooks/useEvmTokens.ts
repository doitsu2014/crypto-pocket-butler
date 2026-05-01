/**
 * TanStack Query hooks for EVM token management
 */

import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { apiClient } from "@/lib/api-client";
import type { EvmToken, CreateEvmTokenInput, UpdateEvmTokenInput } from "@/types/api";

// ============================================================================
// Query Keys
// ============================================================================

export const evmTokenKeys = {
  all: ["evm-tokens"] as const,
  lists: () => [...evmTokenKeys.all, "list"] as const,
  list: (chain?: string) => [...evmTokenKeys.lists(), { chain }] as const,
  details: () => [...evmTokenKeys.all, "detail"] as const,
  detail: (id: string) => [...evmTokenKeys.details(), id] as const,
};

// ============================================================================
// Query Hooks
// ============================================================================

/**
 * Fetch all EVM tokens, optionally filtered by chain
 */
export function useEvmTokens(chain?: string) {
  const params = chain ? `?chain=${encodeURIComponent(chain)}` : "";
  return useQuery({
    queryKey: evmTokenKeys.list(chain),
    queryFn: async () => {
      return apiClient<EvmToken[]>(`/v1/evm-tokens${params}`);
    },
  });
}

// ============================================================================
// Mutation Hooks
// ============================================================================

/**
 * Create a new EVM token
 */
export function useCreateEvmToken() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: async (input: CreateEvmTokenInput) => {
      return apiClient<EvmToken>("/v1/evm-tokens", {
        method: "POST",
        body: input,
      });
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: evmTokenKeys.lists() });
    },
  });
}

/**
 * Update an existing EVM token
 */
export function useUpdateEvmToken(tokenId: string) {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: async (input: UpdateEvmTokenInput) => {
      return apiClient<EvmToken>(`/v1/evm-tokens/${tokenId}`, {
        method: "PUT",
        body: input,
      });
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: evmTokenKeys.lists() });
      queryClient.invalidateQueries({ queryKey: evmTokenKeys.detail(tokenId) });
    },
  });
}

/**
 * Delete an EVM token
 */
export function useDeleteEvmToken() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: async (tokenId: string) => {
      return apiClient<void>(`/v1/evm-tokens/${tokenId}`, {
        method: "DELETE",
      });
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: evmTokenKeys.lists() });
    },
  });
}
