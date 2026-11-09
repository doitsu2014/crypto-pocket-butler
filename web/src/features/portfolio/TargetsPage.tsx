import { useMemo, useState } from "react";
import { useParams } from "react-router-dom";
import { gatewayClient } from "../../api/client";
import { problemMessages } from "../../api/errors";
import { usePortfolio } from "../../api/queries";
import type { PortfolioValuation, TargetInput, TargetSetPreview } from "../../api/types";
import { ErrorPanel, Loading, PageHeader, Percent } from "../../components";
import { useI18n } from "../../i18n/react";

export default function TargetsPage({
  portfolioId: portfolioIdProp,
  valuationOverride,
  preview = (id: string, targets: TargetInput[]) =>
    gatewayClient.previewTargets(id, targets),
  save = (id: string, targets: TargetInput[]) =>
    gatewayClient.saveTargets(id, targets),
}: {
  portfolioId?: string;
  valuationOverride?: PortfolioValuation;
  preview?: (portfolioId: string, targets: TargetInput[]) => Promise<TargetSetPreview>;
  save?: (portfolioId: string, targets: TargetInput[]) => Promise<void>;
} = {}) {
  const { t } = useI18n();
  const params = useParams();
  const portfolioId = portfolioIdProp ?? params.portfolioId;
  const query = usePortfolio(valuationOverride ? undefined : portfolioId);
  const valuation = valuationOverride ?? query.data;
  const [rows, setRows] = useState<TargetInput[] | null>(null);
  const [result, setResult] = useState<TargetSetPreview | null>(null);
  const [errors, setErrors] = useState<string[]>([]);
  const [saving, setSaving] = useState(false);

  const current = useMemo<TargetInput[]>(() => {
    if (rows) {
      return rows;
    }
    return (valuation?.lines ?? []).map((line) => ({
      assetId: line.assetId,
      targetPercent: line.targetPercent ?? 0,
      guardrailPoints: line.guardrailPoints ?? undefined,
    }));
  }, [rows, valuation]);

  const sum = current.reduce((total, row) => total + row.targetPercent, 0);
  const difference = Number((sum - 100).toFixed(6));
  const balanced = Math.abs(difference) < 0.0001;

  if (query.isLoading) {
    return <Loading />;
  }
  if (query.error) {
    return <ErrorPanel error={query.error} onRetry={() => void query.refetch()} />;
  }
  if (!valuation) {
    return <Loading />;
  }

  const noneSet = valuation.lines.every((line) => line.targetPercent === null);

  async function onPreview() {
    setErrors([]);
    if (!portfolioId) return;
    try {
      setResult(await preview(portfolioId, current));
    } catch (error) {
      setErrors(problemMessages(error));
    }
  }

  async function onSave() {
    setErrors([]);
    if (!portfolioId || !balanced) return;
    setSaving(true);
    try {
      await save(portfolioId, current);
    } catch (error) {
      setErrors(problemMessages(error));
    } finally {
      setSaving(false);
    }
  }

  return (
    <section>
      <PageHeader title={t("portfolio.targets")} />

      {noneSet ? (
        <p data-testid="no-targets" className="mb-3 rounded-lg border border-border bg-surface-warm p-3 text-small text-ink-2">
          {t("portfolio.noTargets")}
        </p>
      ) : null}

      {errors.length > 0 ? (
        <ul data-testid="targets-errors" role="alert" className="mb-3 text-small text-danger">
          {errors.map((message) => (
            <li key={message}>{message}</li>
          ))}
        </ul>
      ) : null}

      <div className="grid gap-2">
        {valuation.lines.map((line, index) => (
          <label
            key={line.assetId}
            className="flex items-center justify-between gap-3 rounded-lg border border-border bg-surface p-3 text-small"
          >
            <span className="text-ink">{line.symbol ?? line.assetId}</span>
            <span className="flex items-center gap-2">
              <input
                data-testid={`target-${line.assetId}`}
                type="number"
                min={0}
                max={100}
                value={current[index]?.targetPercent ?? 0}
                onChange={(event) => {
                  const value = Math.max(
                    0,
                    Math.min(100, Number(event.target.value)),
                  );
                  setRows(
                    current.map((row, rowIndex) =>
                      rowIndex === index ? { ...row, targetPercent: value } : row,
                    ),
                  );
                }}
                className="w-20 rounded-xl border border-border bg-surface px-2 py-1 text-right font-mono"
              />
              <Percent value={current[index]?.targetPercent ?? 0} />
            </span>
          </label>
        ))}
      </div>

      <div className="mt-4 flex flex-wrap items-center gap-4 text-small">
        <span data-testid="target-sum" className="text-ink-2">
          {t("portfolio.sum")}: <Percent value={sum} />
        </span>
        <span data-testid="target-difference" className="text-ink-2">
          {t("portfolio.difference")}: <span className="font-mono">{difference}</span>
        </span>
        <button
          type="button"
          data-testid="targets-preview"
          onClick={() => void onPreview()}
          className="rounded-lg bg-surface-warm px-3 py-1.5 text-small font-medium text-ink"
        >
          {t("portfolio.preview")}
        </button>
        <button
          type="button"
          data-testid="targets-save"
          disabled={!balanced || saving}
          onClick={() => void onSave()}
          className="rounded-lg bg-accent px-4 py-2 text-small font-medium text-accent-on disabled:opacity-45"
        >
          {t("portfolio.saveTargets")}
        </button>
      </div>

      {result ? (
        <p data-testid="targets-preview-result" className="mt-3 text-small text-meta">
          {t("portfolio.sum")}: {result.sum} · {t("portfolio.difference")}:{" "}
          {result.difference}
        </p>
      ) : null}
    </section>
  );
}
