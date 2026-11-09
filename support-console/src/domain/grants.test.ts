import { describe, expect, it } from "vitest";
import {
  canReadWithGrant,
  GRANT_CLOCK_INTERVAL_MS,
  grantStatusAt,
  grantStatusPresentation,
  hasGrantRequestErrors,
  MAX_GRANT_MINUTES,
  MIN_REASON_LENGTH,
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
    expect(grantStatusAt(expiredGrant(), NOW)).toBe("expired");
  });
});

describe("validateGrantRequest", () => {
  const valid = {
    requesterId: "agent-1",
    targetUserId: "user-1",
    reason: "Investigating a reported sync failure",
    durationMinutes: 60,
    approverId: "agent-2",
  };

  it("accepts a complete request", () => {
    expect(validateGrantRequest(valid)).toEqual({});
    expect(hasGrantRequestErrors(validateGrantRequest(valid))).toBe(false);
  });

  it("requires a target user", () => {
    const errors = validateGrantRequest({ ...valid, targetUserId: "   " });
    expect(errors.targetUserId).toMatch(/target user/i);
    expect(hasGrantRequestErrors(errors)).toBe(true);
  });

  it("requires a reason of at least 20 characters after trimming", () => {
    expect(validateGrantRequest({ ...valid, reason: "short reason" }).reason).toMatch(/20/);
    expect(
      validateGrantRequest({ ...valid, reason: `  ${"a".repeat(MIN_REASON_LENGTH - 1)}  ` }).reason,
    ).toMatch(/20/);
    expect(
      validateGrantRequest({ ...valid, reason: `  ${"a".repeat(MIN_REASON_LENGTH)}  ` }).reason,
    ).toBeUndefined();
  });

  it("accepts only whole minutes from 1 to 480", () => {
    expect(validateGrantRequest({ ...valid, durationMinutes: 0 }).durationMinutes).toMatch(/1/);
    expect(
      validateGrantRequest({ ...valid, durationMinutes: MAX_GRANT_MINUTES + 1 }).durationMinutes,
    ).toMatch(/480/);
    expect(validateGrantRequest({ ...valid, durationMinutes: 12.5 }).durationMinutes).toMatch(
      /whole/i,
    );
    expect(validateGrantRequest({ ...valid, durationMinutes: 1 }).durationMinutes).toBeUndefined();
    expect(
      validateGrantRequest({ ...valid, durationMinutes: MAX_GRANT_MINUTES }).durationMinutes,
    ).toBeUndefined();
  });

  it("requires a second approver who is not the requester", () => {
    expect(validateGrantRequest({ ...valid, approverId: "" }).approverId).toMatch(/required/i);
    expect(
      validateGrantRequest({ ...valid, approverId: " AGENT-1 " }).approverId,
    ).toMatch(/differ from the requester/i);
    expect(validateGrantRequest({ ...valid, approverId: "agent-3" }).approverId).toBeUndefined();
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
});
