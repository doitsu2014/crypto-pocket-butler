import { describe, expect, it } from "vitest";
import {
  canReadWithGrant,
  formatRemainingMinutes,
  GRANT_CLOCK_INTERVAL_MS,
  grantStatusAt,
  grantStatusPresentation,
  hasGrantRequestErrors,
  MAX_REASON_LENGTH,
  validateGrantRequest,
} from "./grants";
import { activeGrant, expiredGrant, pendingGrant } from "../test/fixtures";

const NOW = Date.parse("2026-11-08T10:00:00.000Z");

function activeGrantAt(expiresAt: string) {
  return activeGrant({
    requestedAt: "2026-11-08T09:00:00.000Z",
    expiresAt,
    approvedAt: "2026-11-08T09:05:00.000Z",
  });
}

describe("grant state", () => {
  it("keeps an active grant active before expiresAt", () => {
    expect(grantStatusAt(activeGrantAt("2026-11-08T10:00:01.000Z"), NOW)).toBe("active");
    expect(canReadWithGrant(activeGrantAt("2026-11-08T10:00:01.000Z"), NOW)).toBe(true);
  });

  it("ends access at expiresAt", () => {
    expect(grantStatusAt(activeGrantAt("2026-11-08T10:00:00.000Z"), NOW)).toBe("expired");
    expect(canReadWithGrant(activeGrantAt("2026-11-08T10:00:00.000Z"), NOW)).toBe(false);
  });

  it("fails closed when an active grant has no valid expiry", () => {
    expect(grantStatusAt(activeGrant({ expiresAt: null }), NOW)).toBe("expired");
    expect(grantStatusAt(activeGrantAt("not-a-timestamp"), NOW)).toBe("expired");
  });

  it("leaves pending and revoked grants untouched", () => {
    expect(grantStatusAt(pendingGrant(), NOW)).toBe("pending_approval");
    expect(grantStatusAt(activeGrant({ status: "revoked" }), NOW)).toBe("revoked");
    expect(grantStatusAt(expiredGrant({ expiresAt: "2026-11-08T09:59:00.000Z" }), NOW)).toBe(
      "expired",
    );
  });
});

describe("validateGrantRequest", () => {
  const valid = {
    targetUserId: "user-1",
    reason: "Investigating a reported sync failure",
  };

  it("accepts a complete request", () => {
    expect(validateGrantRequest(valid)).toEqual({});
    expect(hasGrantRequestErrors(validateGrantRequest(valid))).toBe(false);
  });

  it("requires a target user id after trimming", () => {
    const errors = validateGrantRequest({ ...valid, targetUserId: "   " });
    expect(errors.targetUserId).toMatch(/target user/i);
    expect(hasGrantRequestErrors(errors)).toBe(true);
  });

  it("accepts a reason from one to 500 characters after trimming", () => {
    expect(validateGrantRequest({ ...valid, reason: " a " }).reason).toBeUndefined();
    expect(
      validateGrantRequest({ ...valid, reason: `  ${"a".repeat(MAX_REASON_LENGTH)}  ` }).reason,
    ).toBeUndefined();
  });

  it("requires a reason and refuses one longer than 500 characters", () => {
    expect(validateGrantRequest({ ...valid, reason: "   " }).reason).toMatch(/enter a reason/i);
    expect(
      validateGrantRequest({ ...valid, reason: "a".repeat(MAX_REASON_LENGTH + 1) }).reason,
    ).toMatch(/500 characters or fewer/i);
    expect(MAX_REASON_LENGTH).toBe(500);
  });
});

describe("grant presentation", () => {
  it("gives every status a label, a tone, and a shape", () => {
    const statuses = ["pending_approval", "active", "expired", "revoked"] as const;
    for (const status of statuses) {
      const presentation = grantStatusPresentation(status);
      expect(presentation.label.length).toBeGreaterThan(0);
      expect(["success", "warn", "danger", "neutral"]).toContain(presentation.tone);
      expect(["dot", "diamond", "square", "ring"]).toContain(presentation.shape);
    }
    expect(grantStatusPresentation("active").label).toBe("Active");
    expect(grantStatusPresentation("pending_approval").label).toBe("Pending approval");
    expect(grantStatusPresentation("expired").label).toBe("Expired");
    expect(grantStatusPresentation("revoked").label).toBe("Revoked");
  });

  it("pins the expiry poll interval at 30 seconds", () => {
    expect(GRANT_CLOCK_INTERVAL_MS).toBe(30_000);
  });

  it("renders remaining time as whole minutes and never zero", () => {
    expect(formatRemainingMinutes(42)).toBe("42 minutes left");
    expect(formatRemainingMinutes(1)).toBe("1 minute left");
    expect(formatRemainingMinutes(0)).toBe("less than a minute left");
  });
});
