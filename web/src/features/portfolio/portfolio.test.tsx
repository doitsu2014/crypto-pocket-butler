import { screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it, vi } from "vitest";
import DashboardPage from "./DashboardPage";
import HistoryPage from "./HistoryPage";
import IntentsPage from "./IntentsPage";
import PortfolioDetailPage from "./PortfolioDetailPage";
import PortfoliosPage from "./PortfoliosPage";
import TargetsPage from "./TargetsPage";
import TradesPage from "./TradesPage";
import { ApiError } from "../../api/client";
import { renderWithProviders } from "../../test/render";
import { problem, stubGateway } from "../../test/gateway";
import type { OrderIntent, PortfolioValuation } from "../../api/types";

const valuation: PortfolioValuation = {
  portfolioId: "p1",
  asOf: "2026-11-08T12:00:00Z",
  status: "partial",
  totalValue: "1000.00",
  currency: "USD",
  pricedCoverage: 0.9,
  unpricedCount: 1,
  excludedCount: 1,
  lines: [
    {
      assetId: "btc",
      symbol: "BTC",
      quantity: "0.5",
      value: "900.00",
      allocation: 90,
      targetPercent: 60,
      driftPoints: 30,
      price: "1800.00",
      priceSource: "Binance",
      priceFreshness: "live",
      priceObservedAt: "2026-11-08T11:55:00Z",
      fxRate: null,
      fxSource: null,
      fxObservedAt: null,
      dataStatus: "priced",
    },
    {
      assetId: "doge",
      symbol: "DOGE",
      quantity: "1000",
      value: null,
      allocation: null,
      targetPercent: 40,
      driftPoints: null,
      price: null,
      priceSource: null,
      priceFreshness: "unpriced",
      priceObservedAt: null,
      fxRate: null,
      fxSource: null,
      fxObservedAt: null,
      dataStatus: "unpriced",
    },
  ],
};

const intent: OrderIntent = {
  id: "intent-1",
  assetId: "btc",
  side: "buy",
  quantity: "0.1",
  projectedAllocation: 55,
  projectedDriftPoints: 5,
  shortfall: "0.5",
  status: "pending",
  cancelledAt: null,
};

afterEach(() => {
  vi.unstubAllGlobals();
});

describe("dashboard", () => {
  it("renders a portfolio total and keeps unpriced lines labelled", async () => {
    stubGateway({
      "GET /v1/portfolios": () => ({
        body: [{ id: "p1", name: "Main", baseCurrency: "USD" }],
      }),
      "GET /v1/portfolios/p1": () => ({ body: valuation }),
    });

    renderWithProviders(<DashboardPage />, { route: "/dashboard" });

    expect(await screen.findByText("Main")).toBeInTheDocument();
    expect(await screen.findByTestId("money")).toHaveTextContent("1000.00");
    expect(screen.getByText("DOGE")).toBeInTheDocument();
    expect(
      screen.getAllByTestId("data-status").some((node) => node.textContent?.includes("Unpriced")),
    ).toBe(true);
  });

  it("shows the create-first-portfolio empty state", async () => {
    stubGateway({ "GET /v1/portfolios": () => ({ body: [] }) });

    renderWithProviders(<DashboardPage />, { route: "/dashboard" });

    expect(await screen.findByTestId("empty-state")).toHaveTextContent(/first portfolio/i);
  });

  it("degrades only the failing portfolio panel when one source is down", async () => {
    stubGateway({
      "GET /v1/portfolios": () => ({
        body: [
          { id: "p1", name: "Main", baseCurrency: "USD" },
          { id: "p2", name: "Speculative", baseCurrency: "USD" },
        ],
      }),
      "GET /v1/portfolios/p1": () => ({ body: valuation }),
      "GET /v1/portfolios/p2": () => ({ status: 500, body: problem(500, "outage") }),
    });

    renderWithProviders(<DashboardPage />, { route: "/dashboard" });

    expect(await screen.findByText("Main")).toBeInTheDocument();
    expect(await screen.findByTestId("error-panel")).toHaveTextContent(
      /could not complete|outage/i,
    );
    expect(screen.getByTestId("money")).toHaveTextContent("1000.00");
  });
});

