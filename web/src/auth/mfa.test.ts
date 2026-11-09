import { describe, expect, it } from "vitest";
import {
  isLimitedSession,
  isMfaAllowedPath,
  requiresMfaEnrolment,
} from "./mfa";

describe("MFA routing", () => {
  it("requires enrolment when an exchange-credential user has no MFA", () => {
    expect(
      requiresMfaEnrolment({ mfaRequired: true, mfaEnrolled: false }),
    ).toBe(true);
  });

  it("does not require enrolment once it is complete", () => {
    expect(
      requiresMfaEnrolment({ mfaRequired: true, mfaEnrolled: true }),
    ).toBe(false);
  });

  it("does not require enrolment for a user without exchange credentials", () => {
    expect(
      requiresMfaEnrolment({ mfaRequired: false, mfaEnrolled: false }),
    ).toBe(false);
    expect(requiresMfaEnrolment(null)).toBe(false);
  });

  it("allows only the MFA and settings paths while enrolment is pending", () => {
    expect(isMfaAllowedPath("/settings/mfa")).toBe(true);
    expect(isMfaAllowedPath("/settings")).toBe(true);
    expect(isMfaAllowedPath("/dashboard")).toBe(false);
  });

  it("marks the session limited only when the user declined enrolment", () => {
    expect(
      isLimitedSession({ mfaRequired: true, mfaEnrolled: false }, true),
    ).toBe(true);
    expect(
      isLimitedSession({ mfaRequired: true, mfaEnrolled: true }, true),
    ).toBe(false);
  });
});
