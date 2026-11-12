import type { GrantStatus, SupportAccessGrant } from "../api/types";
import type { StatusPresentation } from "./presentation";

export const MAX_REASON_LENGTH = 500;
export const GRANT_CLOCK_INTERVAL_MS = 30_000;

/**
 * The status that governs access right now. An `active` grant whose
 * `expiresAt` has passed is expired even if no writer has updated the row.
 * A missing or unparseable expiry fails closed (BR1.3).
 */
export function grantStatusAt(grant: SupportAccessGrant, nowMs: number): GrantStatus {
  if (grant.status !== "active") {
    return grant.status;
  }
  const expiresAt = grant.expiresAt === null ? Number.NaN : Date.parse(grant.expiresAt);
  if (!Number.isFinite(expiresAt) || expiresAt <= nowMs) {
    return "expired";
  }
  return "active";
}

export function canReadWithGrant(grant: SupportAccessGrant, nowMs: number): boolean {
  return grantStatusAt(grant, nowMs) === "active";
}

/**
 * Remaining time is shown as whole minutes and never as "0 minutes": zero or
 * less reads as "less than a minute left" (functional spec §6).
 */
export function formatRemainingMinutes(minutes: number): string {
  const whole = Math.max(0, Math.floor(minutes));
  if (whole < 1) {
    return "less than a minute left";
  }
  return whole === 1 ? "1 minute left" : `${whole} minutes left`;
}

export function grantStatusPresentation(status: GrantStatus): StatusPresentation {
  switch (status) {
    case "pending_approval":
      return { label: "Pending approval", tone: "warn", shape: "diamond" };
    case "active":
      return { label: "Active", tone: "success", shape: "dot" };
    case "expired":
      return { label: "Expired", tone: "danger", shape: "square" };
    case "revoked":
      return { label: "Revoked", tone: "danger", shape: "ring" };
  }
}

export interface GrantRequestErrors {
  targetUserId?: string;
  reason?: string;
}

/** W1: a request carries a target user id and a reason of 1–500 characters. */
export function validateGrantRequest(values: {
  targetUserId: string;
  reason: string;
}): GrantRequestErrors {
  const errors: GrantRequestErrors = {};

  if (!values.targetUserId?.trim()) {
    errors.targetUserId = "Enter the target user's id.";
  }

  const reason = values.reason?.trim() ?? "";
  if (!reason) {
    errors.reason = "Enter a reason.";
  } else if (reason.length > MAX_REASON_LENGTH) {
    errors.reason = `Use ${MAX_REASON_LENGTH} characters or fewer.`;
  }

  return errors;
}

export function hasGrantRequestErrors(errors: GrantRequestErrors): boolean {
  return Object.values(errors).some((message) => message !== undefined);
}
