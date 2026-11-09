import { useQuery } from "@tanstack/react-query";
import { useEffect, useState, type FormEvent } from "react";
import { ApiError, fetchAccountMetadata, NetworkError } from "../api/client";
import type { AccountMetadata } from "../api/types";
import { formatMetadataValue } from "../domain/metadata";
import { MIN_REASON_LENGTH } from "../domain/grants";
import { isForbiddenError } from "../api/queryClient";
import { useClearSupportCacheOnErrors } from "../hooks/useClearSupportCacheOnErrors";
import {
  alertClass,
  cardClass,
  calloutClass,
  fieldErrorClass,
  inputClass,
  labelClass,
  overlineClass,
  primaryButtonClass,
  sectionTitleClass,
  linkClass,
} from "./ui";

export interface AccountMetadataPanelProps {
  accountId: string;
  grantId: string;
  token: string | null;
  defaultReason?: string;
  expiresAt?: string | null;
  onAccessEnded?: () => void;
}

function formatExpiry(value: string | null | undefined): string {
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

/** WF2: a metadata read under an active grant. No credential or portfolio field exists here. */
export function AccountMetadataPanel({
  accountId,
  grantId,
  token,
  defaultReason = "",
  expiresAt,
  onAccessEnded,
}: AccountMetadataPanelProps) {
  const [reason, setReason] = useState(defaultReason);
  const [submittedReason, setSubmittedReason] = useState<string | null>(null);
  const [reasonError, setReasonError] = useState<string | null>(null);

  const metadata = useQuery<AccountMetadata, Error>({
    queryKey: ["support", "metadata", accountId, grantId, submittedReason],
    queryFn: () =>
      fetchAccountMetadata(accountId, { grantId, reason: submittedReason ?? "" }, token),
    enabled: grantId.trim().length > 0 && submittedReason !== null,
    staleTime: 10_000,
  });

  useClearSupportCacheOnErrors(metadata.error);

  useEffect(() => {
    if (isForbiddenError(metadata.error)) {
      onAccessEnded?.();
    }
  }, [metadata.error, onAccessEnded]);

  function submit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (reason.trim().length < MIN_REASON_LENGTH) {
      setReasonError(`The reason must be at least ${MIN_REASON_LENGTH} characters.`);
      return;
    }
    setReasonError(null);
    setSubmittedReason(reason.trim());
  }

  const problem = metadata.error instanceof ApiError ? metadata.error.problem : null;
  const forbidden = problem?.status === 403;
  const unauthorized = problem?.status === 401;

  return (
    <section data-testid="account-metadata" className={cardClass}>
      <p className={overlineClass}>Metadata read</p>
      <h2 className={`${sectionTitleClass} mt-1`}>Account metadata</h2>
      <p className="mt-2 text-small text-ink-2">
        Account <span className="font-mono text-ink">{accountId}</span>. The support API records
        every read.
      </p>

      <form data-testid="account-metadata-form" onSubmit={submit} className="mt-4 flex flex-col gap-3">
        <label className={labelClass} htmlFor="account-metadata-reason">
          Reason for this read
          <textarea
            id="account-metadata-reason"
            data-testid="account-metadata-reason"
            value={reason}
            onChange={(event) => setReason(event.target.value)}
            rows={2}
            aria-invalid={Boolean(reasonError)}
            aria-describedby={reasonError ? "account-metadata-reason-error" : undefined}
            className={inputClass}
          />
          {reasonError ? (
            <span id="account-metadata-reason-error" className={fieldErrorClass}>
              {reasonError}
            </span>
          ) : null}
        </label>
        <div>
          <button
            type="submit"
            data-testid="account-metadata-load"
            disabled={metadata.isFetching}
            className={primaryButtonClass}
          >
            {metadata.isFetching ? "Loading…" : "Load metadata"}
          </button>
        </div>
      </form>

      {forbidden ? (
        <div data-testid="account-metadata-access-ended" role="alert" className={`${alertClass} mt-4`}>
          <p className="font-medium text-ink">Access ended</p>
          <p>The grant expired at {formatExpiry(expiresAt)}. Request a new grant to continue.</p>
          <a
            data-testid="account-metadata-request-new"
            href="#grants"
            className={`${linkClass} mt-2 inline-block`}
          >
            Request a new grant
          </a>
        </div>
      ) : null}

      {unauthorized ? (
        <p role="alert" className={`${alertClass} mt-4`}>
          Sign in again to continue.
        </p>
      ) : null}

      {metadata.error && !forbidden && !unauthorized ? (
        <p role="alert" className={`${alertClass} mt-4`}>
          {metadata.error instanceof NetworkError
            ? "Could not reach the gateway. Check your connection and try again."
            : (problem?.title ?? metadata.error.message)}
          {problem ? (
            <span className="ml-2 font-mono text-meta">Correlation ID {problem.correlationId}</span>
          ) : null}
        </p>
      ) : null}

      {metadata.data ? (
        <div data-testid="account-metadata-panel" className={`${calloutClass} mt-4`}>
          <dl className="grid grid-cols-1 gap-3 text-small sm:grid-cols-2">
            <div>
              <dt className="text-meta">Kind</dt>
              <dd className="text-ink">{formatMetadataValue(metadata.data.kind)}</dd>
            </div>
            <div>
              <dt className="text-meta">Label</dt>
              <dd className="text-ink">{formatMetadataValue(metadata.data.label)}</dd>
            </div>
            <div>
              <dt className="text-meta">Venue</dt>
              <dd className="text-ink">{formatMetadataValue(metadata.data.venue)}</dd>
            </div>
            <div>
              <dt className="text-meta">Status</dt>
              <dd className="text-ink">{formatMetadataValue(metadata.data.status)}</dd>
            </div>
            <div>
              <dt className="text-meta">Last synced</dt>
              <dd data-testid="account-metadata-last-synced" className="font-mono text-ink">
                {formatMetadataValue(metadata.data.lastSyncedAt)}
              </dd>
            </div>
          </dl>
        </div>
      ) : null}
    </section>
  );
}
