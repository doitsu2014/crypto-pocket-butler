import { useEffect, useRef } from "react";
import type { KeyboardEvent, ReactNode } from "react";
import { useI18n } from "../i18n/react";
import type {
  DataStatus,
  Freshness,
  LineDataStatus,
  Pnl,
  ProblemDetails,
  UserStatus,
} from "../api/types";
import { NetworkError, ApiError } from "../api/client";
import { localiseProblem } from "../i18n";

/** Missing financial values are an em dash, never 0 or a currency zero (BR2.1). */
export function Money({
  value,
  currency,
  className = "",
  testId = "money",
}: {
  value: string | null | undefined;
  currency?: string | null;
  className?: string;
  testId?: string;
}) {
  const missing = value === null || value === undefined || value === "";
  return (
    <span data-testid={testId} className={`font-mono tabular-nums ${className}`}>
      {missing ? "—" : value}
      {!missing && currency ? <span className="ml-1 text-meta">{currency}</span> : null}
    </span>
  );
}

export function Percent({
  value,
  className = "",
}: {
  value: number | null | undefined;
  className?: string;
}) {
  return (
    <span data-testid="percent" className={`font-mono tabular-nums ${className}`}>
      {value === null || value === undefined ? "—" : `${value}%`}
    </span>
  );
}

export function Points({
  value,
  className = "",
}: {
  value: number | null | undefined;
  className?: string;
}) {
  if (value === null || value === undefined) {
    return (
      <span data-testid="points" className={`font-mono tabular-nums ${className}`}>
        —
      </span>
    );
  }
  const sign = value > 0 ? "+" : value < 0 ? "−" : "±";
  return (
    <span data-testid="points" className={`font-mono tabular-nums ${className}`}>
      {sign}
      {Math.abs(value)} pp
    </span>
  );
}

const STATUS_LABEL: Record<string, string> = {
  complete: "status.complete",
  partial: "status.partial",
  unavailable: "status.unavailable",
  priced: "status.priced",
  unpriced: "status.unpriced",
  holdings_unavailable: "status.holdingsUnavailable",
};

const STATUS_SHAPE: Record<string, string> = {
  complete: "●",
  partial: "◐",
  unavailable: "○",
  priced: "●",
  unpriced: "◌",
  holdings_unavailable: "○",
};

const STATUS_TONE: Record<string, string> = {
  complete: "text-success",
  priced: "text-success",
  partial: "text-warn",
  unpriced: "text-warn",
  unavailable: "text-danger",
  holdings_unavailable: "text-danger",
};

/** A status always pairs a label and a shape with its colour (BR4.1). */
export function DataStatusBadge({
  status,
  asOf,
}: {
  status: DataStatus | LineDataStatus;
  asOf?: string | null;
}) {
  const { t, format } = useI18n();
  return (
    <span
      data-testid="data-status"
      className="inline-flex items-center gap-1 rounded-full border border-border bg-surface px-2 py-0.5 text-overline uppercase tracking-overline"
    >
      <span aria-hidden="true" className={STATUS_TONE[status]}>
        {STATUS_SHAPE[status]}
      </span>
      <span className="text-ink-2">{t(STATUS_LABEL[status] ?? status)}</span>
      {asOf ? <span className="text-meta">· {format(asOf)}</span> : null}
    </span>
  );
}

const FRESHNESS_SHAPE: Record<Freshness, string> = {
  live: "●",
  delayed: "◐",
  stale: "◔",
  unpriced: "◌",
};

const FRESHNESS_TONE: Record<Freshness, string> = {
  live: "text-success",
  delayed: "text-warn",
  stale: "text-warn",
  unpriced: "text-danger",
};

export function FreshnessBadge({
  freshness,
  observedAt,
}: {
  freshness: Freshness;
  observedAt?: string | null;
}) {
  const { t, format } = useI18n();
  return (
    <span data-testid="freshness" className="inline-flex items-center gap-1 text-small text-ink-2">
      <span aria-hidden="true" className={FRESHNESS_TONE[freshness]}>
        {FRESHNESS_SHAPE[freshness]}
      </span>
      <span>{t(`freshness.${freshness}`)}</span>
      {observedAt ? <span className="text-meta">{format(observedAt)}</span> : null}
    </span>
  );
}

const STATUS_PILL: Record<
  UserStatus,
  { label: string; shape: string; glyph: string; tone: string }
> = {
  pending: { label: "status.pending", shape: "ring", glyph: "○", tone: "text-warn" },
  active: { label: "status.active", shape: "dot", glyph: "●", tone: "text-success" },
  disabled: { label: "status.disabled", shape: "square", glyph: "■", tone: "text-muted" },
  pending_deletion: {
    label: "status.pendingDeletion",
    shape: "square-outline",
    glyph: "□",
    tone: "text-warn",
  },
  deleted: { label: "status.deleted", shape: "cross", glyph: "×", tone: "text-danger" },
};

