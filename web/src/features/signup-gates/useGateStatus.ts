/**
 * Entry resolution for the sign-up gates (functional-spec §2.1;
 * performance-design §2/§5/§8).
 *
 * One hook is the single resolution point the router guard and the screens
 * consume. G1 (read-only principal) and G2 (gate state) are issued in parallel
 * and the hook resolves when both settle: total entry latency is max(G1, G2),
 * not the sum. A `null` from either client method is the defined no-record
 * sentinel — a resolved value, never a rejection (performance-design §2).
 *
 * Requirements: FR19.2, FR19.4, FR19.5, NFR2.2, NFR2.6, NFR2.7, NFR24.1.
 */

import { useQuery } from "@tanstack/react-query";
import { useMemo } from "react";
import { gateRetryDelay, gatewayClient, isAutoRetryableGateError } from "../../api/client";
import { useAccountStatus } from "../../api/settingsQueries";
import type { Principal, SignupGateState } from "../../api/types";
import { useSession } from "../../auth/session";
import { deriveGateStatus, type GateStatus } from "./gateStatus";

export const GATE_QUERY_ROOT = "signup-gates";

/** The structural slice of the client this hook needs (tests inject a fake). */
export interface GateReader {
  getPrincipalReadOnly(): Promise<Principal | null>;
  getSignupGateState(): Promise<SignupGateState | null>;
}

export interface UseGateStatusOptions {
  /** Overrides the signed-in session id (tests and shell hand-off). */
  sessionUserId?: string;
  /** Overrides the gateway client (tests; defaults to the shared client). */
  client?: GateReader;
}

function queryUserId(sessionUserId: string | null): string {
  return sessionUserId ?? "anonymous";
}

/** Namespaced keys keep one session's gate state out of another's (NFR24.1). */
export function gateEntryKey(sessionUserId: string | null) {
  return [GATE_QUERY_ROOT, queryUserId(sessionUserId), "entry"] as const;
}

export function principalKey(sessionUserId: string | null) {
  return [...gateEntryKey(sessionUserId), "principal"] as const;
}

export function gateStateKey(sessionUserId: string | null) {
  return [...gateEntryKey(sessionUserId), "gate-state"] as const;
}

export function gateSessionKey(sessionUserId: string | null) {
  return [GATE_QUERY_ROOT, queryUserId(sessionUserId)] as const;
}

export function useGateStatus(options: UseGateStatusOptions = {}): GateStatus {
  const session = useSession();
  const sessionUserId = options.sessionUserId ?? session.session?.user?.id ?? null;
  const client: GateReader = options.client ?? gatewayClient;
  const enabled = Boolean(sessionUserId);

  // D0 answers for every status, including the three the principal read
  // refuses, so it is the first routing row (functional-spec §2.1).
  const accountQuery = useAccountStatus(enabled);

  const principalQuery = useQuery({
    queryKey: principalKey(sessionUserId),
    queryFn: () => client.getPrincipalReadOnly(),
    enabled,
    staleTime: 0,
    retry: (count, error) => count < 2 && isAutoRetryableGateError(error),
    retryDelay: gateRetryDelay,
  });

  const gateStateQuery = useQuery({
    queryKey: gateStateKey(sessionUserId),
    queryFn: () => client.getSignupGateState(),
    enabled,
    staleTime: 0,
    retry: (count, error) => count < 2 && isAutoRetryableGateError(error),
    retryDelay: gateRetryDelay,
  });

  return useMemo(
    () =>
      deriveGateStatus({
        principal: principalQuery.data,
        gateState: gateStateQuery.data,
        accountStatus: accountQuery.data?.status,
        error: principalQuery.error ?? gateStateQuery.error ?? accountQuery.error,
        isLoading:
          principalQuery.isPending || gateStateQuery.isPending || accountQuery.isPending,
      }),
    [
      accountQuery.data,
      accountQuery.error,
      accountQuery.isPending,
      principalQuery.data,
      principalQuery.error,
      principalQuery.isPending,
      gateStateQuery.data,
      gateStateQuery.error,
      gateStateQuery.isPending,
    ],
  );
}
