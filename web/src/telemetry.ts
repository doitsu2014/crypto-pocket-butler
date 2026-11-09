import type { ClientErrorEvent } from "./api/types";

/**
 * Client error telemetry (NFR17.1).
 *
 * One structured event per failure — route, error code, correlation id, and a
 * per-page-load session id. No token, payload, or PII is ever sent. The beacon
 * never blocks rendering and fails silently.
 */

const GATEWAY_URL = (import.meta.env.VITE_GATEWAY_URL as string | undefined) ?? "/v1";

let currentSessionId: string | null = null;

export function sessionId(): string {
  if (currentSessionId) {
    return currentSessionId;
  }
  const cryptoApi = globalThis.crypto;
  currentSessionId =
    cryptoApi && typeof cryptoApi.randomUUID === "function"
      ? cryptoApi.randomUUID()
      : `web-session-${Date.now()}-${Math.random().toString(16).slice(2)}`;
  return currentSessionId;
}

export interface TelemetryDeps {
  endpoint?: string;
  sendBeacon?: (url: string, data: string) => boolean;
  fetchImpl?: typeof fetch;
}

export function reportClientError(
  event: ClientErrorEvent,
  deps: TelemetryDeps = {},
): void {
  const endpoint = deps.endpoint ?? `${GATEWAY_URL}/telemetry/client-errors`;
  const payload = JSON.stringify({
    route: event.route,
    errorCode: event.errorCode,
    correlationId: event.correlationId,
    sessionId: event.sessionId ?? sessionId(),
  });

  const beacon =
    deps.sendBeacon ??
    (typeof navigator !== "undefined" && typeof navigator.sendBeacon === "function"
      ? (url: string, data: string) => navigator.sendBeacon(url, data)
      : undefined);

  try {
    if (beacon && beacon(endpoint, payload) === true) {
      return;
    }
  } catch {
    // A blocked beacon is not fatal; fall through to fetch.
  }

  const fetchImpl = deps.fetchImpl ?? globalThis.fetch;
  if (typeof fetchImpl !== "function") {
    return;
  }
  void fetchImpl(endpoint, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: payload,
    keepalive: true,
  }).catch(() => {
    // Telemetry is best-effort and must never surface to the user.
  });
}
