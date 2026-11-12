import { http, HttpResponse } from "msw";
import { afterEach, describe, expect, it, vi } from "vitest";
import * as client from "./client";
import {
  ApiError,
  approveSupportGrant,
  DeadlineError,
  declareIncident,
  fetchSupportMetadata,
  GrantScopeError,
  listMyGrants,
  listTickets,
  NetworkError,
  requestSupportGrant,
  revokeSupportGrant,
  updateIncident,
} from "./client";
import { activeGrant, problemDetails, supportMetadata, ticketMetadata } from "../test/fixtures";
import { server } from "../test/server";

const TOKEN = "support-token";

function jsonResponse(body: unknown, status = 200) {
  return HttpResponse.json(body as Parameters<typeof HttpResponse.json>[0], { status });
}

function problemResponse(status: number, title: string, correlationId = "corr-1") {
  return HttpResponse.json(problemDetails(status, title, correlationId), {
    status,
    headers: { "Content-Type": "application/problem+json" },
  });
}

/** A fetch that never settles until its request's signal aborts. */
function abortAwareFetch(): typeof fetch {
  return vi.fn(
    (_input: unknown, init?: RequestInit) =>
      new Promise<Response>((_resolve, reject) => {
        init?.signal?.addEventListener("abort", () =>
          reject(new DOMException("aborted", "AbortError")),
        );
      }),
  ) as unknown as typeof fetch;
}

afterEach(() => {
  vi.useRealTimers();
  vi.unstubAllGlobals();
});

describe("grant calls", () => {
  it("creates a grant with a bearer token, a correlation id, and exactly {userId, reason}", async () => {
    const grant = activeGrant({ status: "pending_approval" });
    let seenBody: unknown;
    let seenAuthorization: string | null = null;
    let seenCorrelation: string | null = null;

    server.use(
      http.post("*/v1/support-grants", async ({ request }) => {
        seenBody = await request.json();
        seenAuthorization = request.headers.get("Authorization");
        seenCorrelation = request.headers.get("x-correlation-id");
        return jsonResponse(grant, 201);
      }),
    );

    const created = await requestSupportGrant(
      {
        userId: "user-1",
        reason: "Investigating a reported sync failure",
      },
      TOKEN,
    );

    expect(created).toEqual(grant);
    expect(seenBody).toEqual({
      userId: "user-1",
      reason: "Investigating a reported sync failure",
    });
    expect(seenBody).not.toHaveProperty("durationMinutes");
    expect(seenBody).not.toHaveProperty("approverId");
    expect(seenAuthorization).toBe(`Bearer ${TOKEN}`);
    expect(seenCorrelation).toEqual(expect.any(String));
  });

  it("accepts a grant answer keyed grantId", async () => {
    const { id, ...withoutId } = activeGrant({ id: "grant-1" });
    server.use(
      http.post("*/v1/support-grants", () => jsonResponse({ ...withoutId, grantId: id }, 201)),
    );

    const created = await requestSupportGrant(
      { userId: "user-1", reason: "Investigating a reported sync failure" },
      TOKEN,
    );

    expect(created.id).toBe("grant-1");
  });

  it("approves and revokes a grant on the C2 routes", async () => {
    const seen: string[] = [];
    const grant = activeGrant();
    server.use(
      http.post("*/v1/support-grants/*/approve", ({ request }) => {
        seen.push(`POST ${new URL(request.url).pathname}`);
        return jsonResponse(grant);
      }),
      http.delete("*/v1/support-grants/*", ({ request }) => {
        seen.push(`DELETE ${new URL(request.url).pathname}`);
        return new HttpResponse(null, { status: 204 });
      }),
    );

    expect(await approveSupportGrant("grant-1", TOKEN)).toEqual(grant);
    await expect(revokeSupportGrant("grant-1", TOKEN)).resolves.toBeUndefined();

    expect(seen).toEqual([
      "POST /v1/support-grants/grant-1/approve",
      "DELETE /v1/support-grants/grant-1",
    ]);
  });
});

