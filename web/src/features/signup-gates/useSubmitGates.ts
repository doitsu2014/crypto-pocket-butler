/**
 * G3 submit lifecycle (functional-spec §2.4/§2.6; performance-design §6;
 * security-design §4).
 *
 * The mutation never auto-retries (`retry: 0`); every attempt is a manual user
 * action. The idempotency ticket is resolved at submit time so an identical
 * retry reuses the key and any field edit mints a new one. Outcomes are the
 * screen's only vocabulary — the ProblemDetails body is never parsed for field
 * names and its `detail` is never rendered.
 *
 * Requirements: FR19.4, FR19.5, NFR2.3, NFR2.6, NFR11.1, NFR11.2.
 */

import { useMutation, useQueryClient } from "@tanstack/react-query";
import {
  ApiError,
  gateFailureVariant,
  gateRetryDelay,
  gatewayClient,
  isAutoRetryableGateError,
} from "../../api/client";
import type { SignupGateState } from "../../api/types";
import type { GateFormValues, ServerMissing } from "./gateStatus";
import { discardTicket, ticketKeyFor } from "./submissionTicket";
import { gateSessionKey, gateStateKey } from "./useGateStatus";

/** The structural slice of the client this hook needs (tests inject a fake). */
export interface GateSubmitter {
  submitSignupGates(values: GateFormValues, idempotencyKey: string): Promise<void>;
  getSignupGateState(): Promise<SignupGateState | null>;
}

/** What the screen renders after one submit attempt. */
export type SubmitOutcome =
  | { kind: "passed" }
  | { kind: "blocked" }
  | { kind: "unasserted"; missing: ServerMissing[] }
  | { kind: "nothing-saved"; correlationId?: string }
  | { kind: "incomplete"; correlationId?: string; missing?: ServerMissing[] };

export interface UseSubmitGatesOptions {
  sessionUserId: string;
  /** Overrides the gateway client (tests; defaults to the shared client). */
  client?: GateSubmitter;
}

function correlationIdOf(error: unknown): string | undefined {
  return error instanceof ApiError ? error.problem.correlationId : undefined;
}

export function useSubmitGates({ sessionUserId, client = gatewayClient }: UseSubmitGatesOptions) {
  const queryClient = useQueryClient();
  const mutation = useMutation({
    retry: 0,
    mutationFn: (values: GateFormValues) => {
      const key = ticketKeyFor(sessionUserId, values);
      return client.submitSignupGates(values, key);
    },
  });

  /**
   * Re-reads G2 after a failure that may have committed something. `null` means
   * the re-read itself failed: callers must not turn that into "nothing is
   * outstanding", which would read as "everything is recorded".
   */
  async function readMissing(): Promise<ServerMissing[] | null> {
    try {
      const state = await queryClient.fetchQuery({
        queryKey: gateStateKey(sessionUserId),
        queryFn: () => client.getSignupGateState(),
        staleTime: 0,
        retry: (count, error) => count < 2 && isAutoRetryableGateError(error),
        retryDelay: gateRetryDelay,
      });
      return state?.missing ?? [];
    } catch {
      return null;
    }
  }

  async function submit(values: GateFormValues): Promise<SubmitOutcome> {
    try {
      await mutation.mutateAsync(values);
    } catch (error) {
      const variant = gateFailureVariant(error);
      if (variant === "restricted") {
        // Terminal for the session: no retry, no G2 read (spec §2.5).
        return { kind: "blocked" };
      }
      if (variant === "unasserted") {
        await queryClient.invalidateQueries({ queryKey: gateSessionKey(sessionUserId) });
        const missing = await readMissing();
        if (missing === null) {
          // The server rejected the assertions and the replacement read failed:
          // values are kept and the visitor retries, but we never claim success
          // by rendering an empty missing list.
          return { kind: "incomplete", correlationId: correlationIdOf(error) };
        }
        return { kind: "unasserted", missing };
      }
      if (variant === "unavailable") {
        // Geolocation unavailable: nothing can have committed (spec §2.6).
        return { kind: "nothing-saved", correlationId: correlationIdOf(error) };
      }
      // Possibly partial: re-read G2 so a retry shows what already committed.
      await queryClient.invalidateQueries({ queryKey: gateSessionKey(sessionUserId) });
      const missing = await readMissing();
      return {
        kind: "incomplete",
        correlationId: correlationIdOf(error),
        ...(missing === null ? {} : { missing }),
      };
    }

    discardTicket(sessionUserId);
    await queryClient.invalidateQueries({ queryKey: gateSessionKey(sessionUserId) });
    return { kind: "passed" };
  }

  return { submit, isPending: mutation.isPending };
}
