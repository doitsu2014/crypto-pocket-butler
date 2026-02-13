/**
 * TanStack Query hooks for account data fetching
 */

import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { apiClient } from "@/lib/api-client";
import type { Account, CreateAccountInput, SyncAccountResult } from "@/types/api";

// ============================================================================
// Query Keys
// ============================================================================

export const accountKeys = {
  all: ["accounts"] as const,
  lists: () => [...accountKeys.all, "list"] as const,
  list: () => [...accountKeys.lists()] as const,
  details: () => [...accountKeys.all, "detail"] as const,
  detail: (id: string) => [...accountKeys.details(), id] as const,
};

// ============================================================================
// Query Hooks
// ============================================================================

/**
 * Fetch list of all accounts for the current user
 */
export function useAccounts() {
  return useQuery({
    queryKey: accountKeys.list(),
    queryFn: async () => {
      return apiClient<Account[]>("/v1/accounts");
    },
  });
}

/**
 * Fetch a single account by ID
 */
export function useAccount(accountId: string) {
  return useQuery({
    queryKey: accountKeys.detail(accountId),
    queryFn: async () => {
      return apiClient<Account>(`/v1/accounts/${accountId}`);
    },
    enabled: !!accountId,
  });
}

// ============================================================================
// Mutation Hooks
// ============================================================================

/**
 * Create a new account (wallet or exchange)
 */
export function useCreateAccount() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: async (input: CreateAccountInput) => {
      return apiClient<Account>("/v1/accounts", {
        method: "POST",
        body: input,
      });
    },
    onSuccess: () => {
      // Invalidate and refetch accounts list
      queryClient.invalidateQueries({ queryKey: accountKeys.lists() });
    },
  });
}

/**
 * Update an existing account
 */
export function useUpdateAccount(accountId: string) {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: async (input: Partial<CreateAccountInput>) => {
      return apiClient<Account>(`/v1/accounts/${accountId}`, {
        method: "PUT",
        body: input,
      });
    },
    onSuccess: () => {
      // Invalidate both list and specific account
      queryClient.invalidateQueries({ queryKey: accountKeys.lists() });
      queryClient.invalidateQueries({ queryKey: accountKeys.detail(accountId) });
    },
  });
}

/**
 * Delete an account
 */
export function useDeleteAccount() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: async (accountId: string) => {
      return apiClient<void>(`/v1/accounts/${accountId}`, {
        method: "DELETE",
      });
    },
    onSuccess: () => {
      // Invalidate accounts list
      queryClient.invalidateQueries({ queryKey: accountKeys.lists() });
    },
  });
}

/**
 * Sync an account to fetch latest holdings
 */
export function useSyncAccount() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: async (accountId: string) => {
      return apiClient<SyncAccountResult>(`/v1/accounts/${accountId}/sync`, {
        method: "POST",
      });
    },
    onSuccess: (_, accountId) => {
      // Invalidate account data and related portfolio holdings
      queryClient.invalidateQueries({ queryKey: accountKeys.detail(accountId) });
      queryClient.invalidateQueries({ queryKey: accountKeys.lists() });
      // Also invalidate portfolio holdings as they depend on account data
      queryClient.invalidateQueries({ queryKey: ["portfolios"] });
    },
  });
}
