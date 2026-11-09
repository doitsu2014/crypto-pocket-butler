import { useState } from "react";
import { useMutation, useQueryClient } from "@tanstack/react-query";
import { useParams } from "react-router-dom";
import { gatewayClient } from "../../api/client";
import { problemMessages } from "../../api/errors";
import {
  queryKeys,
  useAccounts,
  usePortfolio,
  usePortfolios,
  useProfile,
} from "../../api/queries";
import {
  DataStatusBadge,
  DataTable,
  ErrorPanel,
  Loading,
  Money,
  PageHeader,
  Percent,
  Points,
  PriceProvenance,
  ValuationSummary,
} from "../../components";
import { useI18n } from "../../i18n/react";

export default function PortfolioDetailPage({
  portfolioId: portfolioIdProp,
  userId: userIdProp,
}: {
  portfolioId?: string;
  userId?: string;
} = {}) {
  const { t } = useI18n();
  const params = useParams();
  const portfolioId = portfolioIdProp ?? params.portfolioId;
  const queryClient = useQueryClient();
  const valuation = usePortfolio(portfolioId);
  const profile = useProfile(!userIdProp);
  const userId = userIdProp ?? profile.data?.userId;
  const accounts = useAccounts(userId);
  const portfolios = usePortfolios();
  const [error, setError] = useState<string | null>(null);

  const move = useMutation({
    mutationFn: (input: { accountId: string; targetPortfolioId: string }) =>
      gatewayClient.assignAccount(input.targetPortfolioId, input.accountId),
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: queryKeys.portfolios });
      if (portfolioId) {
        void queryClient.invalidateQueries({
          queryKey: queryKeys.portfolio(portfolioId),
        });
      }
    },
    onError: (caught: unknown) => setError(problemMessages(caught).join(" · ")),
  });

  const remove = useMutation({
    mutationFn: (accountId: string) =>
      gatewayClient.removeAccount(portfolioId as string, accountId),
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: queryKeys.portfolios });
      if (portfolioId) {
        void queryClient.invalidateQueries({
          queryKey: queryKeys.portfolio(portfolioId),
        });
      }
    },
    onError: (caught: unknown) => setError(problemMessages(caught).join(" · ")),
  });

  if (valuation.isLoading) {
    return <Loading />;
  }
  if (valuation.error) {
    return <ErrorPanel error={valuation.error} onRetry={() => void valuation.refetch()} />;
  }
  if (!valuation.data) {
    return <Loading />;
  }

  return (
    <section className="grid gap-6">
      <PageHeader title={valuation.data.portfolioId} />

      {error ? (
        <p data-testid="detail-error" role="alert" className="text-small text-danger">
          {error}
        </p>
      ) : null}

      <ValuationSummary valuation={valuation.data} />

      <div>
        <h2 className="mb-2 font-display text-section font-medium text-ink">
          {t("portfolio.breakdown")}
        </h2>
        <DataTable
          caption={t("portfolio.breakdown")}
          columns={[
            { key: "asset", header: t("portfolio.asset") },
            { key: "quantity", header: t("portfolio.quantity"), numeric: true },
            { key: "value", header: t("portfolio.value"), numeric: true },
            { key: "actual", header: t("portfolio.actual"), numeric: true },
            { key: "target", header: t("portfolio.target"), numeric: true },
            { key: "drift", header: t("portfolio.driftPoints"), numeric: true },
            { key: "status", header: t("activity.coverage") },
          ]}
          rows={valuation.data.lines.map((line) => ({
            id: line.assetId,
            asset: line.symbol ?? line.assetId,
            quantity: line.quantity,
            value: <Money value={line.value} currency={valuation.data.currency} />,
            actual: <Percent value={line.allocation} />,
            target: <Percent value={line.targetPercent} />,
            drift: <Points value={line.driftPoints} />,
            status: (
              <div className="grid gap-1">
                <DataStatusBadge
                  status={line.dataStatus}
                  asOf={line.priceObservedAt}
                />
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

      <div>
        <h2 className="mb-2 font-display text-section font-medium text-ink">
          {t("portfolio.accounts")}
        </h2>
        <ul className="grid gap-2">
          {(accounts.data ?? []).map((account) => (
            <li
              key={account.id}
              data-testid="detail-account"
              className="flex flex-wrap items-center justify-between gap-2 rounded-lg border border-border bg-surface p-3"
            >
              <span className="text-small text-ink">{account.label}</span>
              <div className="flex items-center gap-2">
                <select
                  data-testid={`move-${account.id}`}
                  aria-label={t("portfolio.moveAccount")}
                  defaultValue=""
                  onChange={(event) => {
                    if (event.target.value) {
                      move.mutate({
                        accountId: account.id,
                        targetPortfolioId: event.target.value,
                      });
                    }
                  }}
                  className="rounded-xl border border-border bg-surface px-2 py-1 text-small"
                >
                  <option value="">{t("portfolio.moveAccount")}</option>
                  {(portfolios.data ?? [])
                    .filter((portfolio) => portfolio.id !== portfolioId)
                    .map((portfolio) => (
                      <option key={portfolio.id} value={portfolio.id}>
                        {portfolio.name}
                      </option>
                    ))}
                </select>
                <button
                  type="button"
                  data-testid={`remove-${account.id}`}
                  onClick={() => remove.mutate(account.id)}
                  className="rounded-lg bg-surface-warm px-2 py-1 text-small text-ink"
                >
                  {t("portfolio.removeAccount")}
                </button>
              </div>
            </li>
          ))}
        </ul>
      </div>
    </section>
  );
}
