/**
 * The full-page retry for a failed entry read (functional-spec §2.1; NFR25.1).
 *
 * Deliberately its own component rather than the shared `ErrorPanel`: on an
 * unresolved gate state nothing may render that could be mistaken for the app,
 * and the retry is manual.
 *
 * Requirements: FR19.2, NFR2.6, NFR25.1.
 */

import { useI18n } from "../../i18n/react";

export function GateFullPageRetry({ onRetry }: { onRetry: () => void }) {
  const { t } = useI18n();

  return (
    <section
      data-testid="gates-entry-error"
      role="alert"
      className="mx-auto max-w-xl rounded-lg border border-danger bg-surface p-6"
    >
      <h1 className="text-section text-ink">{t("gates.entryError.title")}</h1>
      <p className="mt-2 text-small text-ink-2">{t("gates.entryError.body")}</p>
      <button
        type="button"
        data-testid="gates-fullpage-retry"
        onClick={onRetry}
        className="mt-4 rounded-xl bg-accent px-4 py-2 text-small font-medium text-accent-on"
      >
        {t("gates.retry")}
      </button>
    </section>
  );
}
