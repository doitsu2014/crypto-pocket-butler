import { useState } from "react";
import { gatewayClient } from "../../api/client";
import type { Account } from "../../api/types";
import { useI18n } from "../../i18n/react";

const STATUS_KEY: Record<Account["status"], string> = {
  active: "connect.synced",
  synced: "connect.synced",
  syncing: "connect.syncing",
  failed: "connect.failed",
  error: "connect.failed",
  disconnected: "connect.failed",
};

const STATUS_SHAPE: Record<Account["status"], string> = {
  active: "●",
  synced: "●",
  syncing: "◐",
  failed: "○",
  error: "○",
  disconnected: "○",
};

const STATUS_TONE: Record<Account["status"], string> = {
  active: "text-success",
  synced: "text-success",
  syncing: "text-warn",
  failed: "text-danger",
  error: "text-danger",
  disconnected: "text-danger",
};

export default function AccountStatusRow({
  account,
  onResync = (accountId: string) => gatewayClient.resyncAccount(accountId),
}: {
  account: Account;
  onResync?: (accountId: string) => Promise<{ runId: string }>;
}) {
  const { t, format } = useI18n();
  const [state, setState] = useState<"idle" | "resyncing" | "failed">("idle");
  const [failure, setFailure] = useState<string | null>(account.lastError ?? null);

  async function resync() {
    setState("resyncing");
    setFailure(null);
    try {
      await onResync(account.id);
      // The labelled balances stay visible while the sync runs (AC2.3.3).
      setState("idle");
    } catch (error) {
      setState("failed");
      setFailure(error instanceof Error ? error.message : t("problem.server"));
    }
  }

  const shape = state === "resyncing" ? "◐" : STATUS_SHAPE[account.status];
  const tone = state === "resyncing" ? "text-warn" : STATUS_TONE[account.status];

  return (
    <article
      data-testid="account-status"
      className="rounded-lg border border-border bg-surface p-4"
    >
      <div className="flex flex-wrap items-center justify-between gap-2">
        <div>
          <h3 className="text-small font-medium text-ink">{account.label}</h3>
          <p className="text-small text-meta">{account.kind}</p>
        </div>
        <span className="inline-flex items-center gap-1 text-small">
          <span aria-hidden="true" className={tone}>
            {shape}
          </span>
          <span className="text-ink-2">
            {state === "resyncing"
              ? t("connect.syncing")
              : t(STATUS_KEY[account.status])}
          </span>
        </span>
      </div>
      <p className="mt-1 text-small text-meta">
        {t("connect.lastSynced")}:{" "}
        <span className="font-mono">{format(account.lastSyncedAt ?? null)}</span>
      </p>
      {failure ? (
        <p data-testid="account-failure" role="alert" className="mt-1 text-small text-danger">
          {failure}
        </p>
      ) : null}
      <button
        type="button"
        data-testid="account-resync"
        disabled={state === "resyncing"}
        onClick={() => void resync()}
        className="mt-2 rounded-lg bg-surface-warm px-3 py-1 text-small font-medium text-ink disabled:opacity-45"
      >
        {t("connect.resync")}
      </button>
    </article>
  );
}
