import { useState, type FormEvent } from "react";
import { gatewayClient, isUnimplemented } from "../../api/client";
import { problemMessages } from "../../api/errors";
import { usePnl, useProfile, useTransactions } from "../../api/queries";
import type { Pnl, Transaction } from "../../api/types";
import {
  BlockingState,
  DataTable,
  ErrorPanel,
  Loading,
  Money,
  PageHeader,
  PnlFigures,
} from "../../components";
import { useI18n } from "../../i18n/react";

export default function ActivityPage({
  userId: userIdProp,
  transactionsOverride,
  pnlOverride,
  correct = (transactionId: string, correction: { quantity?: string; price?: string }) =>
    gatewayClient.correctTransaction(transactionId, correction),
}: {
  userId?: string;
  transactionsOverride?: Transaction[];
  pnlOverride?: Pnl;
  correct?: (
    transactionId: string,
    correction: { quantity?: string; price?: string },
  ) => Promise<Transaction>;
} = {}) {
  const { t, format } = useI18n();
  const profile = useProfile(!userIdProp);
  const userId = userIdProp ?? profile.data?.userId;
  const transactions = useTransactions(transactionsOverride ? undefined : userId);
  const pnl = usePnl(pnlOverride ? undefined : userId);
  const [selected, setSelected] = useState("");
  const [quantity, setQuantity] = useState("");
  const [price, setPrice] = useState("");
  const [errors, setErrors] = useState<string[]>([]);
  const [saved, setSaved] = useState(false);

  const rows = transactionsOverride ?? transactions.data ?? [];
  const pnlData = pnlOverride ?? pnl.data;

  async function onSubmitCorrection(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setErrors([]);
    setSaved(false);
    if (!selected) {
      setErrors([t("common.required")]);
      return;
    }
    try {
      await correct(selected, {
        quantity: quantity || undefined,
        price: price || undefined,
      });
      setSaved(true);
      setSelected("");
      setQuantity("");
      setPrice("");
    } catch (error) {
      setErrors(problemMessages(error));
    }
  }

  return (
    <section className="grid gap-6">
      <PageHeader title={t("activity.title")} />

      <div>
        <h2 className="mb-2 font-display text-section font-medium text-ink">
          {t("activity.transactions")}
        </h2>
        {transactions.isLoading ? (
          <Loading />
        ) : transactions.error ? (
          isUnimplemented(transactions.error) ? (
            <BlockingState operation="GET /v1/users/{userId}/transactions" unit="U7" />
          ) : (
            <ErrorPanel error={transactions.error} onRetry={() => void transactions.refetch()} />
          )
        ) : rows.length === 0 ? (
          <p data-testid="transactions-empty" className="text-small text-muted">
            {t("activity.noTransactions")}
          </p>
        ) : (
          <DataTable
            caption={t("activity.transactions")}
            columns={[
              { key: "asset", header: t("portfolio.asset") },
              { key: "kind", header: t("portfolio.side") },
              { key: "quantity", header: t("portfolio.quantity"), numeric: true },
              { key: "price", header: t("portfolio.price"), numeric: true },
              { key: "source", header: t("portfolio.priceSource") },
              { key: "executedAt", header: t("portfolio.executedAt") },
            ]}
            rows={rows.map((transaction) => ({
              id: transaction.id,
              asset: transaction.symbol ?? transaction.assetId,
              kind: transaction.kind,
              quantity: transaction.quantity,
              price:
                transaction.priceMissing || transaction.price === null ? (
                  <span className="text-warn">{t("activity.priceMissing")}</span>
                ) : (
                  <Money value={transaction.price} />
                ),
              source: transaction.source,
              executedAt: format(transaction.executedAt),
            }))}
          />
        )}
      </div>

      <div>
        <h2 className="mb-2 font-display text-section font-medium text-ink">{t("activity.pnl")}</h2>
        {pnl.isLoading && !pnlOverride ? (
          <Loading />
        ) : pnl.error ? (
          isUnimplemented(pnl.error) ? (
            <BlockingState operation="GET /v1/users/{userId}/pnl" unit="U7" />
          ) : (
            <ErrorPanel error={pnl.error} onRetry={() => void pnl.refetch()} />
          )
        ) : pnlData ? (
          <PnlFigures pnl={pnlData} />
        ) : null}
      </div>

      <div>
        <h2 className="mb-2 font-display text-section font-medium text-ink">
          {t("activity.correction")}
        </h2>
        <form
          data-testid="correction-form"
          onSubmit={onSubmitCorrection}
          className="grid gap-3 rounded-lg border border-border bg-surface p-4 sm:grid-cols-3"
        >
          <label className="flex flex-col gap-1 text-small font-medium text-ink-2">
            {t("activity.transactions")}
            <select
              data-testid="correction-transaction"
              value={selected}
              onChange={(event) => setSelected(event.target.value)}
              className="rounded-xl border border-border bg-surface px-3 py-2 text-small"
            >
              <option value="">—</option>
              {rows.map((transaction) => (
                <option key={transaction.id} value={transaction.id}>
                  {transaction.id} · {transaction.assetId}
                </option>
              ))}
            </select>
          </label>
          <label className="flex flex-col gap-1 text-small font-medium text-ink-2">
            {t("portfolio.quantity")}
            <input
              data-testid="correction-quantity"
              type="number"
              step="any"
              value={quantity}
              onChange={(event) => setQuantity(event.target.value)}
              className="rounded-xl border border-border bg-surface px-3 py-2 font-mono text-small"
            />
          </label>
          <label className="flex flex-col gap-1 text-small font-medium text-ink-2">
            {t("portfolio.price")}
            <input
              data-testid="correction-price"
              type="number"
              step="any"
              value={price}
              onChange={(event) => setPrice(event.target.value)}
              className="rounded-xl border border-border bg-surface px-3 py-2 font-mono text-small"
            />
          </label>
          <div className="sm:col-span-3">
            <button
              type="submit"
              data-testid="correction-submit"
              className="rounded-lg bg-accent px-4 py-2 text-small font-medium text-accent-on"
            >
              {t("activity.correction")}
            </button>
          </div>
        </form>

        {errors.length > 0 ? (
          <ul data-testid="correction-errors" role="alert" className="mt-3 text-small text-danger">
            {errors.map((message) => (
              <li key={message}>{message}</li>
            ))}
          </ul>
        ) : null}
        {saved ? (
          <p data-testid="correction-saved" role="status" className="mt-2 text-small text-success">
            {t("common.save")}
          </p>
        ) : null}
      </div>
    </section>
  );
}
