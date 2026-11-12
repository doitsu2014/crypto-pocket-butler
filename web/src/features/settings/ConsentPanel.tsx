import { useState } from "react";
import { Loading, PageHeader } from "../../components";
import { useI18n } from "../../i18n/react";
import {
  useConsentHistory,
  useConsentState,
  useGrantConsent,
  useWithdrawConsent,
} from "../../api/settingsQueries";
import type { ConsentEvent, ConsentHistoryView, ConsentStateView } from "../../api/types";
import { correlationOf, isConflict, problemMessage } from "./problemText";

/**
 * The policy version the analytics switch acts on. The service validates no
 * version against a published policy yet (functional-spec §12, open item 3);
 * this constant is the settings feature's own copy of the notice it links to.
 */
export const ANALYTICS_POLICY_VERSION = "2026-11-01.1";

const REQUIRED_KINDS = ["terms", "risk", "age"] as const;

function latestEvent(
  history: ConsentHistoryView | undefined,
  kind: string,
): ConsentEvent | undefined {
  return history?.events.find((event) => event.kind === kind);
}

function latestGrantedAt(history: ConsentHistoryView | undefined, kind: string): string | null {
  const event = history?.events.find(
    (candidate) => candidate.kind === kind && candidate.action === "granted",
  );
  return event?.occurredAt ?? null;
}

function analyticsGranted(state: ConsentStateView | undefined): boolean {
  return state?.kinds.analytics === "granted";
}

/**
 * Consent panel (functional-spec §3.3, X3–X6). Required acceptances are
 * read-only and dated; analytics is the one switch. Every mutation re-reads
 * the server state instead of flipping the switch optimistically.
 */
export default function ConsentPanel() {
  const { t, format } = useI18n();
  const stateQuery = useConsentState();
  const historyQuery = useConsentHistory();
  const grant = useGrantConsent();
  const withdraw = useWithdrawConsent();
  const [error, setError] = useState<unknown>(null);
  const [hint, setHint] = useState<string | null>(null);

  if (stateQuery.isLoading || historyQuery.isLoading) {
    return <Loading />;
  }

  if (stateQuery.isError || historyQuery.isError) {
    return (
      <section role="alert" data-testid="consent-error" className="text-small text-danger">
        <p>{problemMessage(stateQuery.error ?? historyQuery.error, t)}</p>
        <button
          type="button"
          data-testid="consent-retry"
          onClick={() => {
            void stateQuery.refetch();
            void historyQuery.refetch();
          }}
          className="mt-2 rounded-lg bg-surface-warm px-3 py-1 font-medium text-ink"
        >
          {t("common.retry")}
        </button>
      </section>
    );
  }

  const state = stateQuery.data;
  const history = historyQuery.data;
  if (!state || !history) {
    return <Loading />;
  }
  const granted = analyticsGranted(state);
  const pending = grant.isPending || withdraw.isPending;
  const analyticsEvent = latestEvent(history, "analytics");
  const withdrawnAt = analyticsEvent?.action === "withdrawn" ? analyticsEvent.occurredAt : null;

  async function toggleAnalytics() {
    setError(null);
    setHint(null);
    try {
      if (granted) {
        const decision = await withdraw.mutateAsync("analytics");
        if (decision.changed === false) {
          setHint(t("settings.consent.nothingChanged"));
        }
      } else {
        await grant.mutateAsync({
          kind: "analytics",
          policyVersion: ANALYTICS_POLICY_VERSION,
        });
      }
    } catch (caught) {
      if (isConflict(caught)) {
        // The server is the truth: a 409 means the switch was stale, so re-read
        // both answers instead of showing a failure.
        await Promise.all([stateQuery.refetch(), historyQuery.refetch()]);
        return;
      }
      setError(caught);
    }
  }

  return (
    <section className="grid gap-8">
      <PageHeader title={t("settings.consent.title")} />

      <div className="grid gap-3">
        <h2 className="font-display text-section font-medium text-ink">
          {t("settings.consent.required")}
        </h2>
        {REQUIRED_KINDS.map((kind) => {
          const grantedAt = latestGrantedAt(history, kind);
          return (
            <div
              key={kind}
              data-testid={`consent-kind-${kind}`}
              className="grid gap-1 rounded-lg border border-border bg-surface p-3"
            >
              <span className="text-small font-medium text-ink">
                {t(`settings.consent.kind.${kind}`)}
              </span>
              <span className="text-small text-ink-2">
                {state.kinds[kind] === "granted"
                  ? t("settings.consent.state.granted")
                  : t("settings.consent.state.missing")}
                {" · "}
                {grantedAt ? format(grantedAt) : "—"}
              </span>
            </div>
          );
        })}
      </div>

      <div className="grid gap-2">
        <h2 className="font-display text-section font-medium text-ink">
          {t("settings.consent.analytics")}
        </h2>
        <div className="flex items-center gap-3">
          <button
            type="button"
            role="switch"
            aria-checked={granted}
            aria-label={t("settings.consent.analytics")}
            data-testid="consent-analytics"
            disabled={pending}
            onClick={() => void toggleAnalytics()}
            className={`relative h-5 w-9 rounded-full border border-border ${
              granted ? "bg-accent" : "bg-surface-warm"
            } disabled:opacity-45`}
          >
            <span
              aria-hidden="true"
              className={`absolute top-0.5 h-3.5 w-3.5 rounded-full bg-surface ${
                granted ? "right-0.5" : "left-0.5"
              }`}
            />
          </button>
          <span className="text-small text-ink">
            {t(`settings.consent.state.${granted ? "granted" : "withdrawn"}`)}
          </span>
        </div>
        {withdrawnAt ? (
          <p data-testid="consent-analytics-withdrawn" className="text-small text-ink-2">
            {t("settings.consent.withdrawnOn", { date: format(withdrawnAt) })}
          </p>
        ) : null}
        {hint ? (
          <p data-testid="consent-analytics-hint" role="status" className="text-small text-muted">
            {hint}
          </p>
        ) : null}
        {error ? (
          <p role="alert" data-testid="consent-analytics-error" className="text-small text-danger">
            {problemMessage(error, t)}
            {correlationOf(error) ? (
              <span className="ml-2 text-meta">
                {t("common.correlationId")}: {correlationOf(error)}
              </span>
            ) : null}
          </p>
        ) : null}
      </div>
    </section>
  );
}
