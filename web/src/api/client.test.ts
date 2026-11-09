import { afterEach, describe, expect, it, vi } from "vitest";
import {
  ApiError,
  GatewayClient,
  NetworkError,
  PAGE_SIZE,
  resolveAsset,
} from "./client";

const asset = {
  id: "asset-usdc",
  symbol: "USDC",
  name: "USD Coin",
  decimals: 6,
  canonical: true,
};

const problem = {
  type: "https://crypto-pocket-butler.example/problems/asset-unresolved",
  title: "Asset unresolved",
  status: 404,
  detail: "observation 'obs-1' is quarantined",
  correlationId: "corr-client-1",
};

function jsonResponse(body: unknown, status = 200): Response {
  return {
    ok: status >= 200 && status < 300,
    status,
    json: async () => body,
  } as Response;
}

function client(fetchMock: ReturnType<typeof vi.fn>, token: string | null = "token-1") {
  return new GatewayClient({
    baseUrl: "/v1",
    getToken: () => token,
    fetchImpl: fetchMock as unknown as typeof fetch,
  });
}

describe("gateway client", () => {
  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it("sends the bearer token and a correlation id to the gateway", async () => {
    const fetchMock = vi.fn().mockResolvedValue(jsonResponse(asset));

    const result = await resolveAsset(
      { chain: "ethereum", contractAddress: "0xabc", symbol: "USDC" },
      "token-1",
      fetchMock as unknown as typeof fetch,
    );

    expect(result).toEqual(asset);
    const [url, init] = fetchMock.mock.calls[0] as [string, RequestInit];
    expect(url).toContain("/assets/resolve?chain=ethereum");
    expect(url).toContain("contractAddress=0xabc");
    expect(url).toContain("symbol=USDC");
    expect((init.headers as Record<string, string>).Authorization).toBe("Bearer token-1");
    expect(typeof (init.headers as Record<string, string>)["x-correlation-id"]).toBe("string");
  });

  it("throws ApiError carrying the RFC 7807 problem", async () => {
    const fetchMock = vi.fn().mockResolvedValue(jsonResponse(problem, 404));

    await expect(
      resolveAsset(
        { chain: "ethereum", symbol: "SCAM" },
        null,
        fetchMock as unknown as typeof fetch,
      ),
    ).rejects.toMatchObject({
      name: "ApiError",
      problem: { status: 404, correlationId: "corr-client-1" },
    });
  });

  it("throws NetworkError when the gateway is unreachable", async () => {
    const fetchMock = vi.fn().mockRejectedValue(new TypeError("Failed to fetch"));

    await expect(
      resolveAsset(
        { chain: "ethereum", symbol: "ETH" },
        null,
        fetchMock as unknown as typeof fetch,
      ),
    ).rejects.toBeInstanceOf(NetworkError);
  });

  it("maps 401, 403, 409, 422 and 500 problems without losing the correlation id", async () => {
    for (const status of [401, 403, 409, 422, 500]) {
      const fetchMock = vi
        .fn()
        .mockResolvedValue(
          jsonResponse({ type: "about:blank", title: `status ${status}`, status, correlationId: `c-${status}` }, status),
        );
      await expect(client(fetchMock).listPortfolios()).rejects.toMatchObject({
        name: "ApiError",
        problem: { status, correlationId: `c-${status}` },
      });
    }
  });

  it("falls back to the request correlation id when the problem body is unreadable", async () => {
    const fetchMock = vi.fn().mockResolvedValue({
      ok: false,
      status: 500,
      json: async () => {
        throw new SyntaxError("not json");
      },
    } as unknown as Response);

    const error = await client(fetchMock).listPortfolios().catch((caught: unknown) => caught);
    expect(error).toBeInstanceOf(ApiError);
    expect((error as ApiError).problem.correlationId).toMatch(/[0-9a-f-]{8,}/);
  });

  it("requests at most one page of 50 rows and forwards the cursor", async () => {
    const fetchMock = vi.fn().mockResolvedValue(jsonResponse([]));

    await client(fetchMock).listTransactions("user-1", "cursor-2");

    const [url] = fetchMock.mock.calls[0] as [string];
    expect(url).toContain(`limit=${PAGE_SIZE}`);
    expect(url).toContain("cursor=cursor-2");
    expect(PAGE_SIZE).toBeLessThanOrEqual(50);
  });

  it("sends no bearer token on the public telemetry route", async () => {
    const fetchMock = vi.fn().mockResolvedValue(jsonResponse(undefined, 204));

    await client(fetchMock).postClientError({
      route: "/dashboard",
      errorCode: "bootstrap_failed",
      correlationId: "corr-1",
    });

    const [, init] = fetchMock.mock.calls[0] as [string, RequestInit];
    expect((init.headers as Record<string, string>).Authorization).toBeUndefined();
    expect(init.method).toBe("POST");
  });

  it("does not force a JSON content type on a multipart import", async () => {
    const fetchMock = vi.fn().mockResolvedValue(jsonResponse({ id: "import-1" }));
    const file = new File(["asset,quantity\nBTC,1\n"], "transactions.csv", {
      type: "text/csv",
    });

    await client(fetchMock).createImport(file);

    const [, init] = fetchMock.mock.calls[0] as [string, RequestInit];
    expect((init.headers as Record<string, string>)["Content-Type"]).toBeUndefined();
    expect(init.body).toBeInstanceOf(FormData);
  });

  it("returns undefined for a 204 response", async () => {
    const fetchMock = vi.fn().mockResolvedValue({
      ok: true,
      status: 204,
      json: async () => {
        throw new Error("no body");
      },
    } as unknown as Response);

    await expect(client(fetchMock).revokeSession("session-1")).resolves.toBeUndefined();
  });

  it("binds to the configured gateway base and refuses to build another origin", () => {
    expect(client(vi.fn()).base).toBe("/v1");
  });
});
