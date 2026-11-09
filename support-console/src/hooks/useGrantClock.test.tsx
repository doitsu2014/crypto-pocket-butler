import { act, renderHook } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { canReadWithGrant, grantStatusAt } from "../domain/grants";
import { activeGrant } from "../test/fixtures";
import { useGrantClock } from "./useGrantClock";

const START = new Date("2026-11-08T10:00:00.000Z");

describe("useGrantClock", () => {
  beforeEach(() => {
    vi.useFakeTimers();
    vi.setSystemTime(START);
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it("ticks every 30 seconds and ends access at expiry", () => {
    const grant = activeGrant({ expiresAt: new Date(START.getTime() + 60_000).toISOString() });
    const { result } = renderHook(() => useGrantClock(grant.expiresAt));

    expect(canReadWithGrant(grant, result.current)).toBe(true);

    act(() => {
      vi.advanceTimersByTime(30_000);
    });
    expect(result.current).toBe(START.getTime() + 30_000);
    expect(canReadWithGrant(grant, result.current)).toBe(true);

    act(() => {
      vi.advanceTimersByTime(30_000);
    });
    expect(result.current).toBe(START.getTime() + 60_000);
    expect(canReadWithGrant(grant, result.current)).toBe(false);
    expect(grantStatusAt(grant, result.current)).toBe("expired");
  });

  it("ticks at expiry even when it comes before the poll interval", () => {
    const grant = activeGrant({ expiresAt: new Date(START.getTime() + 5_000).toISOString() });
    const { result } = renderHook(() => useGrantClock(grant.expiresAt, 30_000));

    act(() => {
      vi.advanceTimersByTime(5_000);
    });

    expect(result.current).toBe(START.getTime() + 5_000);
    expect(canReadWithGrant(grant, result.current)).toBe(false);
  });

  it("does not schedule a timer without an expiry", () => {
    const { result } = renderHook(() => useGrantClock(null));

    act(() => {
      vi.advanceTimersByTime(120_000);
    });

    expect(result.current).toBe(START.getTime());
  });
});
