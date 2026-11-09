import type { GrantRequestValues, GrantStatus, SupportAccessGrant } from "../api/types";
import type { StatusPresentation } from "./presentation";

export const MIN_REASON_LENGTH = 20;
export const MAX_GRANT_MINUTES = 480;
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
  durationMinutes?: string;
  approverId?: string;
}

export function validateGrantRequest(values: GrantRequestValues): GrantRequestErrors {
  const errors: GrantRequestErrors = {};

  if (!values.targetUserId?.trim()) {
    errors.targetUserId = "Enter the target user's id.";
  }

  const reason = values.reason?.trim() ?? "";
  if (reason.length < MIN_REASON_LENGTH) {
    errors.reason = `The reason must be at least ${MIN_REASON_LENGTH} characters.`;
  }

  if (
    !Number.isInteger(values.durationMinutes) ||
    values.durationMinutes < 1 ||
    values.durationMinutes > MAX_GRANT_MINUTES
  ) {
    errors.durationMinutes = `Choose a whole number of minutes between 1 and ${MAX_GRANT_MINUTES}.`;
  }

  const approver = values.approverId?.trim() ?? "";
  const requester = values.requesterId?.trim() ?? "";
  if (!approver) {
    errors.approverId = "A second approver is required.";
  } else if (approver.toLowerCase() === requester.toLowerCase()) {
    errors.approverId = "The approver must differ from the requester.";
  }

  return errors;
}

export function hasGrantRequestErrors(errors: GrantRequestErrors): boolean {
  return Object.values(errors).some((message) => message !== undefined);
}
