/**
 * Shared MSW handlers for the three support-grant routes of functional-spec §8:
 * `POST /support-grants` (S1), `GET /support-grants/mine` (S2), and
 * `GET /support-grants/{grantId}/metadata` (S3).
 *
 * Test-support code only: nothing under `src/test/` is imported by the app, so
 * none of it reaches the production bundle. No handler here answers a route
 * the console does not call.
 */
import { delay, http, HttpResponse, type HttpHandler, type HttpResponseResolver } from "msw";
import type { SupportAccessGrant, SupportMetadata } from "../api/types";
import { activeGrant, supportMetadata } from "./fixtures";

export interface SupportGrantHandlerOptions {
  /** What `GET /support-grants/mine` answers, newest first. Defaults to none. */
  grants?: SupportAccessGrant[];
  /** What `GET /support-grants/{grantId}/metadata` answers. */
  metadata?: SupportMetadata;
  /** What a successful `POST /support-grants` answers with `201`. */
  created?: SupportAccessGrant;
  /** Milliseconds every handler waits before answering; 0 answers at once. */
  delayMs?: number;
}

/** The RFC 7807 body the gateway sends, with a stable correlation id per case. */
function problem(status: number, title: string, detail: string, correlationId: string): Response {
  return HttpResponse.json(
    {
      type: "https://errors.crypto-pocket-butler.test/support-grants",
      title,
      status,
      detail,
      correlationId,
    },
    {
      status,
      headers: { "Content-Type": "application/problem+json" },
    },
  );
}

function withDelay(
  resolve: HttpResponseResolver,
  delayMs: number | undefined,
): HttpResponseResolver {
  if (!delayMs || delayMs <= 0) {
    return resolve;
  }
  return async (info) => {
    await delay(delayMs);
    return resolve(info);
  };
}

/**
 * The happy path for all three routes. Suites override a single route by
 * registering a handler for it after this set.
 */
export function supportGrantHandlers(options: SupportGrantHandlerOptions = {}): HttpHandler[] {
  const grants = options.grants ?? [];
  const metadata = options.metadata ?? supportMetadata();
  const created =
    options.created ?? activeGrant({ id: "grant-created", status: "pending_approval" });
  const delayMs = options.delayMs;

  return [
    http.post(
      "*/v1/support-grants",
      withDelay(() => HttpResponse.json(created, { status: 201 }), delayMs),
    ),
    http.get(
      "*/v1/support-grants/mine",
      withDelay(() => HttpResponse.json(grants), delayMs),
    ),
    http.get(
      "*/v1/support-grants/:grantId/metadata",
      withDelay(() => HttpResponse.json(metadata), delayMs),
    ),
  ];
}

/** S1: the caller already has an open grant. */
export function supportGrantConflictHandlers(): HttpHandler[] {
  return [
    http.post("*/v1/support-grants", () =>
      problem(
        409,
        "A support grant is already open",
        "Approve or revoke the open grant before requesting another.",
        "corr-support-409",
      ),
    ),
  ];
}

/** S3: the grant is missing, revoked, expired, or belongs to someone else. */
export function supportGrantForbiddenHandlers(): HttpHandler[] {
  return [
    http.get("*/v1/support-grants/:grantId/metadata", () =>
      problem(
        403,
        "Support access is not available",
        "The grant is expired, revoked, or not yours.",
        "corr-support-403",
      ),
    ),
  ];
}

/** The gateway's burst limit on the manual triggers. */
export function supportGrantRateLimitedHandlers(): HttpHandler[] {
  return [
    http.get("*/v1/support-grants/mine", () =>
      HttpResponse.json(
        {
          type: "https://errors.crypto-pocket-butler.test/rate-limit",
          title: "Too many requests",
          status: 429,
          detail: "Wait before asking again.",
          correlationId: "corr-support-429",
        },
        {
          status: 429,
          headers: { "Content-Type": "application/problem+json", "Retry-After": "30" },
        },
      ),
    ),
  ];
}

/** The support API or Identity is down behind the gateway. */
export function supportGrantUnavailableHandlers(): HttpHandler[] {
  return [
    http.get("*/v1/support-grants/:grantId/metadata", () =>
      HttpResponse.json(
        {
          type: "https://errors.crypto-pocket-butler.test/unavailable",
          title: "Support API unavailable",
          status: 503,
          detail: "Try again shortly.",
          correlationId: "corr-support-503",
        },
        {
          status: 503,
          headers: { "Content-Type": "application/problem+json", "Retry-After": "60" },
        },
      ),
    ),
  ];
}
