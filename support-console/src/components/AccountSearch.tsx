import { useQuery } from "@tanstack/react-query";
import { useState, type FormEvent } from "react";
import { ApiError, NetworkError, searchAccounts } from "../api/client";
import type { AccountSearchResult } from "../api/types";
import type { StatusPresentation } from "../domain/presentation";
import { useClearSupportCacheOnErrors } from "../hooks/useClearSupportCacheOnErrors";
import { StatusBadge } from "./StatusBadge";
import {
  alertClass,
  calloutClass,
  cardClass,
  inputClass,
  labelClass,
  overlineClass,
  primaryButtonClass,
  sectionTitleClass,
} from "./ui";

export interface AccountSearchProps {
  grantId: string;
  token: string | null;
  onSelect: (accountId: string) => void;
}

const accountStatus = (status: string): StatusPresentation => ({
  label: status,
  tone: "neutral",
  shape: "dot",
});

/** Search is grant-scoped: the grant id is mandatory and results are metadata only. */
export function AccountSearch({ grantId, token, onSelect }: AccountSearchProps) {
  const [queryText, setQueryText] = useState("");
  const [submittedQuery, setSubmittedQuery] = useState<string | null>(null);
  const hasGrant = grantId.trim().length > 0;

  const results = useQuery<AccountSearchResult, Error>({
    queryKey: ["support", "accounts", grantId, submittedQuery],
    queryFn: () => searchAccounts({ grantId, query: submittedQuery ?? "" }, token),
    enabled: hasGrant && submittedQuery !== null,
    staleTime: 30_000,
  });

  useClearSupportCacheOnErrors(results.error);

  function submit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (!hasGrant) {
      return;
    }
    setSubmittedQuery(queryText.trim());
  }

  const problem = results.error instanceof ApiError ? results.error.problem : null;

  return (
    <section data-testid="account-search" className={cardClass}>
      <p className={overlineClass}>Grant-scoped search</p>
      <h2 className={`${sectionTitleClass} mt-1`}>Account search</h2>
      <p className="mt-2 text-small text-ink-2">
        Results are limited to accounts owned by the active grant&apos;s target user. Metadata only.
      </p>

      {hasGrant ? null : (
        <p data-testid="account-search-grant-required" className={`${calloutClass} mt-4`}>
          An active support grant is required to search accounts. Request or approve a grant
          first.
        </p>
      )}

      <form
        data-testid="account-search-form"
        onSubmit={submit}
        className="mt-4 flex flex-col gap-3 sm:flex-row sm:items-end"
      >
        <label className={`${labelClass} flex-1`} htmlFor="account-search-query">
          Account id or label
          <input
            id="account-search-query"
            data-testid="account-search-query"
            value={queryText}
            onChange={(event) => setQueryText(event.target.value)}
            disabled={!hasGrant}
            autoComplete="off"
            spellCheck={false}
            className={inputClass}
          />
        </label>
        <button
          type="submit"
          data-testid="account-search-submit"
          disabled={!hasGrant || results.isFetching}
          className={primaryButtonClass}
        >
          {results.isFetching ? "Searching…" : "Search"}
        </button>
      </form>

      {results.error ? (
        <p role="alert" className={`${alertClass} mt-4`}>
          {problem?.status === 403
            ? "Access ended. Request a new grant to continue."
            : problem?.status === 401
              ? "Sign in again to continue."
              : results.error instanceof NetworkError
                ? "Could not reach the gateway. Check your connection and try again."
                : (problem?.title ?? results.error.message)}
          {problem ? (
            <span className="ml-2 font-mono text-meta">Correlation ID {problem.correlationId}</span>
          ) : null}
        </p>
      ) : null}

      {results.isSuccess && results.data.items.length === 0 ? (
        <p data-testid="account-search-empty" className={`${calloutClass} mt-4`}>
          No accounts found for this grant.
        </p>
      ) : null}

      {results.isSuccess && results.data.items.length > 0 ? (
        <ul className="mt-4 flex flex-col gap-2" aria-label="Account search results">
          {results.data.items.map((account) => (
            <li
              key={account.id}
              data-testid={`account-search-result-${account.id}`}
              className="flex flex-wrap items-center justify-between gap-2 rounded-lg border border-border-soft bg-surface-warm p-3"
            >
              <span className="text-small font-medium text-ink">{account.label}</span>
              <span className="text-small text-meta">
                {account.kind} · <StatusBadge presentation={accountStatus(account.status)} />
              </span>
              <button
                type="button"
                data-testid={`account-search-open-${account.id}`}
                onClick={() => onSelect(account.id)}
                className={primaryButtonClass}
              >
                Open {account.label}
              </button>
            </li>
          ))}
        </ul>
      ) : null}
    </section>
  );
}
