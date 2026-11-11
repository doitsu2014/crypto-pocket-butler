/**
 * Gate-state types and the entry-routing decision (functional spec §2.1;
 * frontend-components.md §2).
 *
 * The union is transcribed from the design verbatim. `deriveGateStatus` is the
 * single place the "rows are evaluated top to bottom, first match wins" rule is
 * applied, so the guard, the screen, and the tests cannot drift apart.
 *
 * Requirements: FR19.2, FR19.3, FR19.4, FR19.5, US6.2, US6.3, US6.5.
 */

import { ApiError } from "../../api/client";
import type { Principal, SignupGateState } from "../../api/types";

/** The only items the server can name (G2 `missing`): the three consent kinds. */
export type ServerMissing = "terms" | "risk" | "age";

/** Client-side validation additionally names the country (spec §2.3). */
export type ClientMissing = ServerMissing | "country";

/** Screens U8 hands a visitor to instead of rendering the gate (spec §2.1). */
export type GateDestination = "verify-email" | "enrol-mfa" | "restore-account" | "account-blocked";

/** Gate state as resolved from G1+G2; drives `RequireGatesPassed` routing. */
export type GateStatus =
  | { kind: "loading" }
  | { kind: "passed" }
  | {
      kind: "gates-required";
      serverMissing: ServerMissing[];
      /**
       * True when both entry reads answered "no record": the form must start
       * with every acceptance unchecked even though `serverMissing` is empty
       * (spec §2.1 "no record" row). An empty `serverMissing` on a live record
       * instead means every acceptance is already recorded.
       */
      noRecord?: boolean;
    }
  | { kind: "jurisdiction-blocked"; continuedUse: boolean }
  | { kind: "owned-elsewhere"; destination: GateDestination }
  | { kind: "error"; correlationId?: string };

/** The three consent kinds, in the screen's field order (spec §2.2/§2.3). */
export const CONSENT_KINDS: readonly ServerMissing[] = ["terms", "risk", "age"];

/** Every field the missing-acceptance summary can name, in reading order. */
export const FIELD_ORDER: readonly ClientMissing[] = ["country", "terms", "risk", "age"];

/**
 * Where an `owned-elsewhere` visitor lands (plan decision 6). `verify-email`
 * and `settings/mfa` exist in the router; `account/restore` and
 * `account/blocked` are U7 screens the router does not register yet, so the
 * guard renders a terminal surface for those two instead of navigating (which
 * would loop through the catch-all).
 */
export const GATE_DESTINATIONS: Record<GateDestination, string> = {
  "verify-email": "/verify-email",
  "enrol-mfa": "/settings/mfa",
  "restore-account": "/account/restore",
  "account-blocked": "/account/blocked",
};

/** The form payload carried by G3 — exactly four fields (spec §3). */
export interface GateFormValues {
  countryCode: string;
  terms: boolean;
  risk: boolean;
  age: boolean;
}

export interface GateResolutionInput {
  principal: Principal | null | undefined;
  gateState: SignupGateState | null | undefined;
  /** A terminal read failure; the only road to the `error` state. */
  error?: unknown;
  isLoading: boolean;
}

function correlationIdOf(error: unknown): string | undefined {
  return error instanceof ApiError ? error.problem.correlationId : undefined;
}

/**
 * Resolves the visitor's destination from the two read-only entry answers.
 * Callers pass `principal`/`gateState` only once their queries have settled;
 * `null` is the defined "no record" sentinel (spec §2.1).
 */
export function deriveGateStatus({
  principal,
  gateState,
  error,
  isLoading,
}: GateResolutionInput): GateStatus {
  // Row 1 — a failed entry read never routes into the app.
  if (error) {
    return { kind: "error", correlationId: correlationIdOf(error) };
  }

  if (isLoading || principal === undefined || gateState === undefined) {
    return { kind: "loading" };
  }

  // Row 2 — no record exists (or the gate state is not defined): Gate Screen.
  if (principal === null || gateState === null) {
    return { kind: "gates-required", serverMissing: [], noRecord: true };
  }

  // Rows 3–5 — an outstanding next step is owned by the shell or U7.
  if (principal.nextStep === "verify_email") {
    return { kind: "owned-elsewhere", destination: "verify-email" };
  }
  if (principal.nextStep === "enrol_mfa") {
    return { kind: "owned-elsewhere", destination: "enrol-mfa" };
  }
  if (principal.nextStep === "restore_account") {
    return { kind: "owned-elsewhere", destination: "restore-account" };
  }

  // Rows 6–7 — a blocked or deleted account never sees the gate screen.
  if (
    principal.status === "disabled" ||
    principal.status === "pending_deletion" ||
    principal.status === "deleted"
  ) {
    return { kind: "owned-elsewhere", destination: "account-blocked" };
  }

  // Row 8 — gates incomplete on a live record.
  if (principal.status === "pending" || gateState.missing.length > 0) {
    return { kind: "gates-required", serverMissing: [...gateState.missing] };
  }

  // Row 9 — continued use blocked by jurisdiction (AC6.3.2).
  if (gateState.jurisdictionBlocked) {
    return { kind: "jurisdiction-blocked", continuedUse: true };
  }

  // Row 10 — every gate passed on the server.
  return { kind: "passed" };
}
