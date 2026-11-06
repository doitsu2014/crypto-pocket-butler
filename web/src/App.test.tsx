import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import App from "./App";
import { setAccessToken } from "./auth/session";

const asset = {
  id: "asset-usdc",
  symbol: "USDC",
  name: "USD Coin",
  decimals: 6,
  canonical: true,
};

function renderApp() {
  const client = new QueryClient({
    defaultOptions: { queries: { retry: false }, mutations: { retry: false } },
  });
  return render(
    <QueryClientProvider client={client}>
      <App />
    </QueryClientProvider>,
  );
}

async function submitAddress(address: string) {
  const user = userEvent.setup();
  await user.clear(screen.getByTestId("asset-resolve-address"));
  await user.type(screen.getByTestId("asset-resolve-address"), address);
  await user.click(screen.getByTestId("asset-resolve-submit"));
}

describe("asset resolver shell", () => {
  const fetchMock = vi.fn();

  beforeEach(() => {
    fetchMock.mockReset();
    vi.stubGlobal("fetch", fetchMock);
    setAccessToken("test-token");
  });

  afterEach(() => {
    vi.unstubAllGlobals();
    setAccessToken(null);
  });

  it("shows a resolved asset returned by the gateway", async () => {
    fetchMock.mockResolvedValueOnce({
      ok: true,
      status: 200,
      json: async () => asset,
    } as Response);

    renderApp();
    await submitAddress("0xabc");

    const result = await screen.findByTestId("asset-result");
    expect(result).toHaveTextContent("USDC");
    expect(result).toHaveTextContent("6");
    const [, init] = fetchMock.mock.calls[0] as [string, RequestInit];
    expect((init.headers as Record<string, string>).Authorization).toBe("Bearer test-token");
  });

  it("shows a network error and keeps the entered address", async () => {
    fetchMock.mockRejectedValueOnce(new TypeError("Failed to fetch"));

    renderApp();
    await submitAddress("0xdeadbeef");

    expect(await screen.findByTestId("asset-error")).toHaveTextContent(/gateway/i);
    expect((screen.getByTestId("asset-resolve-address") as HTMLInputElement).value).toBe(
      "0xdeadbeef",
    );
  });

  it("shows an unresolved asset with the correlation id", async () => {
    fetchMock.mockResolvedValueOnce({
      ok: false,
      status: 404,
      json: async () => ({
        type: "https://crypto-pocket-butler.example/problems/asset-unresolved",
        title: "Asset unresolved",
        status: 404,
        detail: "observation 'obs-1' is quarantined",
        correlationId: "corr-42",
      }),
    } as Response);

    renderApp();
    await submitAddress("0xdeadbeef");

    const unresolved = await screen.findByTestId("asset-unresolved");
    expect(unresolved).toHaveTextContent(/unresolved/i);
    expect(unresolved).toHaveTextContent("corr-42");
  });

  it("keeps the session token in memory only", async () => {
    fetchMock.mockResolvedValueOnce({
      ok: true,
      status: 200,
      json: async () => asset,
    } as Response);

    renderApp();
    await submitAddress("0xabc");
    await screen.findByTestId("asset-result");

    expect(window.localStorage.length).toBe(0);
    expect(window.sessionStorage.length).toBe(0);
  });
});
