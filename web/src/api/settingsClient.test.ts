import { afterEach, describe, expect, it, vi } from "vitest";
import { ApiError, GatewayClient, NO_RECORD_PROBLEM_TYPE } from "./client";
import type { DeletionView, ProfileView } from "./types";

const profile: ProfileView = {
  userId: "user-1",
  displayName: "Mai Anh",
  displayNameOverridden: true,
  providerDisplayName: "Mai",
  email: "mai@example.com",
  emailVerified: true,
  status: "active",
};

const deletion: DeletionView = {
  deletionId: "deletion-1",
  status: "requested",
  requestedAt: "2026-11-01T09:00:00Z",
  recoveryEndsAt: "2026-12-01T09:00:00Z",
  retainedCategories: [],
  cancelledAt: null,
  erasedAt: null,
  attemptCount: 0,
};

function jsonResponse(body: unknown, status = 200): Response {
  return {
    ok: status >= 200 && status < 300,
    status,
    json: async () => body,
  } as Response;
}

function problemResponse(status: number, type = `https://errors.cpb/identity/${status}`): Response {
  return jsonResponse(
    {
      type,
      title: `Gateway ${status}`,
      status,
      detail: "server detail that must never be rendered",
      correlationId: `corr-${status}`,
    },
    status,
  );
}

function client(fetchMock: ReturnType<typeof vi.fn>, token: string | null = "token-1") {
  return new GatewayClient({
    baseUrl: "/v1",
    getToken: () => token,
    fetchImpl: fetchMock as unknown as typeof fetch,
  });
}

describe("settings client", () => {
  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it("reads the caller's account status from D0", async () => {
    // D0 answers `{ status }` and nothing else (functional-spec §8): the status
    // is the whole contract, and no identifier is needed to route on it.
    const fetchMock = vi.fn().mockResolvedValue(jsonResponse({ status: "pending_deletion" }));

    const result = await client(fetchMock).getAccountStatus();

    expect(result).toEqual({ status: "pending_deletion" });
    const [url, init] = fetchMock.mock.calls[0] as [string, RequestInit];
    expect(url).toBe("/v1/me/account-status");
    expect(init.method ?? "GET").toBe("GET");
    expect((init.headers as Record<string, string>).Authorization).toBe("Bearer token-1");
  });

  it("maps a 401 account-status answer to an ApiError with the status only", async () => {
    const fetchMock = vi.fn().mockResolvedValue(problemResponse(401));

    await expect(client(fetchMock, null).getAccountStatus()).rejects.toMatchObject({
      problem: { status: 401, correlationId: "corr-401" },
    });
  });

  it("reads the profile view and its exact field set", async () => {
    const fetchMock = vi.fn().mockResolvedValue(jsonResponse(profile));

    const result = await client(fetchMock).getProfileView();

    expect(result).toEqual(profile);
    expect((fetchMock.mock.calls[0] as [string, RequestInit])[0]).toBe("/v1/me/profile");
  });

  it("saves a display name with the exact PATCH body", async () => {
    const fetchMock = vi.fn().mockResolvedValue(jsonResponse(profile));

    await client(fetchMock).saveDisplayName("Mai Anh");

    const [url, init] = fetchMock.mock.calls[0] as [string, RequestInit];
    expect(url).toBe("/v1/me/profile");
    expect(init.method).toBe("PATCH");
    expect(JSON.parse(String(init.body))).toEqual({ displayName: "Mai Anh" });
  });

  it("adopts the provider name with useProviderName alone", async () => {
    const fetchMock = vi.fn().mockResolvedValue(jsonResponse(profile));

    await client(fetchMock).useProviderName();

    const [, init] = fetchMock.mock.calls[0] as [string, RequestInit];
    expect(init.method).toBe("PATCH");
    expect(JSON.parse(String(init.body))).toEqual({ useProviderName: true });
  });

  it("reads an open deletion request, and answers null only for the pinned no-record type", async () => {
    const open = vi.fn().mockResolvedValue(jsonResponse(deletion));
    await expect(client(open).getOpenDeletion()).resolves.toEqual(deletion);
    expect((open.mock.calls[0] as [string, RequestInit])[0]).toBe("/v1/me/deletion-request");

    const empty = vi.fn().mockResolvedValue(problemResponse(404, NO_RECORD_PROBLEM_TYPE));
    await expect(client(empty).getOpenDeletion()).resolves.toBeNull();

    // A bare 404 that is not the designed sentinel is an error, never "no request".
    const foreign = vi.fn().mockResolvedValue(problemResponse(404));
    await expect(client(foreign).getOpenDeletion()).rejects.toBeInstanceOf(ApiError);
  });

  it("cancels the open request and maps 410 as a terminal ApiError", async () => {
    const fetchMock = vi.fn().mockResolvedValue(jsonResponse({ ...deletion, status: "cancelled" }));
    await expect(client(fetchMock).cancelDeletion()).resolves.toMatchObject({
      status: "cancelled",
    });
    const [url, init] = fetchMock.mock.calls[0] as [string, RequestInit];
    expect(url).toBe("/v1/me/deletion-request");
    expect(init.method).toBe("DELETE");

    const gone = vi.fn().mockResolvedValue(problemResponse(410));
    await expect(client(gone).cancelDeletion()).rejects.toMatchObject({
      problem: { status: 410 },
    });
  });

  it("requests deletion at D3 with no body and maps 409 as already open", async () => {
    const fetchMock = vi.fn().mockResolvedValue(jsonResponse(deletion, 202));

    await expect(client(fetchMock).requestDeletion()).resolves.toEqual(deletion);
    const [url, init] = fetchMock.mock.calls[0] as [string, RequestInit];
    expect(url).toBe("/v1/privacy/deletions");
    expect(init.method).toBe("POST");
    expect(init.body).toBeUndefined();

    const conflict = vi.fn().mockResolvedValue(problemResponse(409));
    await expect(client(conflict).requestDeletion()).rejects.toMatchObject({
      problem: { status: 409 },
    });
  });
});
