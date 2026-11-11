/**
 * The terminal blocked-jurisdiction notice (functional-spec §2.5;
 * frontend-components §2; AC6.3.1, AC6.3.2).
 *
 * The sign-up variant states that the service is unavailable in the visitor's
 * jurisdiction and that no personal data has been stored; the continued-use
 * variant replaces that sentence with the temporarily-unavailable line. Both
 * are page-level statuses and offer Sign out only — there is deliberately no
 * retry, resubmit, or country re-selection, and `restrictedBy` is never
 * surfaced.
 *
 * Requirements: FR19.4, AC6.3.1, AC6.3.2, NFR25.1, NFR25.2, NFR26.1.
 */

import { useI18n } from "../../i18n/react";

export interface BlockedJurisdictionNoticeProps {
  continuedUse: boolean;
  onSignOut: () => void;
}

export function BlockedJurisdictionNotice({
  continuedUse,
  onSignOut,
}: BlockedJurisdictionNoticeProps) {
  const { t } = useI18n();

  return (
    <section
      data-testid="gates-blocked-notice"
      role="status"
      aria-live="polite"
      className="mx-auto max-w-xl rounded-lg border border-warn bg-surface p-6"
    >
      <h1 className="text-section text-ink">{t("gates.blocked.title")}</h1>
      <p className="mt-3 text-small text-ink-2">
        {t(continuedUse ? "gates.blocked.continuedUseBody" : "gates.blocked.signupBody")}
      </p>
      {continuedUse ? null : (
        <p className="mt-2 text-small text-ink-2">{t("gates.blocked.notStored")}</p>
      )}
      <p className="mt-3 text-small">
        <a href={t("gates.blocked.supportHref")} className="underline text-accent">
          {t("gates.blocked.support")}
        </a>
      </p>
      <button
        type="button"
        data-testid="gates-signout"
        onClick={onSignOut}
        className="mt-6 rounded-xl bg-accent px-4 py-2 text-small font-medium text-accent-on"
      >
        {t("gates.blocked.signOut")}
      </button>
    </section>
  );
}
