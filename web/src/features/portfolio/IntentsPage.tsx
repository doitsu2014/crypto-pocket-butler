import { useState, type FormEvent } from "react";
import { useParams } from "react-router-dom";
import { gatewayClient } from "../../api/client";
import { problemMessages } from "../../api/errors";
import { usePortfolio } from "../../api/queries";
import type {
  OrderIntent,
  OrderIntentRequest,
  PortfolioValuation,
} from "../../api/types";
import { ErrorPanel, Loading, PageHeader, Percent, Points } from "../../components";
import { useI18n } from "../../i18n/react";

export default function IntentsPage({
  portfolioId: portfolioIdProp,
  valuationOverride,
  preview = (id: string, intent: OrderIntentRequest) =>
    gatewayClient.previewIntent(id, intent),
  create = (id: string, intent: OrderIntentRequest) =>
    gatewayClient.createIntent(id, intent),
  cancel = (id: string, intentId: string) =>
    gatewayClient.cancelIntent(id, intentId),
}: {
  portfolioId?: string;
  valuationOverride?: PortfolioValuation;
  preview?: (portfolioId: string, intent: OrderIntentRequest) => Promise<OrderIntent>;
  create?: (portfolioId: string, intent: OrderIntentRequest) => Promise<OrderIntent>;
  cancel?: (portfolioId: string, intentId: string) => Promise<OrderIntent>;
} = {}) {
  const { t } = useI18n();
  const params = useParams();
  const portfolioId = portfolioIdProp ?? params.portfolioId;
  const query = usePortfolio(valuationOverride ? undefined : portfolioId);
  const valuation = valuationOverride ?? query.data;
  const [assetId, setAssetId] = useState("");
  const [side, setSide] = useState<"buy" | "sell">("buy");
  const [quantity, setQuantity] = useState("");
  const [previewResult, setPreviewResult] = useState<OrderIntent | null>(null);
  const [intents, setIntents] = useState<OrderIntent[]>([]);
  const [errors, setErrors] = useState<string[]>([]);
  const [busy, setBusy] = useState(false);

  if (query.isLoading) {
    return <Loading />;
  }
  if (query.error) {
    return <ErrorPanel error={query.error} onRetry={() => void query.refetch()} />;
  }
  if (!valuation) {
    return <Loading />;
  }

  const selectedAsset = assetId || valuation.lines[0]?.assetId || "";

  function valid(): boolean {
    const parsed = Number(quantity);
    return selectedAsset !== "" && Number.isFinite(parsed) && parsed > 0;
  }

  async function onPreview(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setErrors([]);
    if (!valid() || !portfolioId) {
      setErrors([t("common.required")]);
      return;
    }
    setBusy(true);
    try {
      setPreviewResult(
        await preview(portfolioId, {
          assetId: selectedAsset,
          side,
          quantity,
        }),
      );
    } catch (error) {
      setErrors(problemMessages(error));
    } finally {
      setBusy(false);
    }
  }

  async function onCreate() {
    setErrors([]);
    if (!valid() || !portfolioId) {
      setErrors([t("common.required")]);
      return;
    }
    setBusy(true);
    try {
      const created = await create(portfolioId, {
        assetId: selectedAsset,
        side,
        quantity,
      });
      setIntents((current) => [created, ...current]);
      setPreviewResult(null);
      setQuantity("");
    } catch (error) {
      setErrors(problemMessages(error));
    } finally {
      setBusy(false);
    }
  }

  async function onCancel(intentId: string) {
    if (!portfolioId) return;
    try {
      const cancelled = await cancel(portfolioId, intentId);
      setIntents((current) =>
        current.map((intent) => (intent.id === intentId ? cancelled : intent)),
      );
    } catch (error) {
      setErrors(problemMessages(error));
    }
  }

  return (
    <section>
      <PageHeader title={t("portfolio.intents")} />

      {errors.length > 0 ? (
        <ul data-testid="intent-errors" role="alert" className="mb-3 text-small text-danger">
          {errors.map((message) => (
            <li key={message}>{message}</li>
          ))}
        </ul>
      ) : null}

      <form data-testid="intent-form" onSubmit={onPreview} className="grid gap-3 rounded-lg border border-border bg-surface p-4 sm:grid-cols-3">
        <label className="flex flex-col gap-1 text-small font-medium text-ink-2">
          {t("portfolio.asset")}
          <select
            data-testid="intent-asset"
            value={selectedAsset}
            onChange={(event) => setAssetId(event.target.value)}
            className="rounded-xl border border-border bg-surface px-3 py-2 text-small"
          >
            {valuation.lines.map((line) => (
              <option key={line.assetId} value={line.assetId}>
                {line.symbol ?? line.assetId}
              </option>
            ))}
          </select>
        </label>
        <label className="flex flex-col gap-1 text-small font-medium text-ink-2">
          {t("portfolio.side")}
          <select
            data-testid="intent-side"
            value={side}
            onChange={(event) => setSide(event.target.value as "buy" | "sell")}
            className="rounded-xl border border-border bg-surface px-3 py-2 text-small"
          >
            <option value="buy">{t("portfolio.buy")}</option>
            <option value="sell">{t("portfolio.sell")}</option>
          </select>
        </label>
        <label className="flex flex-col gap-1 text-small font-medium text-ink-2">
          {t("portfolio.quantity")}
          <input
            data-testid="intent-quantity"
            type="number"
            min="0"
            step="any"
            value={quantity}
            onChange={(event) => setQuantity(event.target.value)}
            className="rounded-xl border border-border bg-surface px-3 py-2 font-mono text-small"
          />
        </label>
        <div className="flex items-end gap-2 sm:col-span-3">
          <button
            type="submit"
            data-testid="intent-preview"
            disabled={busy}
            className="rounded-lg bg-surface-warm px-3 py-1.5 text-small font-medium text-ink disabled:opacity-45"
          >
            {t("portfolio.preview")}
          </button>
          <button
            type="button"
            data-testid="intent-create"
            disabled={busy || !previewResult}
            onClick={() => void onCreate()}
            className="rounded-lg bg-accent px-4 py-2 text-small font-medium text-accent-on disabled:opacity-45"
          >
            {t("common.create")}
          </button>
        </div>
      </form>

      {previewResult ? (
        <section data-testid="intent-preview-result" className="mt-4 rounded-lg border border-border bg-surface p-4">
          <p className="text-small text-ink-2">
            {t("portfolio.projectedAllocation")}:{" "}
            <Percent value={previewResult.projectedAllocation} /> ·{" "}
            {t("portfolio.projectedDrift")}:{" "}
            <Points value={previewResult.projectedDriftPoints} />
          </p>
          {previewResult.shortfall ? (
            <p data-testid="intent-shortfall" className="mt-1 text-small text-warn">
              {t("portfolio.shortfall")}:{" "}
              <span className="font-mono">{previewResult.shortfall}</span>
            </p>
          ) : null}
        </section>
      ) : null}

      <ul className="mt-6 grid gap-2">
        {intents.map((intent) => (
          <li
            key={intent.id}
            data-testid="intent-row"
            className="flex flex-wrap items-center justify-between gap-2 rounded-lg border border-border bg-surface p-3 text-small"
          >
            <span className="text-ink">
              {intent.side} {intent.quantity} {intent.assetId}
            </span>
            <span className="flex items-center gap-2">
              <span className="text-meta">{intent.status}</span>
              {intent.status === "pending" ? (
                <button
                  type="button"
                  data-testid={`intent-cancel-${intent.id}`}
                  onClick={() => void onCancel(intent.id)}
                  className="rounded-lg bg-surface-warm px-2 py-1 text-small text-ink"
                >
                  {t("portfolio.cancel")}
                </button>
              ) : null}
            </span>
          </li>
        ))}
      </ul>
    </section>
  );
}
