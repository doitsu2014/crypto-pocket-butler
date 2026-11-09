import { describe, expect, it, vi } from "vitest";
import { reportClientError, sessionId } from "./telemetry";

const event = {
  route: "/dashboard",
  errorCode: "bootstrap_failed",
  correlationId: "corr-1",
};

describe("client telemetry", () => {
  it("prefers sendBeacon and does not touch fetch", () => {
    const beacon = vi.fn().mockReturnValue(true);
    const fetchImpl = vi.fn();

    reportClientError(event, { sendBeacon: beacon, fetchImpl });

    expect(beacon).toHaveBeenCalledOnce();
    expect(fetchImpl).not.toHaveBeenCalled();
    const [url, payload] = beacon.mock.calls[0] as [string, string];
    expect(url).toContain("/telemetry/client-errors");
    expect(JSON.parse(payload)).toMatchObject(event);
  });

  it("falls back to fetch when the beacon is refused", async () => {
    const beacon = vi.fn().mockReturnValue(false);
    const fetchImpl = vi.fn().mockResolvedValue({ ok: true });

    reportClientError(event, { sendBeacon: beacon, fetchImpl });

    await vi.waitFor(() => expect(fetchImpl).toHaveBeenCalledOnce());
    const [, init] = fetchImpl.mock.calls[0] as [string, RequestInit];
    expect(init.method).toBe("POST");
    expect(init.keepalive).toBe(true);
  });

  it("silently drops when both transports fail, without throwing", () => {
    const beacon = vi.fn().mockImplementation(() => {
      throw new Error("beacon blocked");
    });
    const fetchImpl = vi.fn().mockRejectedValue(new Error("offline"));

    expect(() => reportClientError(event, { sendBeacon: beacon, fetchImpl })).not.toThrow();
  });

  it("sends one stable session id per page load and no payload or token", () => {
    const beacon = vi.fn().mockReturnValue(true);

    reportClientError(event, { sendBeacon: beacon });
    reportClientError(event, { sendBeacon: beacon });

    const first = JSON.parse((beacon.mock.calls[0] as [string, string])[1]);
    const second = JSON.parse((beacon.mock.calls[1] as [string, string])[1]);
    expect(first.sessionId).toBe(second.sessionId);
    expect(first.sessionId).toBe(sessionId());
    expect(first).not.toHaveProperty("token");
    expect(first).not.toHaveProperty("payload");
  });
});
