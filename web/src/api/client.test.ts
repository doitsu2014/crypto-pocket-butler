import { afterEach, describe, expect, it, vi } from "vitest";
import { NetworkError, resolveAsset } from "./client";

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

describe("gateway client", () => {
  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it("sends the bearer token and a correlation id to the gateway", async () => {
    const fetchMock = vi.fn().mockResolvedValue({
      ok: true,
      status: 200,
      json: async () => asset,
    } as Response);

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
    const fetchMock = vi.fn().mockResolvedValue({
      ok: false,
      status: 404,
      json: async () => problem,
    } as Response);

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
});
