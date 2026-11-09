import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import {
  AsOf,
  BlockingState,
  ConfirmDialog,
  CoverageNote,
  DataStatusBadge,
  DataTable,
  EmptyState,
  ErrorPanel,
  FreshnessBadge,
  Money,
  Percent,
  PnlFigures,
  Points,
  PriceProvenance,
  ValuationSummary,
} from "./index";
import { ApiError, NetworkError } from "../api/client";
import { I18nProvider } from "../i18n/react";

function renderWithI18n(node: React.ReactNode, locale: "en" | "vi" = "en") {
  return render(
    <I18nProvider locale={locale} timezone="UTC">
      {node}
    </I18nProvider>,
  );
}

describe("Money and figures", () => {
  it("renders a supplied decimal string with its currency", () => {
    renderWithI18n(<Money value="1234.56" currency="USD" />);
    expect(screen.getByTestId("money")).toHaveTextContent("1234.56");
    expect(screen.getByTestId("money")).toHaveTextContent("USD");
  });

  it("renders a missing value as an em dash, never zero", () => {
    renderWithI18n(<Money value={null} currency="USD" />);
    const money = screen.getByTestId("money");
    expect(money).toHaveTextContent("—");
    expect(money).not.toHaveTextContent("0");
    expect(money).not.toHaveTextContent("USD");
  });

  it("renders percent and signed percentage points, and em dashes when absent", () => {
    const { rerender } = renderWithI18n(
      <>
        <Percent value={12} />
        <Points value={10} />
        <Points value={-4} />
      </>,
    );
    expect(screen.getByTestId("percent")).toHaveTextContent("12%");
    expect(screen.getAllByTestId("points")[0]).toHaveTextContent("+10 pp");
    expect(screen.getAllByTestId("points")[1]).toHaveTextContent("−4 pp");

    rerender(
      <I18nProvider locale="en" timezone="UTC">
        <>
          <Percent value={null} />
          <Points value={null} />
        </>
      </I18nProvider>,
    );
    expect(screen.getByTestId("percent")).toHaveTextContent("—");
    expect(screen.getByTestId("points")).toHaveTextContent("—");
  });
});

describe("status and provenance", () => {
  it("pairs a status colour with a shape and a text label", () => {
    renderWithI18n(<DataStatusBadge status="partial" asOf="2026-11-08T12:00:00Z" />);
    const badge = screen.getByTestId("data-status");
    expect(badge).toHaveTextContent("Partial");
    expect(badge).toHaveTextContent("◐");
    expect(badge).toHaveTextContent("2026");
  });

  it("labels freshness and keeps the observed timestamp", () => {
    renderWithI18n(<FreshnessBadge freshness="stale" observedAt="2026-11-08T12:00:00Z" />);
    const badge = screen.getByTestId("freshness");
    expect(badge).toHaveTextContent("Stale");
    expect(badge).toHaveTextContent("2026");
  });

  it("names every coverage gap", () => {
    renderWithI18n(
      <CoverageNote
        coverage={0.8}
        excludedCount={2}
        marksUnavailable
        fxEstimated
        gap={["DOGE"]}
      />,
    );
    const note = screen.getByTestId("coverage-note");
    expect(note).toHaveTextContent("80%");
    expect(note).toHaveTextContent("Excluded: 2");
    expect(note).toHaveTextContent("Marks unavailable");
    expect(note).toHaveTextContent("FX estimated");
    expect(note).toHaveTextContent("DOGE");
  });

  it("shows provenance and an unavailable label when the source is absent", () => {
    const { rerender } = renderWithI18n(
      <PriceProvenance source="Binance" observedAt="2026-11-08T12:00:00Z" />,
    );
    expect(screen.getByTestId("price-provenance")).toHaveTextContent("Binance");

    rerender(
      <I18nProvider locale="en" timezone="UTC">
        <PriceProvenance source={null} observedAt={null} />
      </I18nProvider>,
    );
    expect(screen.getByTestId("price-provenance")).toHaveTextContent("Unavailable");
  });

  it("formats an as-of instant and falls back to an em dash", () => {
    const { rerender } = renderWithI18n(<AsOf at="2026-11-08T12:00:00Z" />);
    expect(screen.getByTestId("as-of")).toHaveTextContent("2026");

    rerender(
      <I18nProvider locale="en" timezone="UTC">
        <AsOf at={null} />
      </I18nProvider>,
    );
    expect(screen.getByTestId("as-of")).toHaveTextContent("—");
  });
});

