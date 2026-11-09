import { useEffect, useState } from "react";
import { useQueries } from "@tanstack/react-query";
import { useNavigate } from "react-router-dom";
import { gatewayClient } from "../../api/client";
import { queryKeys, usePortfolios } from "../../api/queries";
import type { PortfolioValuation } from "../../api/types";
import {
  DataTable,
  ErrorPanel,
  EmptyState,
  Loading,
  PageHeader,
  Points,
  Percent,
  PriceProvenance,
  DataStatusBadge,
  Money,
  ValuationSummary,
} from "../../components";
import { useI18n } from "../../i18n/react";

const BATCH = 4;

function PortfolioPanel({ valuation }: { valuation: PortfolioValuation }) {
  const { t } = useI18n();
  return (
    <section className="grid gap-4" data-testid="portfolio-panel">
      <ValuationSummary valuation={valuation} />
      <div>
        <h3 className="mb-2 font-display text-section font-medium text-ink">
          {t("portfolio.breakdown")}
        </h3>
        <DataTable
          caption={t("portfolio.breakdown")}
          columns={[
            { key: "asset", header: t("portfolio.asset") },
            { key: "quantity", header: t("portfolio.quantity"), numeric: true },
            { key: "value", header: t("portfolio.value"), numeric: true },
            { key: "allocation", header: t("portfolio.actual"), numeric: true },
            { key: "target", header: t("portfolio.target"), numeric: true },
            { key: "drift", header: t("portfolio.driftPoints"), numeric: true },
            { key: "status", header: t("dashboard.drift") },
          ]}
          rows={valuation.lines.map((line) => ({
            id: line.assetId,
            asset: line.symbol ?? line.assetId,
            quantity: line.quantity,
            value: <Money value={line.value} currency={valuation.currency} testId="money-cell" />,
            allocation: <Percent value={line.allocation} />,
            target: <Percent value={line.targetPercent} />,
            drift: <Points value={line.driftPoints} />,
            status: (
              <div className="grid gap-1">
                <DataStatusBadge status={line.dataStatus} />
                <PriceProvenance
                  source={line.priceSource}
                  observedAt={line.priceObservedAt}
                  fxSource={line.fxSource}
                  fxObservedAt={line.fxObservedAt}
                />
              </div>
            ),
          }))}
        />
      </div>
    </section>
  );
}

export default function DashboardPage() {
  const { t } = useI18n();
  const navigate = useNavigate();
  const portfolios = usePortfolios();
  const [visible, setVisible] = useState(BATCH);

  const valuations = useQueries({
    queries: (portfolios.data ?? []).map((portfolio, index) => ({
      queryKey: queryKeys.portfolio(portfolio.id),
      queryFn: () => gatewayClient.getPortfolio(portfolio.id),
      staleTime: 30_000,
      enabled: index < visible,
    })),
  });

  useEffect(() => {
    if (!portfolios.data || visible >= portfolios.data.length) {
      return;
    }
    const settled = valuations.slice(0, visible).every((query) => !query.isLoading);
    if (settled) {
      setVisible((current) => current + BATCH);
    }
  }, [portfolios.data, valuations, visible]);

  if (portfolios.isLoading) {
    return <Loading />;
  }
  if (portfolios.error) {
    return <ErrorPanel error={portfolios.error} onRetry={() => void portfolios.refetch()} />;
  }

  return (
    <section>
      <PageHeader title={t("dashboard.title")} />
      {portfolios.data && portfolios.data.length === 0 ? (
        <EmptyState
          title={t("dashboard.noPortfolios")}
          actionLabel={t("portfolio.new")}
          onAction={() => navigate("/portfolios")}
        />
      ) : (
        <div className="grid gap-8">
          {(portfolios.data ?? []).map((portfolio, index) => {
            const query = valuations[index];
            return (
              <article key={portfolio.id}>
                <div className="mb-2 flex items-center justify-between">
                  <h2 className="font-display text-section font-medium text-ink">
                    {portfolio.name}
                  </h2>
                  <span className="text-small text-meta">{portfolio.baseCurrency}</span>
                </div>
                {query?.isLoading ? (
                  <Loading />
                ) : query?.error ? (
                  <ErrorPanel error={query.error} onRetry={() => void query.refetch()} />
                ) : query?.data ? (
                  <PortfolioPanel valuation={query.data} />
                ) : null}
              </article>
            );
          })}
        </div>
      )}
    </section>
  );
}
