import { act, renderHook } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { activeGrant, pendingGrant } from "../test/fixtures";
import { useGrantAccess } from "./useGrantAccess";

const START = new Date("2026-11-08T10:00:00.000Z");

describe("useGrantAccess", () => {
  beforeEach(() => {
    vi.useFakeTimers();
    vi.setSystemTime(START);
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it("reports an active, unexpired grant as readable", () => {
    const grant = activeGrant({ expiresAt: new Date(START.getTime() + 60_000).toISOString() });
    const { result } = renderHook(() => useGrantAccess(grant));

    expect(result.current.status).toBe("active");
    expect(result.current.isActive).toBe(true);
  });

  it("flips to expired when the clock crosses expiresAt", () => {
    const grant = activeGrant({ expiresAt: new Date(START.getTime() + 10_000).toISOString() });
    const { result } = renderHook(() => useGrantAccess(grant));

    act(() => {
      vi.advanceTimersByTime(10_000);
    });

    expect(result.current.status).toBe("expired");
    expect(result.current.isActive).toBe(false);
  });

  it("never grants access for a pending or missing grant", () => {
    const pending = renderHook(() => useGrantAccess(pendingGrant())).result;
    expect(pending.current.status).toBe("pending_approval");
    expect(pending.current.isActive).toBe(false);

    const missing = renderHook(() => useGrantAccess(null)).result;
    expect(missing.current.status).toBeNull();
    expect(missing.current.isActive).toBe(false);
  });
});