describe("error and empty states", () => {
  it("renders the problem title, detail, correlation id and retry", async () => {
    const onRetry = vi.fn();
    renderWithI18n(
      <ErrorPanel
        error={
          new ApiError({
            type: "about:blank",
            title: "Conflict",
            status: 409,
            detail: "duplicate idempotency key",
            correlationId: "corr-9",
          })
        }
        onRetry={onRetry}
      />,
    );

    const panel = screen.getByTestId("error-panel");
    expect(panel).toHaveTextContent("conflicts with the current state");
    expect(panel).toHaveTextContent("duplicate idempotency key");
    expect(screen.getByTestId("correlation-id")).toHaveTextContent("corr-9");
    await userEvent.click(screen.getByTestId("error-retry"));
    expect(onRetry).toHaveBeenCalledOnce();
  });

  it("explains a network failure without a problem body", () => {
    renderWithI18n(<ErrorPanel error={new NetworkError("Failed to fetch")} />);
    expect(screen.getByTestId("error-panel")).toHaveTextContent(/gateway/i);
    expect(screen.queryByTestId("correlation-id")).toBeNull();
  });

  it("renders one empty-state action and calls it", async () => {
    const onAction = vi.fn();
    renderWithI18n(
      <EmptyState title="No assets found" actionLabel="Add a wallet" onAction={onAction} />,
    );
    expect(screen.getByTestId("empty-state")).toHaveTextContent("No assets found");
    await userEvent.click(screen.getByTestId("empty-action"));
    expect(onAction).toHaveBeenCalledOnce();
  });
});

describe("confirm, table and blocking states", () => {
  it("requires a confirmation for a destructive action", async () => {
    const onConfirm = vi.fn();
    const onCancel = vi.fn();
    const { rerender } = renderWithI18n(
      <ConfirmDialog
        open
        title="Delete account"
        confirmLabel="Delete"
        onConfirm={onConfirm}
        onCancel={onCancel}
      />,
    );
    await userEvent.click(screen.getByTestId("confirm-accept"));
    expect(onConfirm).toHaveBeenCalledOnce();
    await userEvent.click(screen.getByTestId("confirm-cancel"));
    expect(onCancel).toHaveBeenCalledOnce();

    rerender(
      <I18nProvider locale="en" timezone="UTC">
        <ConfirmDialog
          open={false}
          title="Delete account"
          confirmLabel="Delete"
          onConfirm={onConfirm}
          onCancel={onCancel}
        />
      </I18nProvider>,
    );
    expect(screen.queryByTestId("confirm-dialog")).toBeNull();
  });

  it("renders a real table with a caption, scoped headers and a numeric column", () => {
    renderWithI18n(
      <DataTable
        caption="Allocation"
        columns={[
          { key: "asset", header: "Asset" },
          { key: "value", header: "Value", numeric: true },
        ]}
        rows={[{ id: "btc", asset: "BTC", value: "100.00" }]}
      />,
    );
    const table = screen.getByTestId("data-table");
    expect(table.querySelector("caption")).toHaveTextContent("Allocation");
    expect(table.querySelector("th")?.getAttribute("scope")).toBe("col");
    expect(table).toHaveTextContent("100.00");
  });

  it("names the blocking C1 operation", () => {
    renderWithI18n(
      <BlockingState operation="GET /v1/users/{userId}/pnl" unit="U7" />,
    );
    expect(screen.getByTestId("blocking-state")).toHaveTextContent(
      "GET /v1/users/{userId}/pnl",
    );
  });
});

describe("valuation and P&L summaries", () => {
  it("renders a total with status, coverage and excluded count", () => {
    renderWithI18n(
      <ValuationSummary
        valuation={{
          totalValue: "1000.00",
          currency: "USD",
          status: "partial",
          asOf: "2026-11-08T12:00:00Z",
          pricedCoverage: 0.9,
          unpricedCount: 1,
          excludedCount: 1,
        }}
      />,
    );
    expect(screen.getByTestId("money")).toHaveTextContent("1000.00");
    expect(screen.getByTestId("data-status")).toHaveTextContent("Partial");
    expect(screen.getByTestId("coverage-note")).toHaveTextContent("Excluded: 1");
  });

  it("withholds an unavailable total as an em dash", () => {
    renderWithI18n(
      <ValuationSummary
        valuation={{
          totalValue: null,
          currency: "USD",
          status: "unavailable",
          asOf: "2026-11-08T12:00:00Z",
          pricedCoverage: 0,
          unpricedCount: 10,
          holdingsUnavailable: true,
        }}
      />,
    );
    expect(screen.getByTestId("money")).toHaveTextContent("—");
    expect(screen.getByTestId("coverage-note")).toHaveTextContent(
      "Holdings unavailable",
    );
  });

  it("labels P&L figures with marks, holdings and FX flags", () => {
    renderWithI18n(
      <PnlFigures
        pnl={{
          asOf: "2026-11-08T12:00:00Z",
          netInvested: null,
          realized: null,
          unrealized: null,
          coverage: 0.5,
          currency: "USD",
          fxAsOf: null,
          fxEstimated: true,
          marksUnavailable: true,
          holdingsUnavailable: false,
          fxUnavailable: false,
          coverageGap: ["DOGE"],
        }}
      />,
    );
    expect(screen.getAllByTestId("money")[0]).toHaveTextContent("—");
    const note = screen.getByTestId("coverage-note");
    expect(note).toHaveTextContent("Marks unavailable");
    expect(note).toHaveTextContent("FX estimated");
    expect(note).toHaveTextContent("DOGE");
  });
});
