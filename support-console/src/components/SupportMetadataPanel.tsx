import { useEffect, useState } from "react";
import { ApiError, NetworkError } from "../api/client";
import type { GrantStatus, SupportAccessGrant } from "../api/types";
import { formatMetadataValue } from "../domain/metadata";
import { READ_AUDIT_STATEMENT } from "../domain/presentation";
import { useSupportMetadata } from "../hooks/useSupportMetadata";
import {
  alertClass,
  cardClass,
  fieldErrorClass,
  inputClass,
  labelClass,
  overlineClass,
  secondaryButtonClass,
  sectionTitleClass,
} from "./ui";

export interface SupportMetadataPanelProps {
  grant: SupportAccessGrant;
  token: string | null;
  status: GrantStatus | null;
  onAccessEnded?: () => void;
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

/**
 * W3: the account and portfolio metadata behind the live grant. The read query
 * is mounted only while the grant is active; there is no request otherwise.
 */
export function SupportMetadataPanel(props: SupportMetadataPanelProps) {
  if (props.status !== "active") {
    return (
      <section data-testid="support-metadata-panel" className={cardClass}>
        <p className={overlineClass}>Support metadata</p>
        <h2 className={`${sectionTitleClass} mt-1`}>Account and portfolio metadata</h2>
        <p className="mt-3 text-small text-ink-2">Metadata is available while a grant is active.</p>
      </section>
    );
  }
  return <ActiveSupportMetadataPanel {...props} />;
}

function ActiveSupportMetadataPanel({
  grant,
  token,
  status,
  onAccessEnded,
}: SupportMetadataPanelProps) {
  const query = useSupportMetadata(grant, token, status, onAccessEnded);
  const [filter, setFilter] = useState("");
  const [failedStatus, setFailedStatus] = useState<number | null>(null);
  const [networkFailure, setNetworkFailure] = useState(false);

  useEffect(() => {
    if (query.error instanceof ApiError) {
      setFailedStatus(query.error.status);
    } else if (query.error instanceof NetworkError) {
      setNetworkFailure(true);
      setFailedStatus(null);
    }
  }, [query.error]);

  useEffect(() => {
    if (query.isSuccess) {
      setFailedStatus(null);
      setNetworkFailure(false);
    }
  }, [query.isSuccess]);

  if (failedStatus === 401) {
    return (
      <section data-testid="support-metadata-panel" className={cardClass}>
        <p className={overlineClass}>Support metadata</p>
        <h2 className={`${sectionTitleClass} mt-1`}>Account and portfolio metadata</h2>
        <p role="alert" className={`${alertClass} mt-3`}>
          Your session has expired. Sign in again.
        </p>
      </section>
    );
  }

  if (failedStatus === 403) {
    return (
      <section data-testid="support-metadata-panel" className={cardClass}>
        <p className={overlineClass}>Support metadata</p>
        <h2 className={`${sectionTitleClass} mt-1`}>Account and portfolio metadata</h2>
        <p role="alert" className={`${alertClass} mt-3`}>
          Access ended. Ask an administrator for a new grant.
        </p>
      </section>
    );
  }

  if (failedStatus !== null || networkFailure) {
    const problem = query.error instanceof ApiError ? query.error.problem : null;
    return (
      <section data-testid="support-metadata-panel" className={cardClass}>
        <p className={overlineClass}>Support metadata</p>
        <h2 className={`${sectionTitleClass} mt-1`}>Account and portfolio metadata</h2>
        <div className={`${alertClass} mt-3`} role="alert">
          <span>Could not load support metadata.</span>
          {problem ? (
            <span className="ml-2 font-mono text-meta">Correlation ID {problem.correlationId}</span>
          ) : null}
        </div>
        <button
          type="button"
          data-testid="metadata-refresh"
          onClick={() => void query.refetch()}
          className={`${secondaryButtonClass} mt-3`}
        >
          Refresh
        </button>
      </section>
    );
  }

  if (query.isPending || !query.data) {
    return (
      <section data-testid="support-metadata-panel" className={cardClass}>
        <p className={overlineClass}>Support metadata</p>
        <h2 className={`${sectionTitleClass} mt-1`}>Account and portfolio metadata</h2>
        <p className="mt-3 text-small text-ink-2">Loading metadata…</p>
      </section>
    );
  }

  const metadata = query.data;
  const needle = filter.trim().toLowerCase();
  const accounts = needle
    ? metadata.accounts.filter((account) =>
        `${account.label} ${account.sourceType}`.toLowerCase().includes(needle),
      )
    : metadata.accounts;

  return (
    <section data-testid="support-metadata-panel" className={cardClass}>
      <p className={overlineClass}>Support metadata</p>
      <h2 className={`${sectionTitleClass} mt-1`}>Account and portfolio metadata</h2>
      <p className="mt-2 text-small text-ink-2">
        {READ_AUDIT_STATEMENT} Balances, holdings, and credentials are never shown.
      </p>

      <dl data-testid="metadata-user" className="mt-4 grid gap-2 text-small sm:grid-cols-2">
        <div>
          <dt className="text-meta">User</dt>
          <dd className="font-mono text-ink">{formatMetadataValue(metadata.userId)}</dd>
        </div>
        <div>
          <dt className="text-meta">Display name</dt>
          <dd className="text-ink">{formatMetadataValue(metadata.displayName)}</dd>
        </div>
        <div>
          <dt className="text-meta">Email</dt>
          <dd className="text-ink">{metadata.emailVerified ? "Verified" : "Not verified"}</dd>
        </div>
      </dl>

      <div className="mt-6 flex flex-col gap-2">
        <label className={labelClass} htmlFor="metadata-filter">
          Filter accounts
          <input
            id="metadata-filter"
            data-testid="metadata-filter"
            value={filter}
            onChange={(event) => setFilter(event.target.value)}
            autoComplete="off"
            spellCheck={false}
            className={inputClass}
          />
        </label>
      </div>

      <div className="mt-4">
        <h3 className="text-small font-medium text-ink">Accounts</h3>
        {metadata.accounts.length === 0 ? (
          <p data-testid="metadata-accounts-empty" className="mt-2 text-small text-ink-2">
            No accounts to show.
          </p>
        ) : accounts.length === 0 ? (
          <p data-testid="metadata-accounts-no-match" className={fieldErrorClass}>
            No accounts match that filter.
          </p>
        ) : (
          <table data-testid="metadata-accounts" className="mt-2 w-full text-small">
            <thead>
              <tr className="text-left text-meta">
                <th scope="col" className="py-1 pr-2 font-medium">
                  Account
                </th>
                <th scope="col" className="py-1 pr-2 font-medium">
                  Source
                </th>
                <th scope="col" className="py-1 pr-2 font-medium">
                  Sync status
                </th>
                <th scope="col" className="py-1 font-medium">
                  Last synced
                </th>
              </tr>
            </thead>
            <tbody>
              {accounts.map((account) => (
                <tr key={account.id} className="border-t border-border-soft">
                  <td className="py-1 pr-2 text-ink">{account.label}</td>
                  <td className="py-1 pr-2 text-ink-2">{account.sourceType}</td>
                  <td className="py-1 pr-2 text-ink-2">{account.syncStatus}</td>
                  <td className="py-1 font-mono text-ink-2">
                    {formatDateTime(account.lastSyncedAt)}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        )}
      </div>

      <div className="mt-6">
        <h3 className="text-small font-medium text-ink">Portfolios</h3>
        {metadata.portfolios.length === 0 ? (
          <p data-testid="metadata-portfolios-empty" className="mt-2 text-small text-ink-2">
            No portfolios to show.
          </p>
        ) : (
          <ul data-testid="metadata-portfolios" className="mt-2 flex flex-col gap-1 text-small">
            {metadata.portfolios.map((portfolio) => (
              <li key={portfolio.id} className="text-ink">
                {portfolio.label}
              </li>
            ))}
          </ul>
        )}
      </div>
    </section>
  );
}
