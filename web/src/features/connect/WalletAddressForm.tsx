import { useState, type FormEvent } from "react";
import { gatewayClient } from "../../api/client";
import { problemMessages } from "../../api/errors";
import type { Account } from "../../api/types";
import { useI18n } from "../../i18n/react";

export const SUPPORTED_NETWORKS = [
  { id: "ethereum", label: "Ethereum" },
  { id: "bnb-smart-chain", label: "BNB Smart Chain" },
  { id: "solana", label: "Solana" },
] as const;

const EVM = /^0x[a-fA-F0-9]{40}$/;
const SOLANA = /^[1-9A-HJ-NP-Za-km-z]{32,44}$/;

export function isValidAddress(network: string, address: string): boolean {
  if (network === "solana") {
    return SOLANA.test(address);
  }
  return EVM.test(address);
}

export default function WalletAddressForm({
  addWallet = (input: { venue: string; label: string; address: string }) =>
    gatewayClient.addWallet(input),
  onAdded,
}: {
  addWallet?: (input: { venue: string; label: string; address: string }) => Promise<Account>;
  onAdded?: (account: Account) => void;
} = {}) {
  const { t } = useI18n();
  const [network, setNetwork] = useState<string>("ethereum");
  const [label, setLabel] = useState("");
  const [address, setAddress] = useState("");
  const [errors, setErrors] = useState<string[]>([]);
  const [saving, setSaving] = useState(false);

  async function onSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setErrors([]);
    if (!isValidAddress(network, address.trim())) {
      setErrors([
        `${t("connect.address")}: ${SUPPORTED_NETWORKS.map((entry) => entry.label).join(", ")}`,
      ]);
      return;
    }
    setSaving(true);
    try {
      const account = await addWallet({
        venue: network,
        label: label.trim() || "My wallet",
        address: address.trim(),
      });
      onAdded?.(account);
    } catch (error) {
      setErrors(problemMessages(error));
    } finally {
      setSaving(false);
    }
  }

  return (
    <form data-testid="wallet-form" onSubmit={onSubmit} className="grid gap-3" noValidate>
      <label className="flex flex-col gap-1 text-small font-medium text-ink-2">
        {t("connect.network")}
        <select
          data-testid="wallet-network"
          value={network}
          onChange={(event) => setNetwork(event.target.value)}
          className="rounded-xl border border-border bg-surface px-3 py-2 text-small"
        >
          {SUPPORTED_NETWORKS.map((entry) => (
            <option key={entry.id} value={entry.id}>
              {entry.label}
            </option>
          ))}
        </select>
      </label>
      <label className="flex flex-col gap-1 text-small font-medium text-ink-2">
        {t("connect.label")}
        <input
          data-testid="wallet-label"
          value={label}
          onChange={(event) => setLabel(event.target.value)}
          className="rounded-xl border border-border bg-surface px-3 py-2 text-small"
        />
      </label>
      <label className="flex flex-col gap-1 text-small font-medium text-ink-2">
        {t("connect.address")}
        <input
          data-testid="wallet-address"
          value={address}
          onChange={(event) => setAddress(event.target.value)}
          autoComplete="off"
          spellCheck={false}
          className="rounded-xl border border-border bg-surface px-3 py-2 font-mono text-small"
        />
      </label>

      {errors.length > 0 ? (
        <ul data-testid="wallet-errors" role="alert" className="text-small text-danger">
          {errors.map((message) => (
            <li key={message}>{message}</li>
          ))}
        </ul>
      ) : null}

      <button
        type="submit"
        data-testid="wallet-submit"
        disabled={saving}
        className="w-fit rounded-lg bg-accent px-4 py-2 text-small font-medium text-accent-on disabled:opacity-45"
      >
        {saving ? t("common.loading") : t("common.save")}
      </button>
    </form>
  );
}
