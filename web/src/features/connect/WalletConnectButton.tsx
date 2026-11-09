import { useState } from "react";
import { gatewayClient } from "../../api/client";
import type { Account } from "../../api/types";
import { useI18n } from "../../i18n/react";
import { connectWalletConnect, type WalletConnectSession } from "./walletconnect";

export default function WalletConnectButton({
  connect = connectWalletConnect,
  attach = (input: {
    venue: string;
    label: string;
    address: string;
    topic: string;
  }) => gatewayClient.addWalletConnect(input),
  onConnected,
}: {
  connect?: () => Promise<WalletConnectSession>;
  attach?: (input: {
    venue: string;
    label: string;
    address: string;
    topic: string;
  }) => Promise<Account>;
  onConnected?: (account: Account) => void;
} = {}) {
  const { t } = useI18n();
  const [state, setState] = useState<"idle" | "connecting" | "connected" | "error">(
    "idle",
  );
  const [error, setError] = useState<string | null>(null);

  async function onConnect() {
    setState("connecting");
    setError(null);
    try {
      const session = await connect();
      const account = await attach({
        venue: session.chain,
        label: "WalletConnect",
        address: session.address,
        topic: session.topic,
      });
      setState("connected");
      onConnected?.(account);
    } catch (caught) {
      setState("error");
      setError(caught instanceof Error ? caught.message : t("problem.server"));
    }
  }

  return (
    <section data-testid="walletconnect" className="grid gap-3">
      <p className="text-small text-muted">{t("connect.walletConnectReadOnly")}</p>
      <button
        type="button"
        data-testid="walletconnect-button"
        onClick={() => void onConnect()}
        disabled={state === "connecting"}
        className="w-fit rounded-lg bg-accent px-4 py-2 text-small font-medium text-accent-on disabled:opacity-45"
      >
        {state === "connecting" ? t("common.loading") : t("connect.walletconnect")}
      </button>
      {state === "connected" ? (
        <p data-testid="walletconnect-connected" role="status" className="text-small text-success">
          {t("connect.synced")}
        </p>
      ) : null}
      {state === "error" && error ? (
        <p data-testid="walletconnect-error" role="alert" className="text-small text-danger">
          {error} · {t("common.retry")}
        </p>
      ) : null}
    </section>
  );
}
