import { useState, type FormEvent } from "react";
import { useParams } from "react-router-dom";
import { gatewayClient, ApiError } from "../../api/client";
import { problemMessages } from "../../api/errors";
import { usePortfolio } from "../../api/queries";
import type { PortfolioValuation, Trade, TradeRequest } from "../../api/types";
import { ErrorPanel, Loading, PageHeader } from "../../components";
import { useI18n } from "../../i18n/react";

function newKey(): string {
  const cryptoApi = globalThis.crypto;
  if (cryptoApi && typeof cryptoApi.randomUUID === "function") {
    return cryptoApi.randomUUID();
  }
  return `trade-${Date.now()}-${Math.random().toString(16).slice(2)}`;
}

export default function TradesPage({
  portfolioId: portfolioIdProp,
  valuationOverride,
  recordTrade = (id: string, trade: TradeRequest) =>
    gatewayClient.recordTrade(id, trade),
}: {
  portfolioId?: string;
  valuationOverride?: PortfolioValuation;
  recordTrade?: (portfolioId: string, trade: TradeRequest) => Promise<Trade>;
} = {}) {
  const { t } = useI18n();
  const params = useParams();
  const portfolioId = portfolioIdProp ?? params.portfolioId;
  const query = usePortfolio(valuationOverride ? undefined : portfolioId);
  const valuation = valuationOverride ?? query.data;
  const [assetId, setAssetId] = useState("");
  const [side, setSide] = useState<"buy" | "sell">("buy");
  const [quantity, setQuantity] = useState("");
  const [price, setPrice] = useState("");
  const [priceSource, setPriceSource] = useState<"venue" | "user">("venue");
  const [executedAt, setExecutedAt] = useState(new Date().toISOString().slice(0, 16));
  const [idempotencyKey, setIdempotencyKey] = useState(newKey);
  const [errors, setErrors] = useState<string[]>([]);
  const [duplicate, setDuplicate] = useState(false);
  const [saved, setSaved] = useState(false);
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

  async function onSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setErrors([]);
    setDuplicate(false);
    setSaved(false);
    const parsedQuantity = Number(quantity);
    const parsedPrice = Number(price);
    const messages: string[] = [];
    if (!selectedAsset) messages.push(`${t("portfolio.asset")}: ${t("common.required")}`);
    if (!Number.isFinite(parsedQuantity) || parsedQuantity <= 0) {
      messages.push(`${t("portfolio.quantity")}: ${t("common.required")}`);
    }
    if (!Number.isFinite(parsedPrice) || parsedPrice < 0 || price === "") {
      messages.push(`${t("portfolio.price")}: ${t("common.required")}`);
    }
    if (messages.length > 0) {
      setErrors(messages);
      return;
    }
    setBusy(true);
    try {
      await recordTrade(portfolioId as string, {
        assetId: selectedAsset,
        side,
        quantity,
        price,
        priceSource,
        executedAt: new Date(executedAt).toISOString(),
        idempotencyKey,
      });
      setSaved(true);
      setIdempotencyKey(newKey());
      setQuantity("");
    } catch (error) {
      if (error instanceof ApiError && error.problem.status === 409) {
        setDuplicate(true);
      }
      setErrors(problemMessages(error));
    } finally {
      setBusy(false);
    }
  }

  return (
    <section>
      <PageHeader title={t("portfolio.trades")} />

      <form data-testid="trade-form" onSubmit={onSubmit} className="grid gap-3 rounded-lg border border-border bg-surface p-4 sm:grid-cols-2" noValidate>
        <label className="flex flex-col gap-1 text-small font-medium text-ink-2">
          {t("portfolio.asset")}
          <select
            data-testid="trade-asset"
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
            data-testid="trade-side"
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
            data-testid="trade-quantity"
            type="number"
            min="0"
            step="any"
            value={quantity}
            onChange={(event) => setQuantity(event.target.value)}
            className="rounded-xl border border-border bg-surface px-3 py-2 font-mono text-small"
          />
        </label>
        <label className="flex flex-col gap-1 text-small font-medium text-ink-2">
          {t("portfolio.price")}
          <input
            data-testid="trade-price"
            type="number"
            min="0"
            step="any"
            value={price}
            onChange={(event) => setPrice(event.target.value)}
            className="rounded-xl border border-border bg-surface px-3 py-2 font-mono text-small"
          />
        </label>
        <label className="flex flex-col gap-1 text-small font-medium text-ink-2">
          {t("portfolio.priceSource")}
          <select
            data-testid="trade-price-source"
            value={priceSource}
            onChange={(event) => setPriceSource(event.target.value as "venue" | "user")}
            className="rounded-xl border border-border bg-surface px-3 py-2 text-small"
          >
            <option value="venue">{t("portfolio.priceSourceVenue")}</option>
            <option value="user">{t("portfolio.priceSourceUser")}</option>
          </select>
        </label>
        <label className="flex flex-col gap-1 text-small font-medium text-ink-2">
          {t("portfolio.executedAt")}
          <input
            data-testid="trade-executed-at"
            type="datetime-local"
            value={executedAt}
            onChange={(event) => setExecutedAt(event.target.value)}
            className="rounded-xl border border-border bg-surface px-3 py-2 text-small"
          />
        </label>
        <div className="sm:col-span-2">
          <button
            type="submit"
            data-testid="trade-submit"
            disabled={busy}
            className="rounded-lg bg-accent px-4 py-2 text-small font-medium text-accent-on disabled:opacity-45"
          >
            {busy ? t("common.loading") : t("portfolio.trades")}
          </button>
        </div>
      </form>

      {errors.length > 0 ? (
        <ul data-testid="trade-errors" role="alert" className="mt-3 text-small text-danger">
          {errors.map((message) => (
            <li key={message}>{message}</li>
          ))}
        </ul>
      ) : null}
      {duplicate ? (
        <p data-testid="trade-duplicate" className="mt-2 text-small text-warn">
          {t("portfolio.duplicateTrade")}
        </p>
      ) : null}
      {saved ? (
        <p data-testid="trade-saved" role="status" className="mt-2 text-small text-success">
          {t("common.save")}
        </p>
      ) : null}
    </section>
  );
}