/**
 * The account status as a word plus its own shape (design-system: colour never
 * carries meaning alone). A pending-deletion account also carries the exact
 * restore date as text beside the pill, never inside it.
 */
export function StatusPill({
  status,
  restoreDate,
  className = "",
}: {
  status: UserStatus;
  restoreDate?: string | null;
  className?: string;
}) {
  const { t, format } = useI18n();
  const meta = STATUS_PILL[status];
  return (
    <span className={`inline-flex flex-wrap items-center gap-2 ${className}`}>
      <span
        data-testid="status-pill"
        data-status={status}
        className="inline-flex items-center gap-1 rounded-full border border-border bg-surface px-2 py-0.5 text-overline uppercase tracking-overline"
      >
        <span
          data-testid="status-pill-shape"
          data-shape={meta.shape}
          aria-hidden="true"
          className={meta.tone}
        >
          {meta.glyph}
        </span>
        <span className="text-ink-2">{t(meta.label)}</span>
      </span>
      {status === "pending_deletion" && restoreDate ? (
        <span data-testid="status-pill-restore" className="text-small text-ink-2">
          {t("status.restoreOn", { date: format(restoreDate) })}
        </span>
      ) : null}
    </span>
  );
}

/** Names every coverage gap so a withheld figure is never silent (BR2.2, US4.3). */
export function CoverageNote({
  coverage,
  excludedCount,
  marksUnavailable,
  holdingsUnavailable,
  fxUnavailable,
  fxEstimated,
  gap,
}: {
  coverage?: number | null;
  excludedCount?: number | null;
  marksUnavailable?: boolean;
  holdingsUnavailable?: boolean;
  fxUnavailable?: boolean;
  fxEstimated?: boolean;
  gap?: string[];
}) {
  const { t } = useI18n();
  const labels: string[] = [];
  if (coverage !== null && coverage !== undefined) {
    labels.push(`${t("coverage.pricedCoverage")}: ${(coverage * 100).toFixed(0)}%`);
  }
  if (excludedCount) {
    labels.push(`${t("coverage.excluded")}: ${excludedCount}`);
  }
  if (marksUnavailable) labels.push(t("coverage.marksUnavailable"));
  if (holdingsUnavailable) labels.push(t("coverage.holdingsUnavailable"));
  if (fxUnavailable) labels.push(t("coverage.fxUnavailable"));
  if (fxEstimated) labels.push(t("coverage.fxEstimated"));
  if (gap && gap.length > 0) labels.push(`${t("coverage.excluded")}: ${gap.join(", ")}`);

  return (
    <p data-testid="coverage-note" className="text-small text-muted">
      {labels.length > 0 ? labels.join(" · ") : t("common.unavailable")}
    </p>
  );
}

export function PriceProvenance({
  source,
  observedAt,
  fxSource,
  fxObservedAt,
}: {
  source: string | null;
  observedAt: string | null;
  fxSource?: string | null;
  fxObservedAt?: string | null;
}) {
  const { t, format } = useI18n();
  return (
    <span data-testid="price-provenance" className="text-small text-meta">
      {source ? `${source} · ${format(observedAt)}` : t("common.unavailable")}
      {fxSource ? ` · FX ${fxSource} · ${format(fxObservedAt ?? null)}` : null}
    </span>
  );
}

export function AsOf({ at }: { at: string | null | undefined }) {
  const { t, format } = useI18n();
  return (
    <span data-testid="as-of" className="text-small text-meta">
      {t("common.asOf")} {at ? <time dateTime={at}>{format(at)}</time> : "—"}
    </span>
  );
}

export function ErrorPanel({
  error,
  problem,
  onRetry,
}: {
  error?: unknown;
  problem?: ProblemDetails | null;
  onRetry?: () => void;
}) {
  const { t } = useI18n();
  const resolved: ProblemDetails | null =
    problem ?? (error instanceof ApiError ? error.problem : null);
  const message =
    error instanceof NetworkError
      ? t("problem.network")
      : resolved
        ? localiseProblem(resolved, t)
        : t("problem.server");

  return (
    <section
      data-testid="error-panel"
      role="alert"
      aria-live="assertive"
      className="rounded-lg border border-danger bg-surface p-4"
    >
      <h3 className="font-display text-small font-medium text-ink">{message}</h3>
      {resolved?.detail ? <p className="mt-1 text-small text-ink-2">{resolved.detail}</p> : null}
      {resolved ? (
        <p className="mt-2 text-small text-meta">
          {t("common.correlationId")}{" "}
          <span data-testid="correlation-id" className="font-mono text-ink">
            {resolved.correlationId}
          </span>
        </p>
      ) : null}
      {onRetry ? (
        <button
          type="button"
          data-testid="error-retry"
          onClick={onRetry}
          className="mt-3 rounded-lg bg-accent px-3 py-1.5 text-small font-medium text-accent-on"
        >
          {t("common.retry")}
        </button>
      ) : null}
    </section>
  );
}

