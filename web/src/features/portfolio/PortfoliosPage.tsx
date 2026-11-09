import { useState, type FormEvent } from "react";
import { useMutation, useQueryClient } from "@tanstack/react-query";
import { Link, useNavigate } from "react-router-dom";
import { gatewayClient } from "../../api/client";
import { problemMessages } from "../../api/errors";
import { queryKeys, usePortfolios } from "../../api/queries";
import { EmptyState, ErrorPanel, Loading, PageHeader } from "../../components";
import { useI18n } from "../../i18n/react";

export default function PortfoliosPage() {
  const { t } = useI18n();
  const navigate = useNavigate();
  const queryClient = useQueryClient();
  const portfolios = usePortfolios();
  const [name, setName] = useState("");
  const [baseCurrency, setBaseCurrency] = useState("USD");
  const [error, setError] = useState<string | null>(null);
  const [renaming, setRenaming] = useState<string | null>(null);
  const [renameValue, setRenameValue] = useState("");

  const create = useMutation({
    mutationFn: () => gatewayClient.createPortfolio({ name, baseCurrency }),
    onSuccess: (portfolio) => {
      void queryClient.invalidateQueries({ queryKey: queryKeys.portfolios });
      setName("");
      navigate(`/portfolios/${portfolio.id}`);
    },
    onError: (caught: unknown) => setError(problemMessages(caught).join(" · ")),
  });

  const rename = useMutation({
    mutationFn: (portfolioId: string) =>
      gatewayClient.renamePortfolio(portfolioId, renameValue),
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: queryKeys.portfolios });
      setRenaming(null);
      setRenameValue("");
    },
    onError: (caught: unknown) => setError(problemMessages(caught).join(" · ")),
  });

  function onCreate(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setError(null);
    if (!name.trim()) {
      setError(t("common.required"));
      return;
    }
    create.mutate();
  }

  return (
    <section>
      <PageHeader title={t("portfolio.title")} />

      {error ? (
        <p data-testid="portfolio-error" role="alert" className="mb-3 text-small text-danger">
          {error}
        </p>
      ) : null}

      <form
        data-testid="portfolio-create"
        onSubmit={onCreate}
        className="mb-6 flex flex-wrap items-end gap-3 rounded-lg border border-border bg-surface p-4"
      >
        <label className="flex flex-col gap-1 text-small font-medium text-ink-2">
          {t("portfolio.name")}
          <input
            data-testid="portfolio-name"
            value={name}
            onChange={(event) => setName(event.target.value)}
            className="rounded-xl border border-border bg-surface px-3 py-2 text-small"
          />
        </label>
        <label className="flex flex-col gap-1 text-small font-medium text-ink-2">
          {t("onboarding.baseCurrency")}
          <input
            data-testid="portfolio-currency"
            value={baseCurrency}
            onChange={(event) => setBaseCurrency(event.target.value)}
            className="w-24 rounded-xl border border-border bg-surface px-3 py-2 text-small"
          />
        </label>
        <button
          type="submit"
          data-testid="portfolio-create-submit"
          disabled={create.isPending}
          className="rounded-lg bg-accent px-4 py-2 text-small font-medium text-accent-on disabled:opacity-45"
        >
          {t("portfolio.new")}
        </button>
      </form>

      {portfolios.isLoading ? (
        <Loading />
      ) : portfolios.error ? (
        <ErrorPanel error={portfolios.error} onRetry={() => void portfolios.refetch()} />
      ) : portfolios.data && portfolios.data.length === 0 ? (
        <EmptyState
          title={t("dashboard.noPortfolios")}
          actionLabel={t("portfolio.new")}
          onAction={() => {
            const input = document.querySelector<HTMLInputElement>(
              '[data-testid="portfolio-name"]',
            );
            input?.focus();
          }}
        />
      ) : (
        <ul className="grid gap-3">
          {(portfolios.data ?? []).map((portfolio) => (
            <li
              key={portfolio.id}
              data-testid="portfolio-row"
              className="flex flex-wrap items-center justify-between gap-3 rounded-lg border border-border bg-surface p-4"
            >
              {renaming === portfolio.id ? (
                <div className="flex items-center gap-2">
                  <input
                    data-testid="portfolio-rename-input"
                    value={renameValue}
                    onChange={(event) => setRenameValue(event.target.value)}
                    className="rounded-xl border border-border bg-surface px-3 py-2 text-small"
                  />
                  <button
                    type="button"
                    data-testid="portfolio-rename-save"
                    onClick={() => rename.mutate(portfolio.id)}
                    className="rounded-lg bg-accent px-3 py-1.5 text-small font-medium text-accent-on"
                  >
                    {t("common.save")}
                  </button>
                </div>
              ) : (
                <Link
                  to={`/portfolios/${portfolio.id}`}
                  className="font-display text-section font-medium text-ink underline"
                >
                  {portfolio.name}
                </Link>
              )}
              <div className="flex items-center gap-2">
                <span className="text-small text-meta">{portfolio.baseCurrency}</span>
                <button
                  type="button"
                  data-testid="portfolio-rename"
                  onClick={() => {
                    setRenaming(portfolio.id);
                    setRenameValue(portfolio.name);
                  }}
                  className="rounded-lg bg-surface-warm px-3 py-1 text-small text-ink"
                >
                  {t("portfolio.rename")}
                </button>
              </div>
            </li>
          ))}
        </ul>
      )}
    </section>
  );
}