describe("the agent's own grants", () => {
  it("reads GET /support-grants/mine and maps an empty list", async () => {
    let seenPath: string | null = null;
    let seenAuthorization: string | null = null;
    server.use(
      http.get("*/v1/support-grants/mine", ({ request }) => {
        seenPath = new URL(request.url).pathname;
        seenAuthorization = request.headers.get("Authorization");
        return jsonResponse([]);
      }),
    );

    await expect(listMyGrants(TOKEN)).resolves.toEqual([]);
    expect(seenPath).toBe("/v1/support-grants/mine");
    expect(seenAuthorization).toBe(`Bearer ${TOKEN}`);
  });

  it("maps a newest-first list keyed grantId into grants", async () => {
    const newest = activeGrant({ id: "grant-new", status: "pending_approval" });
    const { id, ...oldestBody } = activeGrant({ id: "grant-old" });
    server.use(
      http.get("*/v1/support-grants/mine", () =>
        jsonResponse([
          newest,
          {
            ...oldestBody,
            grantId: id,
            credentials: { apiKey: "never-render" },
          },
        ]),
      ),
    );

    const grants = await listMyGrants(TOKEN);

    expect(grants.map((grant) => grant.id)).toEqual(["grant-new", "grant-old"]);
    expect(JSON.stringify(grants)).not.toMatch(/credential|never-render/i);
  });

  it("aborts with DeadlineError at fifteen seconds", async () => {
    vi.useFakeTimers();
    const fetchImpl = abortAwareFetch();

    const pending = listMyGrants(TOKEN, fetchImpl);
    const assertion = expect(pending).rejects.toBeInstanceOf(DeadlineError);
    await vi.advanceTimersByTimeAsync(15_000);
    await assertion;
    expect(fetchImpl).toHaveBeenCalledTimes(1);
  });

  it("clears its timer on a settled call so it never aborts a later one", async () => {
    vi.useFakeTimers();
    let settledSignal: AbortSignal | null = null;
    const settledFetch = vi.fn(async (_input: unknown, init?: RequestInit) => {
      settledSignal = init?.signal ?? null;
      return new Response(JSON.stringify([activeGrant()]), {
        status: 200,
        headers: { "Content-Type": "application/json" },
      });
    }) as unknown as typeof fetch;

    await listMyGrants(TOKEN, settledFetch);
    await vi.advanceTimersByTimeAsync(30_000);

    expect(settledSignal).not.toBeNull();
    expect((settledSignal as AbortSignal | null)?.aborted).toBe(false);
  });
});

describe("grant-scoped support metadata", () => {
  it("reads GET /support-grants/{grantId}/metadata and narrows the projection", async () => {
    let seenPath: string | null = null;
    server.use(
      http.get("*/v1/support-grants/*/metadata", ({ request }) => {
        seenPath = new URL(request.url).pathname;
        return jsonResponse({
          ...supportMetadata(),
          credentials: { apiKey: "never-render" },
          holdings: [{ asset: "BTC", quantity: "1" }],
          valuation: "1000.00",
        });
      }),
    );

    const metadata = await fetchSupportMetadata("grant-1", TOKEN);

    expect(seenPath).toBe("/v1/support-grants/grant-1/metadata");
    expect(metadata).toEqual(supportMetadata());
    expect(JSON.stringify(metadata)).not.toMatch(/credential|holding|valuation|never-render/i);
  });

  it("encodes the grant id and trims it before use", async () => {
    let seenPath: string | null = null;
    server.use(
      http.get("*/v1/support-grants/*/metadata", ({ request }) => {
        seenPath = new URL(request.url).pathname;
        return jsonResponse(supportMetadata());
      }),
    );

    await fetchSupportMetadata("  grant/1  ", TOKEN);

    expect(seenPath).toBe("/v1/support-grants/grant%2F1/metadata");
  });

  it("refuses a blank grant id and makes no request", async () => {
    const handler = vi.fn();
    server.use(
      http.get("*/v1/support-grants/*/metadata", () => {
        handler();
        return jsonResponse(supportMetadata());
      }),
    );

    await expect(fetchSupportMetadata("   ", TOKEN)).rejects.toBeInstanceOf(GrantScopeError);
    expect(handler).not.toHaveBeenCalled();
  });

  it("rejects a malformed projection rather than rendering a partial one", async () => {
    server.use(
      http.get("*/v1/support-grants/*/metadata", () =>
        jsonResponse({ ...supportMetadata(), emailVerified: "yes" }),
      ),
    );

    await expect(fetchSupportMetadata("grant-1", TOKEN)).rejects.toThrow(/metadata/i);
  });

  it("aborts with DeadlineError at fifteen seconds", async () => {
    vi.useFakeTimers();
    const fetchImpl = abortAwareFetch();

    const pending = fetchSupportMetadata("grant-1", TOKEN, fetchImpl);
    const assertion = expect(pending).rejects.toBeInstanceOf(DeadlineError);
    await vi.advanceTimersByTimeAsync(15_000);
    await assertion;
    expect(fetchImpl).toHaveBeenCalledTimes(1);
  });
});

