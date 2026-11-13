/**
 * Settings server-state hooks (U7; frontend-components §3).
 *
 * Every gateway call stays in `client.ts`; these hooks only name the queries,
 * the cache keys, and what a successful mutation invalidates. Business math
 * never happens here.
 */

import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { gatewayClient } from "./client";
import type {
  AccountStatusView,
  ConsentDecisionView,
  ConsentHistoryView,
  ConsentStateView,
  DeletionView,
  EffectivePreferences,
  PreferencesPatch,
  ProfileView,
} from "./types";

const MINUTE = 60_000;

export const settingsKeys = {
  accountStatus: ["me", "account-status"] as const,
  profile: ["me", "profile"] as const,
  preferences: ["me", "preferences"] as const,
  consentState: ["me", "consent", "state"] as const,
  consentHistory: ["me", "consent", "history"] as const,
  consent: ["me", "consent"] as const,
  deletion: ["me", "deletion"] as const,
};

/** D0: the routing read. It answers for every status, including denied ones. */
export function useAccountStatus(enabled = true) {
  return useQuery<AccountStatusView>({
    queryKey: settingsKeys.accountStatus,
    queryFn: () => gatewayClient.getAccountStatus(),
    // NFR2.9: the entry resolution must see the status as it is now, so D0
    // overrides the app-wide default and refetches on every authenticated entry.
    staleTime: 0,
    enabled,
  });
}

/** P1: the profile the settings panel renders. */
export function useProfile() {
  return useQuery<ProfileView>({
    queryKey: settingsKeys.profile,
    queryFn: () => gatewayClient.getProfileView(),
    staleTime: MINUTE,
  });
}

export function useSaveDisplayName() {
  const queryClient = useQueryClient();
  return useMutation<ProfileView, Error, string>({
    mutationFn: (displayName) => gatewayClient.saveDisplayName(displayName),
    onSuccess: (profile) => {
      // The PATCH answer is authoritative; refetching here would race it and
      // re-render the previous name (the panel's tests stub GET separately).
      queryClient.setQueryData(settingsKeys.profile, profile);
    },
  });
}

export function useUseProviderName() {
  const queryClient = useQueryClient();
  return useMutation<ProfileView, Error, void>({
    mutationFn: () => gatewayClient.useProviderName(),
    onSuccess: (profile) => {
      queryClient.setQueryData(settingsKeys.profile, profile);
    },
  });
}

/** X2: effective values with their sources. */
export function usePreferences() {
  return useQuery<EffectivePreferences>({
    queryKey: settingsKeys.preferences,
    queryFn: () => gatewayClient.readPreferences(),
    staleTime: 5 * MINUTE,
  });
}

export function useSavePreferences() {
  const queryClient = useQueryClient();
  return useMutation<EffectivePreferences, Error, PreferencesPatch>({
    mutationFn: (patch) => gatewayClient.savePreferences(patch),
    onSuccess: (preferences) => {
      queryClient.setQueryData(settingsKeys.preferences, preferences);
      void queryClient.invalidateQueries({ queryKey: settingsKeys.preferences });
    },
  });
}

export function useConsentState() {
  return useQuery<ConsentStateView>({
    queryKey: settingsKeys.consentState,
    queryFn: () => gatewayClient.getConsentState(),
    staleTime: MINUTE,
  });
}

export function useConsentHistory() {
  return useQuery<ConsentHistoryView>({
    queryKey: settingsKeys.consentHistory,
    queryFn: () => gatewayClient.getConsentHistory(),
    staleTime: MINUTE,
  });
}

export function useGrantConsent() {
  const queryClient = useQueryClient();
  return useMutation<ConsentDecisionView, Error, { kind: string; policyVersion: string }>({
    mutationFn: ({ kind, policyVersion }) => gatewayClient.grantConsent(kind, policyVersion),
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: settingsKeys.consent });
    },
  });
}

export function useWithdrawConsent() {
  const queryClient = useQueryClient();
  return useMutation<ConsentDecisionView, Error, string>({
    mutationFn: (kind) => gatewayClient.withdrawConsent(kind),
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: settingsKeys.consent });
    },
  });
}

/**
 * D1: the open request, or `null` when the designed no-record answer arrives.
 * A `null` is a resolved value, never an error.
 */
export function useDeletionRequest() {
  return useQuery<DeletionView | null>({
    queryKey: settingsKeys.deletion,
    queryFn: () => gatewayClient.getOpenDeletion(),
    staleTime: MINUTE,
  });
}

export function useRequestDeletion() {
  const queryClient = useQueryClient();
  return useMutation<DeletionView, Error, void>({
    mutationFn: () => gatewayClient.requestDeletion(),
    onSuccess: (deletion) => {
      queryClient.setQueryData(settingsKeys.deletion, deletion);
      void queryClient.invalidateQueries({ queryKey: settingsKeys.deletion });
      void queryClient.invalidateQueries({ queryKey: settingsKeys.accountStatus });
    },
  });
}

export function useCancelDeletion() {
  const queryClient = useQueryClient();
  return useMutation<DeletionView, Error, void>({
    mutationFn: () => gatewayClient.cancelDeletion(),
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: settingsKeys.deletion });
      void queryClient.invalidateQueries({ queryKey: settingsKeys.accountStatus });
      void queryClient.invalidateQueries({ queryKey: settingsKeys.profile });
    },
  });
}
