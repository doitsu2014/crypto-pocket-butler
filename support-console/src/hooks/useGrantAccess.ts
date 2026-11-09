import type { GrantStatus, SupportAccessGrant } from "../api/types";
import { grantStatusAt } from "../domain/grants";
import { useGrantClock } from "./useGrantClock";

export interface GrantAccess {
  status: GrantStatus | null;
  isActive: boolean;
  now: number;
}

/** The UI mirror of BR1.3: expiry ends access even before a writer updates the row. */
export function useGrantAccess(grant: SupportAccessGrant | null): GrantAccess {
  const now = useGrantClock(grant?.expiresAt ?? null);
  const status = grant ? grantStatusAt(grant, now) : null;
  return { status, isActive: status === "active", now };
}
