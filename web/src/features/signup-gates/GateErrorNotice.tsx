/**
 * The inline retryable gate error (functional-spec §2.6;
 * frontend-components §2; NFR25).
 *
 * `nothing-saved` is used only where nothing can have committed (C8 503 or a
 * failure before any write); `incomplete` is used when the record and/or some
 * consents may exist, so it never claims nothing was saved. Both display the
 * correlation id for support, keep a manual retry control, and render no
 * ProblemDetails `detail` text.
 *
 * Requirements: FR19.2, NFR25.1, NFR25.2.
 */

import { useI18n } from "../../i18n/react";

export interface GateErrorNoticeProps {
  variant: "nothing-saved" | "incomplete";
  correlationId?: string;
  onRetry: () => void;
}

export function GateErrorNotice({ variant, correlationId, onRetry }: GateErrorNoticeProps) {
  const { t } = useI18n();

  return (
    <div
      data-testid="gates-error-notice"
      role="alert"
      className="rounded-lg border border-danger bg-surface p-4"
    >
      <p className="flex items-start gap-2 text-small font-medium text-danger">
        <span
          aria-hidden="true"
          className="mt-0.5 inline-flex h-4 w-4 items-center justify-center rounded-full border border-danger text-overline"
        >
          !
        </span>
        <span>
          {t(variant === "nothing-saved" ? "gates.error.nothingSaved" : "gates.error.incomplete")}
        </span>
      </p>
      {correlationId ? (
        <p className="mt-2 text-small text-muted">
          {t("gates.error.correlation", { correlationId })}
        </p>
      ) : null}
      <button
        type="button"
        data-testid="gates-error-retry"
        onClick={onRetry}
        className="mt-3 rounded-xl border border-border bg-surface px-4 py-2 text-small font-medium text-ink"
      >
        {t("gates.error.retry")}
      </button>
    </div>
  );
}
