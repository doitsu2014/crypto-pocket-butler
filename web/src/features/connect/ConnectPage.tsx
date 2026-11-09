import { useState } from "react";
import { useAccounts, useProfile } from "../../api/queries";
import { useLimitedSession } from "../../auth/limited";
import type { Account } from "../../api/types";
import { useI18n } from "../../i18n/react";
import { PageHeader, QueryBoundary } from "../../components";
import AccountStatusRow from "./AccountStatusRow";
import CsvImport from "./CsvImport";
import ExchangeCredentialForm from "./ExchangeCredentialForm";
import WalletAddressForm from "./WalletAddressForm";
import WalletConnectButton from "./WalletConnectButton";

const TABS = ["exchange", "wallet", "walletconnect", "csv"] as const;
type Tab = (typeof TABS)[number];

const TAB_KEY: Record<Tab, string> = {
  exchange: "connect.exchange",
  wallet: "connect.wallet",
  walletconnect: "connect.walletconnect",
  csv: "connect.csv",
};

export default function ConnectPage({
  userId: userIdProp,
  accountsOverride,
}: {
  userId?: string;
  accountsOverride?: Account[];
} = {}) {
  const { t } = useI18n();
  const { limited } = useLimitedSession();
  const profile = useProfile(!userIdProp);
  const userId = userIdProp ?? profile.data?.userId;
  const accountsQuery = useAccounts(accountsOverride ? undefined : userId);
  const [tab, setTab] = useState<Tab>("exchange");
  const [added, setAdded] = useState<Account[]>([]);

  const accounts = accountsOverride ?? accountsQuery.data ?? [];

  function onAdded(account: Account) {
    setAdded((current) => [account, ...current]);
  }

  return (
    <section>
      <PageHeader title={t("connect.title")} />

      {limited ? (
        <p role="status" className="mb-4 rounded-lg border border-warn bg-surface-warm p-3 text-small text-ink-2">
          {t("settings.mfaLimited")}
        </p>
      ) : null}

      <div role="tablist" aria-label={t("connect.title")} className="mb-4 flex flex-wrap gap-2">
        {TABS.map((entry) => (
          <button
            key={entry}
            type="button"
            role="tab"
            aria-selected={tab === entry}
            data-testid={`connect-tab-${entry}`}
            onClick={() => setTab(entry)}
            className={`rounded-lg px-3 py-1.5 text-small ${
              tab === entry ? "bg-accent text-accent-on" : "bg-surface-warm text-ink-2"
            }`}
          >
            {t(TAB_KEY[entry])}
          </button>
        ))}
      </div>

      <div role="tabpanel" className="rounded-lg border border-border bg-surface p-6">
        {limited ? (
          <p className="text-small text-muted">{t("settings.mfaLimited")}</p>
        ) : tab === "exchange" ? (
          <ExchangeCredentialForm onAdded={onAdded} />
        ) : tab === "wallet" ? (
          <WalletAddressForm onAdded={onAdded} />
        ) : tab === "walletconnect" ? (
          <WalletConnectButton onConnected={onAdded} />
        ) : (
          <CsvImport />
        )}
      </div>

      <h2 className="mt-8 font-display text-section font-medium text-ink">
        {t("portfolio.accounts")}
      </h2>
      <QueryBoundary
        loading={accountsQuery.isLoading}
        error={accountsQuery.error}
        onRetry={() => void accountsQuery.refetch()}
      >
        {[...added, ...accounts].length === 0 ? (
          <p data-testid="accounts-empty" className="mt-2 text-small text-muted">
            {t("common.empty")}
          </p>
        ) : (
          <div className="mt-3 grid gap-3">
            {[...added, ...accounts].map((account) => (
              <AccountStatusRow key={account.id} account={account} />
            ))}
          </div>
        )}
      </QueryBoundary>
    </section>
  );
}
