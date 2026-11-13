import { http, HttpResponse } from "msw";
import { afterEach, describe, expect, it, vi } from "vitest";
import {
  ApiError,
  DeadlineError,
  changeUserRole,
  disableUser,
  listPendingGrants,
  lookupAdminUser,
  restoreUser,
} from "./client";
import type { AdminUserRecord, SupportAccessGrant } from "./types";
import { pendingGrant, problemDetails } from "../test/fixtures";
import { server } from "../test/server";

const TOKEN = "admin-token";

const ADMIN_USER: AdminUserRecord = {
  id: "user-1",
  displayName: "Test User",
  email: "user@example.com",
  emailVerified: true,
  status: "active",
  role: "support",
  createdAt: "2026-11-01T10:00:00.000Z",
  updatedAt: "2026-11-02T10:00:00.000Z",
};

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

describe("the five admin calls", () => {
  it("looks a user up with a body-carrying POST, never a URL", async () => {
    let seenMethod: string | null = null;
    let seenUrl = "";
    let seenBody: unknown;
    let seenAuthorization: string | null = null;
    server.use(
      http.post("*/v1/admin/users/lookup", async ({ request }) => {
        seenMethod = request.method;
        seenUrl = request.url;
        seenBody = await request.json();
        seenAuthorization = request.headers.get("Authorization");
        return jsonResponse(ADMIN_USER);
      }),
    );

    const record = await lookupAdminUser(TOKEN, "user@example.com");

    expect(record).toEqual(ADMIN_USER);
    expect(seenMethod).toBe("POST");
    expect(new URL(seenUrl).pathname).toBe("/v1/admin/users/lookup");
    expect(seenUrl).not.toContain("user@example.com");
    expect(seenBody).toEqual({ email: "user@example.com" });
    expect(seenAuthorization).toBe(`Bearer ${TOKEN}`);
  });

  it("changes a role with PUT and keeps the reason out of the URL", async () => {
    let seenMethod: string | null = null;
    let seenUrl = "";
    let seenBody: unknown;
    server.use(
      http.put("*/v1/admin/users/user-1/role", async ({ request }) => {
        seenMethod = request.method;
        seenUrl = request.url;
        seenBody = await request.json();
        return jsonResponse({ userId: "user-1", previousRole: "admin", newRole: "support" });
      }),
    );

    const result = await changeUserRole(TOKEN, "user-1", {
      newRole: "support",
      reason: "Moved to the support team",
    });

    expect(result).toEqual({ userId: "user-1", previousRole: "admin", newRole: "support" });
    expect(seenMethod).toBe("PUT");
    expect(new URL(seenUrl).pathname).toBe("/v1/admin/users/user-1/role");
    expect(seenUrl).not.toContain("reason=");
    expect(seenUrl).not.toContain("Moved");
    expect(seenBody).toEqual({ newRole: "support", reason: "Moved to the support team" });
  });

  it("disables and restores through their POST paths with a body reason", async () => {
    const seen: string[] = [];
    let seenBody: unknown;
    server.use(
      http.post("*/v1/admin/users/user-1/disable", async ({ request }) => {
        seen.push(`POST ${new URL(request.url).pathname}`);
        seenBody = await request.json();
        return jsonResponse({
          userId: "user-1",
          status: "disabled",
          transitionId: "transition-1",
          occurredAt: "2026-11-02T10:05:00.000Z",
        });
      }),
      http.post("*/v1/admin/users/user-1/restore", async ({ request }) => {
        seen.push(`POST ${new URL(request.url).pathname}`);
        return jsonResponse({
          userId: "user-1",
          status: "active",
          transitionId: "transition-2",
          occurredAt: "2026-11-02T10:10:00.000Z",
        });
      }),
    );

    const disabled = await disableUser(TOKEN, "user-1", { reason: "Account abuse" });
    expect(disabled.status).toBe("disabled");
    expect(seenBody).toEqual({ reason: "Account abuse" });

    const restored = await restoreUser(TOKEN, "user-1", { reason: "Appeal upheld" });
    expect(restored.status).toBe("active");
    expect(seen).toEqual([
      "POST /v1/admin/users/user-1/disable",
      "POST /v1/admin/users/user-1/restore",
    ]);
  });

  it("lists the pending grants with the pending_approval filter", async () => {
    const grant = pendingGrant();
    let seenStatus: string | null = null;
    let seenUrl = "";
    server.use(
      http.get("*/v1/support-grants", ({ request }) => {
        seenUrl = request.url;
        seenStatus = new URL(request.url).searchParams.get("status");
        return jsonResponse([grant]);
      }),
    );

    const grants: SupportAccessGrant[] = await listPendingGrants(TOKEN);

    expect(seenStatus).toBe("pending_approval");
    expect(seenUrl).toContain("status=pending_approval");
    expect(grants).toEqual([grant]);
  });
});

