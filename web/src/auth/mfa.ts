/**
 * MFA route gating (NFR10.1, C1 amendment).
 *
 * A user who must enrol in MFA (one with exchange credentials at aal1) is
 * routed to the enrolment panel before any portfolio route. Declining leaves a
 * limited session: a persistent banner and the connect/trade actions disabled.
 */

export interface MfaProfile {
  mfaRequired?: boolean;
  mfaEnrolled?: boolean;
}

export function requiresMfaEnrolment(profile: MfaProfile | null | undefined): boolean {
  return Boolean(profile?.mfaRequired && !profile?.mfaEnrolled);
}

/** Paths reachable while MFA enrolment is pending. */
export function isMfaAllowedPath(pathname: string): boolean {
  return pathname === "/settings/mfa" || pathname === "/settings";
}

/** True when the user declined enrolment and the session is limited. */
export function isLimitedSession(
  profile: MfaProfile | null | undefined,
  declined: boolean,
): boolean {
  return Boolean(declined && requiresMfaEnrolment(profile));
}