describe("portfolio list", () => {
  it("creates the first portfolio and opens it", async () => {
    const fetchMock = stubGateway({
      "GET /v1/portfolios": () => ({ body: [] }),
      "POST /v1/portfolios": () => ({
        status: 201,
        body: { id: "p9", name: "Long term", baseCurrency: "USD" },
      }),
    });
    const user = userEvent.setup();
    renderWithProviders(<PortfoliosPage />, { route: "/portfolios" });

    await screen.findByTestId("portfolio-create");
    await user.type(screen.getByTestId("portfolio-name"), "Long term");
    await user.click(screen.getByTestId("portfolio-create-submit"));

    await waitFor(() =>
      expect(
        fetchMock.mock.calls.some(
          ([url, init]) =>
            String(url).endsWith("/v1/portfolios") && (init as RequestInit).method === "POST",
        ),
      ).toBe(true),
    );
  });

  it("renames an existing portfolio", async () => {
    const fetchMock = stubGateway({
      "GET /v1/portfolios": () => ({
        body: [{ id: "p1", name: "Main", baseCurrency: "USD" }],
      }),
      "PATCH /v1/portfolios/p1": () => ({
        body: { id: "p1", name: "Core", baseCurrency: "USD" },
      }),
    });
    const user = userEvent.setup();
    renderWithProviders(<PortfoliosPage />, { route: "/portfolios" });

    await screen.findByText("Main");
    await user.click(screen.getByTestId("portfolio-rename"));
    const input = screen.getByTestId("portfolio-rename-input");
    await user.clear(input);
    await user.type(input, "Core");
    await user.click(screen.getByTestId("portfolio-rename-save"));

    await waitFor(() =>
      expect(
        fetchMock.mock.calls.some(
          ([, init]) => (init as RequestInit | undefined)?.method === "PATCH",
        ),
      ).toBe(true),
    );
  });
});

describe("portfolio detail", () => {
  it("renders the breakdown, keeps unpriced rows and moves an account", async () => {
    const fetchMock = stubGateway({
      "GET /v1/portfolios/p1": () => ({ body: valuation }),
      "GET /v1/portfolios": () => ({
        body: [
          { id: "p1", name: "Main", baseCurrency: "USD" },
          { id: "p2", name: "Other", baseCurrency: "USD" },
        ],
      }),
      "GET /v1/users/user-1/accounts": () => ({
        body: [
          {
            id: "acc-1",
            kind: "exchange",
            label: "Binance main",
            status: "synced",
            lastSyncedAt: "2026-11-08T10:00:00Z",
          },
        ],
      }),
      "PUT /v1/portfolios/p2/accounts/acc-1": () => ({ status: 204 }),
    });
    const user = userEvent.setup();
    renderWithProviders(<PortfolioDetailPage portfolioId="p1" userId="user-1" />, {
      route: "/portfolios/p1",
    });

    expect(await screen.findByText("DOGE")).toBeInTheDocument();
    expect(screen.getByTestId("data-table")).toHaveTextContent("Unpriced");

    await user.selectOptions(await screen.findByTestId("move-acc-1"), "p2");

    await waitFor(() =>
      expect(
        fetchMock.mock.calls.some(([url]) =>
          String(url).includes("/v1/portfolios/p2/accounts/acc-1"),
        ),
      ).toBe(true),
    );
  });

  it("shows a source error for the whole detail panel", async () => {
    stubGateway({
      "GET /v1/portfolios/p1": () => ({ status: 500, body: problem(500, "outage") }),
      "GET /v1/portfolios": () => ({ body: [] }),
      "GET /v1/users/user-1/accounts": () => ({ body: [] }),
    });
    renderWithProviders(<PortfolioDetailPage portfolioId="p1" userId="user-1" />, {
      route: "/portfolios/p1",
    });
    expect(await screen.findByTestId("error-panel")).toBeInTheDocument();
  });
});

