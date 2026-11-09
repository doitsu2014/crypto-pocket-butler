import { useMutation } from "@tanstack/react-query";
import { useState, type FormEvent } from "react";
import { ApiError, NetworkError, requestSupportGrant } from "../api/client";
import type { SupportAccessGrant } from "../api/types";
import { hasGrantRequestErrors, validateGrantRequest, type GrantRequestErrors } from "../domain/grants";
import {
  alertClass,
  cardClass,
  fieldErrorClass,
  inputClass,
  labelClass,
  overlineClass,
  primaryButtonClass,
  sectionTitleClass,
} from "./ui";

export interface GrantRequestFormProps {
  requesterId: string;
  token: string | null;
  onCreated: (grant: SupportAccessGrant) => void;
}

/** BR1.2: reason ≥ 20 characters, 1–480 minutes, and a second approver who is not the requester. */
export function GrantRequestForm({ requesterId, token, onCreated }: GrantRequestFormProps) {
  const [targetUserId, setTargetUserId] = useState("");
  const [reason, setReason] = useState("");
  const [durationMinutes, setDurationMinutes] = useState("60");
  const [approverId, setApproverId] = useState("");
  const [errors, setErrors] = useState<GrantRequestErrors>({});

  const mutation = useMutation<SupportAccessGrant, Error>({
    mutationFn: () =>
      requestSupportGrant(
        {
          userId: targetUserId.trim(),
          reason: reason.trim(),
          durationMinutes: Number(durationMinutes),
          approverId: approverId.trim(),
        },
        token,
      ),
    onSuccess: (grant) => {
      setErrors({});
      onCreated(grant);
    },
  });

  function submit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (mutation.isPending) {
      return;
    }

    const nextErrors = validateGrantRequest({
      requesterId,
      targetUserId,
      reason,
      durationMinutes: Number(durationMinutes),
      approverId,
    });
    setErrors(nextErrors);
    if (hasGrantRequestErrors(nextErrors)) {
      return;
    }
    mutation.mutate();
  }

  const problem = mutation.error instanceof ApiError ? mutation.error.problem : null;
  const isNetworkFailure = mutation.error instanceof NetworkError;

  return (
    <form data-testid="grant-request-form" onSubmit={submit} className={cardClass} noValidate>
      <p className={overlineClass}>New support grant</p>
      <h2 className={`${sectionTitleClass} mt-1`}>Request access</h2>
      <p className="mt-2 text-small text-ink-2">
        A grant is pending until a second approver accepts it. Access ends automatically at the
        chosen time.
      </p>

      <div className="mt-6 grid gap-4 sm:grid-cols-2">
        <label className={labelClass} htmlFor="grant-request-target">
          Target user
          <input
            id="grant-request-target"
            data-testid="grant-request-target"
            value={targetUserId}
            onChange={(event) => setTargetUserId(event.target.value)}
            autoComplete="off"
            spellCheck={false}
            aria-invalid={Boolean(errors.targetUserId)}
            aria-describedby={errors.targetUserId ? "grant-request-target-error" : undefined}
            className={inputClass}
          />
          {errors.targetUserId ? (
            <span id="grant-request-target-error" className={fieldErrorClass}>
              {errors.targetUserId}
            </span>
          ) : null}
        </label>

        <label className={labelClass} htmlFor="grant-request-approver">
          Second approver
          <input
            id="grant-request-approver"
            data-testid="grant-request-approver"
            value={approverId}
            onChange={(event) => setApproverId(event.target.value)}
            autoComplete="off"
            spellCheck={false}
            aria-invalid={Boolean(errors.approverId)}
            aria-describedby={errors.approverId ? "grant-request-approver-error" : undefined}
            className={inputClass}
          />
          {errors.approverId ? (
            <span id="grant-request-approver-error" className={fieldErrorClass}>
              {errors.approverId}
            </span>
          ) : null}
        </label>

        <label className={`${labelClass} sm:col-span-2`} htmlFor="grant-request-reason">
          Reason
          <textarea
            id="grant-request-reason"
            data-testid="grant-request-reason"
            value={reason}
            onChange={(event) => setReason(event.target.value)}
            rows={3}
            aria-invalid={Boolean(errors.reason)}
            aria-describedby={errors.reason ? "grant-request-reason-error" : undefined}
            className={inputClass}
          />
          {errors.reason ? (
            <span id="grant-request-reason-error" className={fieldErrorClass}>
              {errors.reason}
            </span>
          ) : null}
        </label>

        <label className={labelClass} htmlFor="grant-request-duration">
          Duration (minutes)
          <input
            id="grant-request-duration"
            data-testid="grant-request-duration"
            type="number"
            min={1}
            max={480}
            value={durationMinutes}
            onChange={(event) => setDurationMinutes(event.target.value)}
            aria-invalid={Boolean(errors.durationMinutes)}
            aria-describedby={errors.durationMinutes ? "grant-request-duration-error" : undefined}
            className={inputClass}
          />
          {errors.durationMinutes ? (
            <span id="grant-request-duration-error" className={fieldErrorClass}>
              {errors.durationMinutes}
            </span>
          ) : null}
        </label>
      </div>

      {mutation.error ? (
        <p role="alert" className={`${alertClass} mt-4`}>
          {isNetworkFailure
            ? "Could not reach the gateway. Check your connection and try again."
            : (problem?.title ?? mutation.error.message)}
          {problem ? (
            <span className="ml-2 font-mono text-meta">Correlation ID {problem.correlationId}</span>
          ) : null}
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
