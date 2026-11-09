import type { SupportAccessGrant } from "../api/types";
import { grantStatusAt, grantStatusPresentation } from "../domain/grants";
import { formatMetadataValue } from "../domain/metadata";
import { useGrantClock } from "../hooks/useGrantClock";
import { StatusBadge } from "./StatusBadge";
import { cardClass, overlineClass, primaryButtonClass } from "./ui";

export interface GrantBannerProps {
  grant: SupportAccessGrant;
  now?: number;
  onRequestNew: () => void;
}

function formatDateTime(value: string | null): string {
  if (!value) {
    return "—";
  }
  const parsed = Date.parse(value);
  if (!Number.isFinite(parsed)) {
    return "—";
  }
  return new Intl.DateTimeFormat(undefined, {
    dateStyle: "medium",
    timeStyle: "short",
  }).format(new Date(parsed));
}

export function GrantBanner({ grant, now, onRequestNew }: GrantBannerProps) {
  const clock = useGrantClock(grant.expiresAt);
  const currentNow = now ?? clock;
  const status = grantStatusAt(grant, currentNow);
  const presentation = grantStatusPresentation(status);

  let copy: string;
  switch (status) {
    case "active":
      copy = `Access ends at ${formatDateTime(grant.expiresAt)}. Every metadata read is recorded by the support API.`;
      break;
    case "pending_approval":
      copy = "Waiting for a second approver. No metadata reads are allowed yet.";
      break;
    case "revoked":
      copy = "This grant was revoked. Request a new grant to continue.";
      break;
    case "expired":
      copy = `Access ended at ${formatDateTime(grant.expiresAt)}. Request a new grant to continue.`;
      break;
  }

  return (
    <section
      data-testid="grant-banner"
      aria-live="polite"
      aria-label="Current support grant"
      className={`${cardClass} flex flex-col gap-3`}
    >
      <div className="flex flex-wrap items-center justify-between gap-3">
        <p className={overlineClass}>Support grant</p>
        <StatusBadge presentation={presentation} testId="grant-banner-status" />
      </div>

      <h2 className="font-display text-body font-medium leading-body text-ink">{grant.reason}</h2>
      <p className="text-small text-ink-2">{copy}</p>

      <dl className="grid grid-cols-1 gap-2 text-small sm:grid-cols-2">
        <div>
          <dt className="text-meta">Target user</dt>
          <dd className="font-mono text-ink">{formatMetadataValue(grant.userId)}</dd>
        </div>
        <div>
          <dt className="text-meta">Approved by</dt>
          <dd className="font-mono text-ink">{formatMetadataValue(grant.approvedBy)}</dd>
        </div>
        <div>
          <dt className="text-meta">Expires</dt>
          <dd className="font-mono text-ink">{formatDateTime(grant.expiresAt)}</dd>
        </div>
      </dl>

      <div>
        <button
          type="button"
          data-testid="grant-banner-request-new"
          onClick={onRequestNew}
          className={primaryButtonClass}
        >
          Request new grant
        </button>
      </div>
    </section>
  );
}