describe("targets", () => {
  it("shows the difference from 100 and disables save until balanced", async () => {
    const save = vi.fn().mockResolvedValue(undefined);
    const user = userEvent.setup();
    renderWithProviders(
      <TargetsPage portfolioId="p1" valuationOverride={valuation} save={save} />,
      { route: "/portfolios/p1/targets" },
    );

    expect(screen.getByTestId("target-difference")).toHaveTextContent("0");
    expect(screen.getByTestId("targets-save")).toBeEnabled();

    const btc = screen.getByTestId("target-btc");
    await user.clear(btc);
    await user.type(btc, "50");

    expect(screen.getByTestId("target-difference")).toHaveTextContent("-10");
    expect(screen.getByTestId("targets-save")).toBeDisabled();
  });

  it("previews the sum and names the offending asset on a 422", async () => {
    const preview = vi
      .fn()
      .mockResolvedValueOnce({ sum: 100, difference: 0 })
      .mockRejectedValueOnce(
        new ApiError({
          type: "about:blank",
          title: "Validation failed",
          status: 422,
          correlationId: "corr-422",
          errors: [{ field: "assetId", reason: "unknown asset" }],
        }),
      );
    const user = userEvent.setup();
    renderWithProviders(
      <TargetsPage portfolioId="p1" valuationOverride={valuation} preview={preview} />,
      { route: "/portfolios/p1/targets" },
    );

    await user.click(screen.getByTestId("targets-preview"));
    expect(await screen.findByTestId("targets-preview-result")).toHaveTextContent("100");

    await user.click(screen.getByTestId("targets-preview"));
    expect(await screen.findByTestId("targets-errors")).toHaveTextContent("assetId: unknown asset");
  });

  it("prompts when no targets are set", () => {
    const noTargets = {
      ...valuation,
      lines: valuation.lines.map((line) => ({ ...line, targetPercent: null })),
    };
    renderWithProviders(<TargetsPage portfolioId="p1" valuationOverride={noTargets} />, {
      route: "/portfolios/p1/targets",
    });
    expect(screen.getByTestId("no-targets")).toBeInTheDocument();
  });
});

describe("order intents", () => {
  it("previews projected allocation and shortfall, then creates and cancels", async () => {
    const preview = vi.fn().mockResolvedValue(intent);
    const create = vi.fn().mockResolvedValue(intent);
    const cancel = vi.fn().mockResolvedValue({
      ...intent,
      status: "cancelled",
      cancelledAt: "2026-11-08T13:00:00Z",
    });
    const user = userEvent.setup();
    renderWithProviders(
      <IntentsPage
        portfolioId="p1"
        valuationOverride={valuation}
        preview={preview}
        create={create}
        cancel={cancel}
      />,
      { route: "/portfolios/p1/intents" },
    );

    await user.type(screen.getByTestId("intent-quantity"), "0.1");
    await user.click(screen.getByTestId("intent-preview"));

    expect(await screen.findByTestId("intent-preview-result")).toHaveTextContent("55%");
    expect(screen.getByTestId("intent-shortfall")).toHaveTextContent("0.5");

    await user.click(screen.getByTestId("intent-create"));
    expect(await screen.findByTestId("intent-row")).toHaveTextContent("buy 0.1");

    await user.click(screen.getByTestId("intent-cancel-intent-1"));
    await waitFor(() => expect(cancel).toHaveBeenCalledWith("p1", "intent-1"));
  });

  it("surfaces a 409 conflict without creating an intent", async () => {
    const create = vi.fn().mockRejectedValue(
      new ApiError({
        type: "about:blank",
        title: "Conflict",
        status: 409,
        detail: "holding changed",
        correlationId: "corr-409",
      }),
    );
    const user = userEvent.setup();
    renderWithProviders(
      <IntentsPage
        portfolioId="p1"
        valuationOverride={valuation}
        preview={vi.fn().mockResolvedValue(intent)}
        create={create}
      />,
      { route: "/portfolios/p1/intents" },
    );

    await user.type(screen.getByTestId("intent-quantity"), "0.1");
    await user.click(screen.getByTestId("intent-preview"));
    await screen.findByTestId("intent-preview-result");
    await user.click(screen.getByTestId("intent-create"));

    expect(await screen.findByTestId("intent-errors")).toHaveTextContent("holding changed");
    await waitFor(() => expect(screen.queryByTestId("intent-row")).toBeNull());
  });
});