export function EmptyState({
  title,
  description,
  actionLabel,
  onAction,
}: {
  title: string;
  description?: string;
  actionLabel?: string;
  onAction?: () => void;
}) {
  return (
    <div
      data-testid="empty-state"
      className="rounded-lg border border-border bg-surface p-8 text-center"
    >
      <h2 className="font-display text-section font-medium text-ink">{title}</h2>
      {description ? <p className="mt-2 text-small text-muted">{description}</p> : null}
      {actionLabel && onAction ? (
        <button
          type="button"
          data-testid="empty-action"
          onClick={onAction}
          className="mt-4 rounded-lg bg-accent px-4 py-2 text-small font-medium text-accent-on"
        >
          {actionLabel}
        </button>
      ) : null}
    </div>
  );
}

export function ConfirmDialog({
  open,
  title,
  description,
  confirmLabel,
  onConfirm,
  onCancel,
}: {
  open: boolean;
  title: string;
  description?: string;
  confirmLabel: string;
  onConfirm: () => void;
  onCancel: () => void;
}) {
  const { t } = useI18n();
  const dialogRef = useRef<HTMLDivElement>(null);
  const cancelRef = useRef<HTMLButtonElement>(null);
  const triggerRef = useRef<HTMLElement | null>(null);

  // Focus enters the dialog (on the least destructive action) and returns to
  // the control that opened it (spec §5, WCAG 2.2 AA).
  useEffect(() => {
    if (!open) {
      return;
    }
    triggerRef.current =
      document.activeElement instanceof HTMLElement ? document.activeElement : null;
    cancelRef.current?.focus();
    return () => {
      triggerRef.current?.focus();
    };
  }, [open]);

  if (!open) {
    return null;
  }

  function onKeyDown(event: KeyboardEvent<HTMLDivElement>) {
    if (event.key === "Escape") {
      event.stopPropagation();
      onCancel();
      return;
    }
    if (event.key !== "Tab") {
      return;
    }
    const focusable = dialogRef.current?.querySelectorAll<HTMLElement>(
      'button, [href], input, select, textarea, [tabindex]:not([tabindex="-1"])',
    );
    if (!focusable || focusable.length === 0) {
      return;
    }
    const first = focusable[0];
    const last = focusable[focusable.length - 1];
    if (event.shiftKey && document.activeElement === first) {
      event.preventDefault();
      last.focus();
    } else if (!event.shiftKey && document.activeElement === last) {
      event.preventDefault();
      first.focus();
    }
  }

  return (
    <div
      ref={dialogRef}
      data-testid="confirm-dialog"
      role="dialog"
      aria-modal="true"
      aria-labelledby="confirm-dialog-title"
      onKeyDown={onKeyDown}
      className="fixed inset-0 z-50 flex items-center justify-center bg-ink/40 p-4"
    >
      <div className="w-full max-w-md rounded-lg border border-border bg-surface p-6">
        <h3 id="confirm-dialog-title" className="font-display text-section font-medium text-ink">
          {title}
        </h3>
        {description ? <p className="mt-2 text-small text-muted">{description}</p> : null}
        <div className="mt-4 flex gap-2">
          <button
            type="button"
            data-testid="confirm-accept"
            onClick={onConfirm}
            className="rounded-lg bg-danger px-4 py-2 text-small font-medium text-accent-on"
          >
            {confirmLabel}
          </button>
          <button
            ref={cancelRef}
            type="button"
            data-testid="confirm-cancel"
            onClick={onCancel}
            className="rounded-lg bg-surface-warm px-4 py-2 text-small font-medium text-ink"
          >
            {t("common.cancel")}
          </button>
        </div>
      </div>
    </div>
  );
}

