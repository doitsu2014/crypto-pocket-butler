/**
 * TanStack Query hooks for EVM chain management
 */

import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { apiClient } from "@/lib/api-client";
import type { EvmChain, CreateEvmChainInput, UpdateEvmChainInput } from "@/types/api";

// ============================================================================
// Query Keys
// ============================================================================

export const evmChainKeys = {
  all: ["evm-chains"] as const,
  lists: () => [...evmChainKeys.all, "list"] as const,
  list: () => [...evmChainKeys.lists()] as const,
  details: () => [...evmChainKeys.all, "detail"] as const,
  detail: (id: string) => [...evmChainKeys.details(), id] as const,
};

// ============================================================================
// Query Hooks
// ============================================================================

/**
 * Fetch all configured EVM chains
 */
export function useEvmChains() {
  return useQuery({
    queryKey: evmChainKeys.list(),
    queryFn: async () => {
      return apiClient<EvmChain[]>("/v1/evm-chains");
    },
  });
}

// ============================================================================
// Mutation Hooks
// ============================================================================

/**
 * Create a new EVM chain configuration
 */
export function useCreateEvmChain() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: async (input: CreateEvmChainInput) => {
      return apiClient<EvmChain>("/v1/evm-chains", {
        method: "POST",
        body: input,
      });
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: evmChainKeys.lists() });
    },
  });
}

/**
 * Update an existing EVM chain configuration
 */
export function useUpdateEvmChain(chainUuid: string) {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: async (input: UpdateEvmChainInput) => {
      return apiClient<EvmChain>(`/v1/evm-chains/${chainUuid}`, {
        method: "PUT",
        body: input,
      });
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: evmChainKeys.lists() });
      queryClient.invalidateQueries({ queryKey: evmChainKeys.detail(chainUuid) });
    },
  });
}

/**
 * Delete an EVM chain configuration
 */
export function useDeleteEvmChain() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: async (chainUuid: string) => {
      return apiClient<void>(`/v1/evm-chains/${chainUuid}`, {
        method: "DELETE",
      });
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: evmChainKeys.lists() });
    },
  });
}
