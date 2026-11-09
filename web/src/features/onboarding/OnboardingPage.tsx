import { useState } from "react";
import { useNavigate } from "react-router-dom";
import { gatewayClient } from "../../api/client";
import { problemMessages } from "../../api/errors";
import type { ConsentDecision, Preferences } from "../../api/types";
import { useI18n } from "../../i18n/react";

const PURPOSES = ["analytics", "marketing"] as const;

interface ConsentRow {
  granted: boolean;
  legalBasis: string;
}

export default function OnboardingPage({
  postConsent = (decisions: ConsentDecision[]) => gatewayClient.postConsent(decisions),
  updatePreferences = (preferences: Preferences) =>
    gatewayClient.updatePreferences(preferences),
  addWallet = (input: { venue: string; label: string; address: string }) =>
    gatewayClient.addWallet(input),
}: {
  postConsent?: (decisions: ConsentDecision[]) => Promise<void>;
  updatePreferences?: (preferences: Preferences) => Promise<Preferences>;
  addWallet?: (input: { venue: string; label: string; address: string }) => Promise<unknown>;
} = {}) {
  const { t } = useI18n();
  const navigate = useNavigate();
  const [step, setStep] = useState<"consent" | "preferences" | "account">("consent");
  const [consent, setConsent] = useState<Record<string, ConsentRow>>({
    analytics: { granted: false, legalBasis: "" },
    marketing: { granted: false, legalBasis: "" },
  });
  const [preferences, setPreferences] = useState<Preferences>({
    baseCurrency: "USD",
    locale: "en",
    timezone: Intl.DateTimeFormat().resolvedOptions().timeZone || "UTC",
  });
  const [label, setLabel] = useState("My wallet");
  const [network, setNetwork] = useState("ethereum");
  const [address, setAddress] = useState("");
  const [errors, setErrors] = useState<string[]>([]);
  const [saving, setSaving] = useState(false);

  async function saveConsent() {
    setErrors([]);
    const missingBasis = PURPOSES.filter(
      (purpose) => consent[purpose].granted && !consent[purpose].legalBasis,
    );
    if (missingBasis.length > 0) {
      setErrors([t("onboarding.legalBasis")]);
      return;
    }
    setSaving(true);
    try {
      await postConsent(
        PURPOSES.map((purpose) => ({
          purpose,
          granted: consent[purpose].granted,
          legalBasis: consent[purpose].legalBasis || "consent",
        })),
      );
      setStep("preferences");
    } catch (error) {
      setErrors(problemMessages(error));
    } finally {
      setSaving(false);
    }
  }

  async function savePreferences() {
    setErrors([]);
    setSaving(true);
    try {
      await updatePreferences(preferences);
      setStep("account");
    } catch (error) {
      setErrors(problemMessages(error));
    } finally {
      setSaving(false);
    }
  }

  async function saveAccount() {
    setErrors([]);
    if (!address.trim() || !label.trim()) {
      setErrors([t("common.required")]);
      return;
    }
    setSaving(true);
    try {
      await addWallet({ venue: network, label, address });
      navigate("/dashboard");
    } catch (error) {
      setErrors(problemMessages(error));
    } finally {
      setSaving(false);
    }
  }

  return (
    <section className="mx-auto max-w-2xl">
      <p className="text-overline uppercase tracking-overline text-meta">
        {t("onboarding.title")}
      </p>
      <h1 className="mt-1 font-display text-section font-medium text-ink">
        {step === "consent"
          ? t("onboarding.consent")
          : step === "preferences"
            ? t("onboarding.preferences")
            : t("onboarding.firstAccount")}
      </h1>

      {errors.length > 0 ? (
        <ul data-testid="onboarding-errors" role="alert" className="mt-3 text-small text-danger">
          {errors.map((message) => (
            <li key={message}>{message}</li>
          ))}
        </ul>
      ) : null}

      {step === "consent" ? (
        <div data-testid="onboarding-consent" className="mt-4 grid gap-4">
          <p className="text-small text-muted">{t("onboarding.legalBasis")}</p>
          {PURPOSES.map((purpose) => (
            <fieldset key={purpose} className="rounded-lg border border-border bg-surface p-4">
              <legend className="px-1 text-small font-medium text-ink">
                {purpose}
              </legend>
              <label className="mt-2 flex items-center gap-2 text-small text-ink-2">
                <input
                  type="checkbox"
                  data-testid={`consent-${purpose}`}
                  checked={consent[purpose].granted}
                  onChange={(event) =>
                    setConsent({
                      ...consent,
                      [purpose]: { ...consent[purpose], granted: event.target.checked },
                    })
                  }
                />
                {t("onboarding.consent")}
              </label>
              <label className="mt-2 flex flex-col gap-1 text-small text-ink-2">
                {t("onboarding.legalBasis")}
                <select
                  data-testid={`consent-${purpose}-basis`}
                  value={consent[purpose].legalBasis}
                  onChange={(event) =>
                    setConsent({
                      ...consent,
                      [purpose]: { ...consent[purpose], legalBasis: event.target.value },
                    })
                  }
                  className="rounded-xl border border-border bg-surface px-3 py-2 text-small"
                >
                  <option value="">—</option>
                  <option value="consent">Consent</option>
                  <option value="contract">Contract</option>
                  <option value="legitimate_interest">Legitimate interest</option>
                </select>
              </label>
            </fieldset>
          ))}
          <button
            type="button"
            data-testid="onboarding-consent-next"
            disabled={saving}
            onClick={() => void saveConsent()}
            className="w-fit rounded-lg bg-accent px-4 py-2 text-small font-medium text-accent-on disabled:opacity-45"
          >
            {t("common.save")}
          </button>
        </div>
      ) : null}

      {step === "preferences" ? (
        <div data-testid="onboarding-preferences" className="mt-4 grid gap-3">
          <label className="flex flex-col gap-1 text-small font-medium text-ink-2">
            {t("onboarding.baseCurrency")}
            <input
              data-testid="preferences-currency"
              value={preferences.baseCurrency}
              onChange={(event) =>
                setPreferences({ ...preferences, baseCurrency: event.target.value })
              }
              className="rounded-xl border border-border bg-surface px-3 py-2 text-small"
            />
          </label>
          <label className="flex flex-col gap-1 text-small font-medium text-ink-2">
            {t("onboarding.locale")}
            <select
              data-testid="preferences-locale"
              value={preferences.locale}
              onChange={(event) =>
                setPreferences({
                  ...preferences,
                  locale: event.target.value as Preferences["locale"],
                })
              }
              className="rounded-xl border border-border bg-surface px-3 py-2 text-small"
            >
              <option value="en">English</option>
              <option value="vi">Tiếng Việt</option>
            </select>
          </label>
          <label className="flex flex-col gap-1 text-small font-medium text-ink-2">
            {t("onboarding.timezone")}
            <input
              data-testid="preferences-timezone"
              value={preferences.timezone}
              onChange={(event) =>
                setPreferences({ ...preferences, timezone: event.target.value })
              }
              className="rounded-xl border border-border bg-surface px-3 py-2 text-small"
            />
          </label>
          <button
            type="button"
            data-testid="onboarding-preferences-next"
            disabled={saving}
            onClick={() => void savePreferences()}
            className="w-fit rounded-lg bg-accent px-4 py-2 text-small font-medium text-accent-on disabled:opacity-45"
          >
            {t("common.save")}
          </button>
        </div>
      ) : null}

      {step === "account" ? (
        <div data-testid="onboarding-account" className="mt-4 grid gap-3">
          <label className="flex flex-col gap-1 text-small font-medium text-ink-2">
            {t("connect.label")}
            <input
              data-testid="onboarding-label"
              value={label}
              onChange={(event) => setLabel(event.target.value)}
              className="rounded-xl border border-border bg-surface px-3 py-2 text-small"
            />
          </label>
          <label className="flex flex-col gap-1 text-small font-medium text-ink-2">
            {t("connect.network")}
            <select
              data-testid="onboarding-network"
              value={network}
              onChange={(event) => setNetwork(event.target.value)}
              className="rounded-xl border border-border bg-surface px-3 py-2 text-small"
            >
              <option value="ethereum">Ethereum</option>
              <option value="bnb-smart-chain">BNB Smart Chain</option>
              <option value="solana">Solana</option>
            </select>
          </label>
          <label className="flex flex-col gap-1 text-small font-medium text-ink-2">
            {t("connect.address")}
            <input
              data-testid="onboarding-address"
              value={address}
              onChange={(event) => setAddress(event.target.value)}
              className="rounded-xl border border-border bg-surface px-3 py-2 font-mono text-small"
            />
          </label>
          <button
            type="button"
            data-testid="onboarding-account-submit"
            disabled={saving}
            onClick={() => void saveAccount()}
            className="w-fit rounded-lg bg-accent px-4 py-2 text-small font-medium text-accent-on disabled:opacity-45"
          >
            {t("common.save")}
          </button>
        </div>
      ) : null}
    </section>
  );
}