export function DataTable({
  caption,
  columns,
  rows,
}: {
  caption: string;
  columns: Array<{ key: string; header: string; numeric?: boolean }>;
  rows: Array<Record<string, ReactNode>>;
}) {
  return (
    <div className="overflow-x-auto rounded-lg border border-border bg-surface">
      <table data-testid="data-table" className="w-full border-collapse text-small">
        <caption className="sr-only">{caption}</caption>
        <thead>
          <tr className="text-left text-overline uppercase tracking-overline text-meta">
            {columns.map((column) => (
              <th
                key={column.key}
                scope="col"
                className={`border-b border-border-soft px-3 py-2 ${
                  column.numeric ? "text-right" : ""
                }`}
              >
                {column.header}
              </th>
            ))}
          </tr>
        </thead>
        <tbody>
          {rows.map((row, index) => (
            <tr key={String(row.id ?? index)} className="border-b border-border-soft">
              {columns.map((column) => (
                <td
                  key={column.key}
                  className={`px-3 py-2 ${column.numeric ? "text-right font-mono tabular-nums" : ""}`}
                >
                  {row[column.key]}
                </td>
              ))}
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}

/** Renders when the gateway does not yet expose a route's C1 operation. */
export function BlockingState({ operation, unit }: { operation: string; unit?: string }) {
  const { t } = useI18n();
  return (
    <section
      data-testid="blocking-state"
      className="rounded-lg border border-border bg-surface-warm p-4"
    >
      <p className="text-small text-ink-2">{t("problem.unimplemented")}</p>
      <p className="mt-1 text-small text-meta">
        <span className="font-mono">{operation}</span>
        {unit ? ` · ${unit}` : null}
      </p>
    </section>
  );
}

export function Loading({ label }: { label?: string }) {
  const { t } = useI18n();
  return (
    <p
      data-testid="loading"
      role="status"
      aria-live="polite"
      className="animate-pulse text-small text-muted"
    >
      {label ?? t("common.loading")}
    </p>
  );
}

export function StatusRegion({ message }: { message: string | null }) {
  return (
    <div
      data-testid="status-region"
      role="status"
      aria-live="polite"
      className="text-small text-ink-2"
    >
      {message}
    </div>
  );
}

export function PageHeader({
  title,
  description,
  action,
}: {
  title: string;
  description?: string;
  action?: ReactNode;
}) {
  return (
    <header className="mb-6 flex flex-wrap items-end justify-between gap-3">
      <div>
        <h1 className="font-display text-display font-medium leading-display text-ink">{title}</h1>
        {description ? (
          <p className="mt-2 max-w-prose text-small text-muted">{description}</p>
        ) : null}
      </div>
      {action}
    </header>
  );
}

/**
 * The loading/error half of every async surface (BR4.2). Pages add their own
 * empty and degraded states; a source outage therefore degrades one panel.
 */
export function QueryBoundary({
  loading,
  error,
  onRetry,
  children,
}: {
  loading: boolean;
  error: unknown;
  onRetry?: () => void;
  children: ReactNode;
}) {
  if (loading) {
    return <Loading />;
  }
  if (error) {
    return <ErrorPanel error={error} onRetry={onRetry} />;
  }
  return <>{children}</>;
}

/** A valuation summary: total, status, coverage and as-of, never bare numbers. */
export function ValuationSummary({
  valuation,
}: {
  valuation: {
    totalValue: string | null;
    currency: string;
    status: DataStatus;
    asOf: string;
    pricedCoverage: number;
    unpricedCount: number;
    excludedCount?: number;
    holdingsUnavailable?: boolean;
  };
}) {
  const { t } = useI18n();
  return (
    <section className="rounded-lg border border-border bg-surface p-6">
      <p className="text-overline uppercase tracking-overline text-meta">
        {t("dashboard.totalValue")}
      </p>
      <p className="mt-1 text-kpi font-mono tabular-nums text-ink">
        <Money value={valuation.totalValue} currency={valuation.currency} />
      </p>
      <div className="mt-2 flex flex-wrap items-center gap-3">
        <DataStatusBadge status={valuation.status} asOf={valuation.asOf} />
        <CoverageNote
          coverage={valuation.pricedCoverage}
          excludedCount={valuation.excludedCount ?? valuation.unpricedCount}
          holdingsUnavailable={valuation.holdingsUnavailable}
        />
      </div>
    </section>
  );
}

/** A labelled GPT grid for P&L coverage flags. */
export function PnlFigures({ pnl }: { pnl: Pnl }) {
  const { t } = useI18n();
  return (
    <section className="rounded-lg border border-border bg-surface p-6">
      <dl className="grid gap-4 sm:grid-cols-3">
        <div>
          <dt className="text-meta text-small">{t("activity.netInvested")}</dt>
          <dd className="text-section">
            <Money value={pnl.netInvested} currency={pnl.currency} />
          </dd>
        </div>
        <div>
          <dt className="text-meta text-small">{t("activity.realized")}</dt>
          <dd className="text-section">
            <Money value={pnl.realized} currency={pnl.currency} />
          </dd>
        </div>
        <div>
          <dt className="text-meta text-small">{t("activity.unrealized")}</dt>
          <dd className="text-section">
            <Money value={pnl.unrealized} currency={pnl.currency} />
          </dd>
        </div>
      </dl>
      <div className="mt-4">
        <CoverageNote
          coverage={pnl.coverage}
          marksUnavailable={pnl.marksUnavailable}
          holdingsUnavailable={pnl.holdingsUnavailable}
          fxUnavailable={pnl.fxUnavailable}
          fxEstimated={pnl.fxEstimated}
          gap={pnl.coverageGap}
        />
      </div>
    </section>
  );
}
