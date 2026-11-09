import { http, HttpResponse } from "msw";
import { afterEach, describe, expect, it, vi } from "vitest";
import * as client from "./client";
import {
  ApiError,
  approveSupportGrant,
  declareIncident,
  fetchAccountMetadata,
  getSupportGrant,
  GrantScopeError,
  listTickets,
  NetworkError,
  requestSupportGrant,
  revokeSupportGrant,
  searchAccounts,
  updateIncident,
} from "./client";
import { activeGrant, accountMetadata, problemDetails, ticketMetadata } from "../test/fixtures";
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

afterEach(() => {
  vi.unstubAllGlobals();
});

describe("grant calls", () => {
  it("creates a grant with a bearer token, a correlation id, and the request body", async () => {
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
        durationMinutes: 60,
        approverId: "agent-2",
      },
      TOKEN,
    );

    expect(created).toEqual(grant);
    expect(seenBody).toEqual({
      userId: "user-1",
      reason: "Investigating a reported sync failure",
      durationMinutes: 60,
      approverId: "agent-2",
    });
    expect(seenAuthorization).toBe(`Bearer ${TOKEN}`);
    expect(seenCorrelation).toEqual(expect.any(String));
  });

  it("reads, approves, and revokes a grant on the C2 routes", async () => {
    const seen: string[] = [];
    const grant = activeGrant();
    server.use(
      http.get("*/v1/support-grants/*", ({ request }) => {
        seen.push(`GET ${new URL(request.url).pathname}`);
        return jsonResponse(grant);
      }),
      http.post("*/v1/support-grants/*/approve", ({ request }) => {
        seen.push(`POST ${new URL(request.url).pathname}`);
        return jsonResponse(grant);
      }),
      http.delete("*/v1/support-grants/*", ({ request }) => {
        seen.push(`DELETE ${new URL(request.url).pathname}`);
        return new HttpResponse(null, { status: 204 });
      }),
    );

    expect(await getSupportGrant("grant-1", TOKEN)).toEqual(grant);
    expect(await approveSupportGrant("grant-1", TOKEN)).toEqual(grant);
    await expect(revokeSupportGrant("grant-1", TOKEN)).resolves.toBeUndefined();

    expect(seen).toEqual([
      "GET /v1/support-grants/grant-1",
      "POST /v1/support-grants/grant-1/approve",
      "DELETE /v1/support-grants/grant-1",
    ]);
  });
});

describe("grant-scoped account search", () => {
  it("refuses to search without a grant id and makes no request", async () => {
    const handler = vi.fn();
    server.use(
      http.get("*/v1/support/accounts", () => {
        handler();
        return jsonResponse({ items: [], nextCursor: null });
      }),
    );

    await expect(searchAccounts({ grantId: "   ", query: "acct" }, TOKEN)).rejects.toBeInstanceOf(
      GrantScopeError,
    );
    expect(handler).not.toHaveBeenCalled();
  });

  it("sends the grant id and query, and returns metadata only", async () => {
    let seenGrantId: string | null = null;
    let seenQuery: string | null = null;
    server.use(
      http.get("*/v1/support/accounts", ({ request }) => {
        const url = new URL(request.url);
        seenGrantId = url.searchParams.get("grantId");
        seenQuery = url.searchParams.get("query");
        return jsonResponse({
          items: [
            {
              ...accountMetadata,
              credentials: { apiKey: "never-render" },
              holdings: [{ asset: "BTC", quantity: "1" }],
            },
          ],
          nextCursor: "cursor-2",
        });
      }),
    );

    const result = await searchAccounts({ grantId: "grant-1", query: " binance " }, TOKEN);

    expect(seenGrantId).toBe("grant-1");
    expect(seenQuery).toBe("binance");
    expect(result.items).toHaveLength(1);
    expect(result.items[0]).toEqual(accountMetadata);
    expect(JSON.stringify(result)).not.toMatch(/credential|holding|never-render/i);
  });

  it("sends the reason in X-Support-Reason, never in the URL", async () => {
    let seenGrantId: string | null = null;
    let seenReason: string | null = null;
    let seenUrl = "";
    server.use(
      http.get("*/v1/support/accounts/*/metadata", ({ request }) => {
        const url = new URL(request.url);
        seenGrantId = url.searchParams.get("grantId");
        seenReason = request.headers.get("X-Support-Reason");
        seenUrl = request.url;
        return jsonResponse(accountMetadata);
      }),
    );

    await expect(
      fetchAccountMetadata("acct-1", { grantId: "", reason: "reason long enough" }, TOKEN),
    ).rejects.toBeInstanceOf(GrantScopeError);

    const metadata = await fetchAccountMetadata(
      "acct-1",
      { grantId: "grant-1", reason: "Investigating a sync failure" },
      TOKEN,
    );
    expect(metadata).toEqual(accountMetadata);
    expect(seenGrantId).toBe("grant-1");
    expect(seenReason).toBe("Investigating a sync failure");
    expect(seenUrl).not.toContain("reason=");
    expect(seenUrl).not.toContain("Investigating");
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
      http.get("*/v1/support/tickets", () =>
        problemResponse(403, "Access ended", "corr-403"),
      ),
    );

    await expect(listTickets("grant-1", undefined, TOKEN)).rejects.toMatchObject({
      name: "ApiError",
      problem: { status: 403, title: "Access ended", correlationId: "corr-403" },
    });
  });

  it("maps a non-problem error response to a fallback problem with the request correlation id", async () => {
    let correlationId: string | null = null;
    server.use(
      http.get("*/v1/support/tickets", ({ request }) => {
        correlationId = request.headers.get("x-correlation-id");
        return jsonResponse({ unexpected: true }, 401);
      }),
    );

    const error = await listTickets("grant-1", undefined, TOKEN).catch((caught: unknown) => caught);

    expect(error).toBeInstanceOf(ApiError);
    const problem = (error as ApiError).problem;
    expect(problem.status).toBe(401);
    expect(problem.title).toBe("Gateway returned 401");
    expect(problem.correlationId).toBe(correlationId);
  });

  it("throws NetworkError when the gateway is unreachable", async () => {
    const failingFetch = vi.fn().mockRejectedValue(new TypeError("Failed to fetch"));

    await expect(
      listTickets("grant-1", undefined, TOKEN, failingFetch as unknown as typeof fetch),
    ).rejects.toBeInstanceOf(NetworkError);
  });
});

describe("audit boundary", () => {
  it("exposes no audit write path", () => {
    const exported = Object.keys(client);
    expect(exported.some((name) => /audit/i.test(name))).toBe(false);
  });
});
