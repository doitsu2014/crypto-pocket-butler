import { useId, useState, type FormEvent } from "react";
import { ApiError, NetworkError } from "../../api/client";
import {
  alertClass,
  fieldErrorClass,
  inputClass,
  labelClass,
  primaryButtonClass,
} from "../../components/ui";

export interface AdminUserLookupProps {
  onSubmit: (email: string) => void;
  loading: boolean;
  error?: unknown;
  notFound?: boolean;
  onRetry?: () => void;
}

const EMAIL_PATTERN = /^[^\s@]+@[^\s@]+\.[^\s@]+$/;

function errorMessage(error: unknown): string {
  if (error instanceof NetworkError) {
    return "Could not reach the gateway. Check your connection and try again.";
  }
  if (error instanceof ApiError) {
    if (error.problem.status === 403) {
      return "Your session is not an administrator.";
    }
    return "This user cannot be looked up right now.";
  }
  return "This user cannot be looked up right now.";
}

/**
 * The address travels in the request body (D-S1): it must never reach a URL
 * or an access log. Client checks are presentation only; the server re-checks.
 */
export function AdminUserLookup({
  onSubmit,
  loading,
  error,
  notFound,
  onRetry,
}: AdminUserLookupProps) {
  const emailId = useId();
  const emailErrorId = useId();
  const [email, setEmail] = useState("");
  const [fieldError, setFieldError] = useState<string | null>(null);

  function submit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    const trimmed = email.trim();
    if (trimmed.length === 0) {
      setFieldError("Enter the user's email address.");
      return;
    }
    if (!EMAIL_PATTERN.test(trimmed)) {
      setFieldError("Enter a valid email address.");
      return;
    }
    setFieldError(null);
    onSubmit(trimmed);
  }

  const problem = error instanceof ApiError ? error.problem : null;

  return (
    <form data-testid="admin-lookup" onSubmit={submit} noValidate>
      <div className="flex flex-wrap items-end gap-3">
        <label htmlFor={emailId} className={`${labelClass} min-w-64 flex-1`}>
          User email address
          <input
            id={emailId}
            data-testid="admin-lookup-email"
            type="email"
            name="email"
            autoComplete="off"
            spellCheck={false}
            className={inputClass}
            value={email}
            onChange={(event) => setEmail(event.target.value)}
            disabled={loading}
            aria-invalid={fieldError ? true : undefined}
            aria-describedby={fieldError ? emailErrorId : undefined}
          />
        </label>
        <button
          type="submit"
          data-testid="admin-lookup-submit"
          className={primaryButtonClass}
          disabled={loading}
        >
          {loading ? "Looking up…" : "Look up user"}
        </button>
      </div>

      {fieldError ? (
        <p
          id={emailErrorId}
          data-testid="admin-lookup-email-error"
          className={`${fieldErrorClass} mt-2`}
        >
          {fieldError}
        </p>
      ) : null}

      {notFound ? (
        <p
          data-testid="admin-lookup-not-found"
          role="status"
          className="mt-3 text-small text-ink-2"
        >
          No user has that address.
        </p>
      ) : null}

      {error && !notFound ? (
        <div data-testid="admin-lookup-error" role="alert" className={`${alertClass} mt-3`}>
          <p>{errorMessage(error)}</p>
          {problem ? (
            <p className="mt-1 font-mono text-meta">Correlation ID {problem.correlationId}</p>
          ) : null}
          {onRetry ? (
            <button
              type="button"
              data-testid="admin-lookup-retry"
              onClick={onRetry}
              className="mt-2 underline"
            >
              Try again
            </button>
          ) : null}
        </div>
      ) : null}
    </form>
  );
}
