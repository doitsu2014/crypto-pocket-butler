import { useState, type FormEvent } from "react";
import { gatewayClient } from "../../api/client";
import { fieldError, problemMessages } from "../../api/errors";
import type { Account } from "../../api/types";
import { useI18n } from "../../i18n/react";

export default function ExchangeCredentialForm({
  addAccount = (input: {
    venue: "binance" | "okx";
    label: string;
    apiKey: string;
    apiSecret: string;
    passphrase?: string;
  }) => gatewayClient.addExchangeAccount(input),
  verify = (accountId: string) => gatewayClient.verifyAccount(accountId),
  onAdded,
}: {
  addAccount?: (input: {
    venue: "binance" | "okx";
    label: string;
    apiKey: string;
    apiSecret: string;
    passphrase?: string;
  }) => Promise<Account>;
  verify?: (accountId: string) => Promise<{ status: Account["status"] }>;
  onAdded?: (account: Account) => void;
} = {}) {
  const { t } = useI18n();
  const [venue, setVenue] = useState<"binance" | "okx">("binance");
  const [label, setLabel] = useState("");
  const [apiKey, setApiKey] = useState("");
  const [apiSecret, setApiSecret] = useState("");
  const [passphrase, setPassphrase] = useState("");
  const [errors, setErrors] = useState<string[]>([]);
  const [verificationError, setVerificationError] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);

  async function onSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setErrors([]);
    setVerificationError(null);
    const missing: string[] = [];
    if (!label.trim()) missing.push(`${t("connect.label")}: ${t("common.required")}`);
    if (!apiKey.trim()) missing.push(`${t("connect.apiKey")}: ${t("common.required")}`);
    if (!apiSecret.trim()) missing.push(`${t("connect.apiSecret")}: ${t("common.required")}`);
    if (missing.length > 0) {
      setErrors(missing);
      return;
    }
    setSaving(true);
    try {
      const account = await addAccount({
        venue,
        label,
        apiKey,
        apiSecret,
        passphrase: passphrase.trim() || undefined,
      });
      try {
        await verify(account.id);
      } catch (verifyError) {
        const field = fieldError(verifyError, "permissions");
        setVerificationError(
          field ?? problemMessages(verifyError).join(" · "),
        );
        setSaving(false);
        return;
      }
      // Credentials are never cached client-side; clear the secret fields.
      setApiKey("");
      setApiSecret("");
      setPassphrase("");
      onAdded?.(account);
    } catch (error) {
      setErrors(problemMessages(error));
    } finally {
      setSaving(false);
    }
  }

  return (
    <form data-testid="exchange-form" onSubmit={onSubmit} className="grid gap-3" noValidate>
      <label className="flex flex-col gap-1 text-small font-medium text-ink-2">
        {t("connect.venue")}
        <select
          data-testid="exchange-venue"
          value={venue}
          onChange={(event) => setVenue(event.target.value as "binance" | "okx")}
          className="rounded-xl border border-border bg-surface px-3 py-2 text-small"
        >
          <option value="binance">Binance</option>
          <option value="okx">OKX</option>
        </select>
      </label>
      <label className="flex flex-col gap-1 text-small font-medium text-ink-2">
        {t("connect.label")}
        <input
          data-testid="exchange-label"
          value={label}
          onChange={(event) => setLabel(event.target.value)}
          className="rounded-xl border border-border bg-surface px-3 py-2 text-small"
        />
      </label>
      <label className="flex flex-col gap-1 text-small font-medium text-ink-2">
        {t("connect.apiKey")}
        <input
          data-testid="exchange-api-key"
          autoComplete="off"
          value={apiKey}
          onChange={(event) => setApiKey(event.target.value)}
          className="rounded-xl border border-border bg-surface px-3 py-2 font-mono text-small"
        />
      </label>
      <label className="flex flex-col gap-1 text-small font-medium text-ink-2">
        {t("connect.apiSecret")}
        <input
          data-testid="exchange-api-secret"
          type="password"
          autoComplete="off"
          value={apiSecret}
          onChange={(event) => setApiSecret(event.target.value)}
          className="rounded-xl border border-border bg-surface px-3 py-2 font-mono text-small"
        />
      </label>
      <label className="flex flex-col gap-1 text-small font-medium text-ink-2">
        {t("connect.passphrase")}
        <input
          data-testid="exchange-passphrase"
          type="password"
          autoComplete="off"
          value={passphrase}
          onChange={(event) => setPassphrase(event.target.value)}
          className="rounded-xl border border-border bg-surface px-3 py-2 font-mono text-small"
        />
      </label>

      {errors.length > 0 ? (
        <ul data-testid="exchange-errors" role="alert" className="text-small text-danger">
          {errors.map((message) => (
            <li key={message}>{message}</li>
          ))}
        </ul>
      ) : null}
      {verificationError ? (
        <p data-testid="exchange-verification-error" role="alert" className="text-small text-danger">
          {verificationError}
        </p>
      ) : null}

      <button
        type="submit"
        data-testid="exchange-submit"
        disabled={saving}
        className="w-fit rounded-lg bg-accent px-4 py-2 text-small font-medium text-accent-on disabled:opacity-45"
      >
        {saving ? t("common.loading") : t("connect.verify")}
      </button>
    </form>
  );
}