describe("retired calls", () => {
  it("no longer exports getSupportGrant, searchAccounts, or fetchAccountMetadata", () => {
    expect("getSupportGrant" in client).toBe(false);
    expect("searchAccounts" in client).toBe(false);
    expect("fetchAccountMetadata" in client).toBe(false);
  });
});

describe("tickets and incidents", () => {
  it("requires a grant id for the ticket list", async () => {
    await expect(listTickets("   ", "open", TOKEN)).rejects.toBeInstanceOf(GrantScopeError);
  });

  it("lists grant-scoped ticket metadata and strips the requester label and conversation", async () => {
    let seenGrantId: string | null = null;
    let seenStatus: string | null = null;
    server.use(
      http.get("*/v1/support/tickets", ({ request }) => {
        const url = new URL(request.url);
        seenGrantId = url.searchParams.get("grantId");
        seenStatus = url.searchParams.get("status");
        return jsonResponse({
          items: [
            {
              ...ticketMetadata,
              requesterLabel: "Customer A",
              conversation: [{ body: "private" }],
            },
          ],
        });
      }),
    );

    const result = await listTickets("grant-1", "open", TOKEN);

    expect(seenGrantId).toBe("grant-1");
    expect(seenStatus).toBe("open");
    expect(result.items[0]).toEqual(ticketMetadata);
    expect(result.items[0].requesterRef).toBe("userref-1");
    expect(JSON.stringify(result)).not.toMatch(/conversation|private|Customer A|requesterLabel/i);
  });

  it("declares and updates an incident on the support routes", async () => {
    const seen: string[] = [];
    server.use(
      http.post("*/v1/support/incidents", async ({ request }) => {
        seen.push(`POST ${JSON.stringify(await request.json())}`);
        return jsonResponse(
          {
            id: "incident-2",
            severity: "sev1",
            status: "declared",
            summary: "Payments are failing",
            components: ["portfolio"],
            declaredAt: "2026-11-08T10:00:00.000Z",
            updatedAt: "2026-11-08T10:00:00.000Z",
            notification: { state: "sent" },
          },
          201,
        );
      }),
      http.patch("*/v1/support/incidents/*", async ({ request }) => {
        seen.push(`PATCH ${JSON.stringify(await request.json())}`);
        return jsonResponse({
          id: "incident-2",
          severity: "sev1",
          status: "investigating",
          summary: "Payments are failing",
          components: ["portfolio"],
          declaredAt: "2026-11-08T10:00:00.000Z",
          updatedAt: "2026-11-08T10:05:00.000Z",
          notification: { state: "sent" },
        });
      }),
    );

    const declared = await declareIncident(
      { severity: "sev1", summary: "Payments are failing", components: ["portfolio"] },
      TOKEN,
    );
    expect(declared.notification?.state).toBe("sent");

    const updated = await updateIncident("incident-2", { status: "investigating" }, TOKEN);
    expect(updated.status).toBe("investigating");

    expect(seen[0]).toContain('"severity":"sev1"');
    expect(seen[1]).toContain('"status":"investigating"');
  });
});

describe("problem mapping", () => {
  it("throws ApiError carrying the RFC 7807 problem and status", async () => {
    server.use(
      http.get("*/v1/support-grants/mine", () => problemResponse(403, "Access ended", "corr-403")),
    );

    await expect(listMyGrants(TOKEN)).rejects.toMatchObject({
      name: "ApiError",
      problem: { status: 403, title: "Access ended", correlationId: "corr-403" },
    });
  });

  it("maps a non-problem error response to a fallback problem with the request correlation id", async () => {
    let correlationId: string | null = null;
    server.use(
      http.get("*/v1/support-grants/mine", ({ request }) => {
        correlationId = request.headers.get("x-correlation-id");
        return jsonResponse({ unexpected: true }, 401);
      }),
    );

    const error = await listMyGrants(TOKEN).catch((caught: unknown) => caught);

    expect(error).toBeInstanceOf(ApiError);
    const problem = (error as ApiError).problem;
    expect(problem.status).toBe(401);
    expect(problem.title).toBe("Gateway returned 401");
    expect(problem.correlationId).toBe(correlationId);
  });

  it("throws NetworkError when the gateway is unreachable", async () => {
    const failingFetch = vi.fn().mockRejectedValue(new TypeError("Failed to fetch"));

    await expect(
      listMyGrants(TOKEN, failingFetch as unknown as typeof fetch),
    ).rejects.toBeInstanceOf(NetworkError);
  });
});

describe("audit boundary", () => {
  it("exposes no audit write path", () => {
    const exported = Object.keys(client);
    expect(exported.some((name) => /audit/i.test(name))).toBe(false);
  });
});
