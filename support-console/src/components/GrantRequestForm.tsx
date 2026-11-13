import { useMutation } from "@tanstack/react-query";
import { useState, type FormEvent } from "react";
import { ApiError, NetworkError, requestSupportGrant } from "../api/client";
import type { SupportAccessGrant } from "../api/types";
import {
  hasGrantRequestErrors,
  validateGrantRequest,
  type GrantRequestErrors,
} from "../domain/grants";
import { GATEWAY_UNREACHABLE_MESSAGE } from "../domain/presentation";
import {
  alertClass,
  cardClass,
  fieldErrorClass,
  inputClass,
  labelClass,
  overlineClass,
  primaryButtonClass,
  secondaryButtonClass,
  sectionTitleClass,
} from "./ui";

export interface GrantRequestFormProps {
  token: string | null;
  onCreated: (grant: SupportAccessGrant) => void;
  onCheckStatus: () => void;
}

/** W1: a request carries a target user id and a reason of 1–500 characters. */
export function GrantRequestForm({ token, onCreated, onCheckStatus }: GrantRequestFormProps) {
  const [targetUserId, setTargetUserId] = useState("");
  const [reason, setReason] = useState("");
  const [errors, setErrors] = useState<GrantRequestErrors>({});
  const [created, setCreated] = useState(false);

  const mutation = useMutation<SupportAccessGrant, Error>({
    mutationFn: () =>
      requestSupportGrant(
        {
          userId: targetUserId.trim(),
          reason: reason.trim(),
        },
        token,
      ),
    onSuccess: (grant) => {
      setErrors({});
      setCreated(true);
      setTargetUserId("");
      setReason("");
      onCreated(grant);
    },
  });

  function submit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (mutation.isPending) {
      return;
    }

    const nextErrors = validateGrantRequest({ targetUserId, reason });
    setErrors(nextErrors);
    setCreated(false);
    if (hasGrantRequestErrors(nextErrors)) {
      return;
    }
    mutation.mutate();
  }

  const problem = mutation.error instanceof ApiError ? mutation.error : null;
  const isNetworkFailure = mutation.error instanceof NetworkError;
  const serverFieldErrors: GrantRequestErrors = {};
  for (const entry of problem?.problem.errors ?? []) {
    if (entry.field === "userId") {
      serverFieldErrors.targetUserId = entry.reason;
    } else if (entry.field === "reason") {
      serverFieldErrors.reason = entry.reason;
    }
  }
  const shownErrors: GrantRequestErrors = { ...errors, ...serverFieldErrors };

  let message: string | null = null;
  if (isNetworkFailure) {
    message = GATEWAY_UNREACHABLE_MESSAGE;
  } else if (problem !== null) {
    switch (problem.status) {
      case 403:
        message = "You are not allowed to request support access.";
        break;
      case 404:
        message = "No user matches that id. Check the target user's id and try again.";
        break;
      case 409:
        message =
          "You already have an open grant. Check your current grant before requesting a new one.";
        break;
      case 422:
        message = "The request could not be accepted. Check the fields below.";
        break;
      case 429:
        message =
          mutation.error instanceof ApiError && mutation.error.retryAfterSeconds !== undefined
            ? `Too many requests. Try again in ${mutation.error.retryAfterSeconds} seconds.`
            : "Too many requests. Try again shortly.";
        break;
      default:
        message = problem.problem.title;
    }
  }

  const showCorrelationId = problem !== null && problem.status !== 403;

  return (
    <form data-testid="grant-request-form" onSubmit={submit} className={cardClass} noValidate>
      <p className={overlineClass}>New support grant</p>
      <h2 className={`${sectionTitleClass} mt-1`}>Request access</h2>
      <p className="mt-2 text-small text-ink-2">
        A grant is pending until a second approver accepts it. Access lasts only as long as the
        approved grant, and every metadata read is recorded by the support API.
      </p>

      <div className="mt-6 grid gap-4">
        <label className={labelClass} htmlFor="grant-request-target">
          Target user
          <input
            id="grant-request-target"
            data-testid="grant-request-target"
            value={targetUserId}
            onChange={(event) => setTargetUserId(event.target.value)}
            autoComplete="off"
            spellCheck={false}
            aria-invalid={Boolean(shownErrors.targetUserId)}
            aria-describedby={shownErrors.targetUserId ? "grant-request-target-error" : undefined}
            className={inputClass}
          />
          {shownErrors.targetUserId ? (
            <span id="grant-request-target-error" className={fieldErrorClass}>
              {shownErrors.targetUserId}
            </span>
          ) : null}
        </label>

        <label className={labelClass} htmlFor="grant-request-reason">
          Reason
          <textarea
            id="grant-request-reason"
            data-testid="grant-request-reason"
            value={reason}
            onChange={(event) => setReason(event.target.value)}
            rows={3}
            aria-invalid={Boolean(shownErrors.reason)}
            aria-describedby={shownErrors.reason ? "grant-request-reason-error" : undefined}
            className={inputClass}
          />
          {shownErrors.reason ? (
            <span id="grant-request-reason-error" className={fieldErrorClass}>
              {shownErrors.reason}
            </span>
          ) : null}
        </label>
      </div>

      {message ? (
        <div className={`${alertClass} mt-4`} role="alert">
          <span>{message}</span>
          {showCorrelationId ? (
            <span className="ml-2 font-mono text-meta">
              Correlation ID {problem?.problem.correlationId}
            </span>
          ) : null}
        </div>
      ) : null}

      {problem?.status === 409 ? (
        <div className="mt-3">
          <button
            type="button"
            data-testid="grant-request-check-status"
            onClick={onCheckStatus}
            className={secondaryButtonClass}
          >
            Check your current grant
          </button>
        </div>
      ) : null}

      {created ? (
        <p
          data-testid="grant-request-success"
          role="status"
          className="mt-4 text-small text-success"
        >
          Grant requested. It stays pending until a second approver accepts it.
        </p>
      ) : null}

      <button
        type="submit"
        data-testid="grant-request-submit"
        disabled={mutation.isPending}
        className={`${primaryButtonClass} mt-6`}
      >
        {mutation.isPending ? "Requesting…" : "Request grant"}
      </button>
    </form>
  );
}