describe("trades", () => {
  it("requires a price and quantity before recording", async () => {
    const recordTrade = vi.fn();
    const user = userEvent.setup();
    renderWithProviders(
      <TradesPage portfolioId="p1" valuationOverride={valuation} recordTrade={recordTrade} />,
      { route: "/portfolios/p1/trades" },
    );

    await user.type(screen.getByTestId("trade-quantity"), "1");
    await user.click(screen.getByTestId("trade-submit"));

    expect(screen.getByTestId("trade-errors")).toHaveTextContent(/price/i);
    expect(recordTrade).not.toHaveBeenCalled();
  });

  it("records a trade and labels a duplicate 409 with the existing trade notice", async () => {
    const recordTrade = vi
      .fn()
      .mockResolvedValueOnce({ id: "trade-1" })
      .mockRejectedValueOnce(
        new ApiError({
          type: "about:blank",
          title: "Conflict",
          status: 409,
          detail: "duplicate idempotency key",
          correlationId: "corr-409",
        }),
      );
    const user = userEvent.setup();
    renderWithProviders(
      <TradesPage portfolioId="p1" valuationOverride={valuation} recordTrade={recordTrade} />,
      { route: "/portfolios/p1/trades" },
    );

    await user.type(screen.getByTestId("trade-quantity"), "0.5");
    await user.type(screen.getByTestId("trade-price"), "1800");
    await user.click(screen.getByTestId("trade-submit"));
    expect(await screen.findByTestId("trade-saved")).toBeInTheDocument();

    await user.type(screen.getByTestId("trade-quantity"), "0.5");
    await user.click(screen.getByTestId("trade-submit"));
    expect(await screen.findByTestId("trade-duplicate")).toHaveTextContent(/already recorded/i);
  });
});

describe("snapshot history", () => {
  it("renders as-of, value and data status", () => {
    renderWithProviders(
      <HistoryPage
        snapshotsOverride={[
          {
            id: "s1",
            asOf: "2026-11-01T00:00:00Z",
            totalValue: "950.00",
            currency: "USD",
            dataStatus: "complete",
            verified: true,
          },
        ]}
      />,
      { route: "/portfolios/p1/history" },
    );
    expect(screen.getByTestId("data-table")).toHaveTextContent("950.00");
    expect(screen.getByTestId("data-table")).toHaveTextContent("Complete");
    expect(screen.getByTestId("data-table")).toHaveTextContent("2026");
  });

  it("shows an empty state when there are no snapshots", () => {
    renderWithProviders(<HistoryPage snapshotsOverride={[]} />, {
      route: "/portfolios/p1/history",
    });
    expect(screen.getByTestId("empty-state")).toBeInTheDocument();
  });

  it("shows an error panel when the history source fails", async () => {
    stubGateway({
      "GET /v1/portfolios/p1/snapshots": () => ({
        status: 500,
        body: problem(500, "outage"),
      }),
    });
    renderWithProviders(<HistoryPage portfolioId="p1" />, {
      route: "/portfolios/p1/history",
    });
    expect(await screen.findByTestId("error-panel")).toBeInTheDocument();
  });
});