describe("problem mapping on the admin calls", () => {
  it.each([
    [403, "Not an administrator", "corr-403"],
    [503, "Service unavailable", "corr-503"],
  ])("surfaces a %i as ApiError with its correlation id", async (status, title, id) => {
    server.use(http.post("*/v1/admin/users/lookup", () => problemResponse(status, title, id)));

    const error = await lookupAdminUser(TOKEN, "user@example.com").catch(
      (caught: unknown) => caught,
    );

    expect(error).toBeInstanceOf(ApiError);
    expect((error as ApiError).status).toBe(status);
    expect((error as ApiError).problem.correlationId).toBe(id);
  });
});

describe("the 15 s deadline", () => {
  it("aborts with DeadlineError at fifteen seconds", async () => {
    vi.useFakeTimers();
    const fetchImpl = abortAwareFetch();

    const pending = listPendingGrants(TOKEN, fetchImpl);
    const assertion = expect(pending).rejects.toBeInstanceOf(DeadlineError);
    await vi.advanceTimersByTimeAsync(15_000);
    await assertion;
    expect(fetchImpl).toHaveBeenCalledTimes(1);
  });

  it("clears the timer on a settled call so it never aborts a later one", async () => {
    vi.useFakeTimers();
    let settledSignal: AbortSignal | null = null;
    const settledFetch = vi.fn(async (_input: unknown, init?: RequestInit) => {
      settledSignal = init?.signal ?? null;
      return new Response(JSON.stringify([pendingGrant()]), {
        status: 200,
        headers: { "Content-Type": "application/json" },
      });
    }) as unknown as typeof fetch;

    await listPendingGrants(TOKEN, settledFetch);
    await vi.advanceTimersByTimeAsync(30_000);

    expect(settledSignal).not.toBeNull();
    expect((settledSignal as AbortSignal | null)?.aborted).toBe(false);
  });
});

describe("the 429 retry-after surface", () => {
  it("parses integer seconds", async () => {
    server.use(
      http.get("*/v1/support-grants", () =>
        HttpResponse.json(problemDetails(429, "Too many requests"), {
          status: 429,
          headers: { "Content-Type": "application/problem+json", "Retry-After": "42" },
        }),
      ),
    );

    const error = await listPendingGrants(TOKEN).catch((caught: unknown) => caught);
    expect(error).toBeInstanceOf(ApiError);
    expect((error as ApiError).retryAfterSeconds).toBe(42);
  });

  it("parses an HTTP-date into whole seconds from now", async () => {
    vi.useFakeTimers();
    vi.setSystemTime(new Date("2026-11-12T10:00:00.000Z"));
    server.use(
      http.get("*/v1/support-grants", () =>
        HttpResponse.json(problemDetails(429, "Too many requests"), {
          status: 429,
          headers: {
            "Content-Type": "application/problem+json",
            "Retry-After": "Thu, 12 Nov 2026 10:00:30 GMT",
          },
        }),
      ),
    );

    const error = await listPendingGrants(TOKEN).catch((caught: unknown) => caught);
    expect(error).toBeInstanceOf(ApiError);
    expect((error as ApiError).retryAfterSeconds).toBe(30);
  });

  it("leaves retryAfterSeconds undefined when the header is unparseable", async () => {
    server.use(
      http.get("*/v1/support-grants", () =>
        HttpResponse.json(problemDetails(429, "Too many requests"), {
          status: 429,
          headers: {
            "Content-Type": "application/problem+json",
            "Retry-After": "soon please",
          },
        }),
      ),
    );

    const error = await listPendingGrants(TOKEN).catch((caught: unknown) => caught);
    expect(error).toBeInstanceOf(ApiError);
    expect((error as ApiError).retryAfterSeconds).toBeUndefined();
  });
});
