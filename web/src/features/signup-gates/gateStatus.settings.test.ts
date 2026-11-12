import { describe, expect, it } from "vitest";
import type { Principal, SignupGateState } from "../../api/types";
import { deriveGateStatus } from "./gateStatus";

const principal: Principal = {
  userId: "user-1",
  role: "owner",
  status: "active",
  emailVerified: true,
  nextStep: "none",
};

const passedGates: SignupGateState = { missing: [], jurisdictionBlocked: false };

function resolve(accountStatus?: Parameters<typeof deriveGateStatus>[0]["accountStatus"]) {
  return deriveGateStatus({
    principal,
    gateState: passedGates,
    isLoading: false,
    accountStatus,
  });
}

describe("gateStatus U7 refinement", () => {
  it("sends a pending-deletion account to the restore screen", () => {
    expect(resolve("pending_deletion")).toEqual({
      kind: "owned-elsewhere",
      destination: "restore-account",
    });
  });

  it("sends a disabled account to the blocked screen", () => {
    expect(resolve("disabled")).toEqual({
      kind: "owned-elsewhere",
      destination: "account-blocked",
    });
  });

  it("sends a deleted account to the blocked screen", () => {
    expect(resolve("deleted")).toEqual({
      kind: "owned-elsewhere",
      destination: "account-blocked",
    });
  });

  it("leaves active and pending accounts in the existing gate flow", () => {
    expect(resolve("active").kind).toBe("passed");
    expect(resolve("pending").kind).toBe("passed");
    expect(resolve().kind).toBe("passed");
  });

  it("splits a principal-reported pending_deletion to restore rather than blocked", () => {
    const status = deriveGateStatus({
      principal: { ...principal, status: "pending_deletion" },
      gateState: passedGates,
      isLoading: false,
    });
    expect(status).toEqual({ kind: "owned-elsewhere", destination: "restore-account" });
  });

  it("keeps a principal-reported disabled account on the blocked screen", () => {
    const status = deriveGateStatus({
      principal: { ...principal, status: "disabled" },
      gateState: passedGates,
      isLoading: false,
    });
    expect(status).toEqual({ kind: "owned-elsewhere", destination: "account-blocked" });
  });
});
