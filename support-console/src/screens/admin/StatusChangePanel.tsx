import { useEffect, useId, useRef, useState, type FormEvent, type KeyboardEvent } from "react";
import { ApiError, NetworkError } from "../../api/client";
import type { AdminUserRecord } from "../../api/types";
import {
  alertClass,
  dangerButtonClass,
  fieldErrorClass,
  inputClass,
  labelClass,
  primaryButtonClass,
  secondaryButtonClass,
} from "../../components/ui";
import { useSetUserStatus } from "../../hooks/useAdminUsers";

export interface StatusChangePanelProps {
  record: AdminUserRecord;
  token: string;
  onChanged: (status: AdminUserRecord["status"]) => void;
}

const MAX_REASON = 500;

function disableCopy(): string {
  return "The user cannot sign in while the account is disabled. History is not deleted.";
}

function restoreCopy(): string {
  return "The user can sign in again at their next request.";
}

/**
 * Disable and restore take a confirmation step: nothing is sent until Confirm
 * is pressed, recovery is offered only for a disabled account, and an account
 * already headed for deletion explains why no action is offered.
 */
export function StatusChangePanel({ record, token, onChanged }: StatusChangePanelProps) {
  const reasonId = useId();
  const reasonErrorId = useId();
  const [reason, setReason] = useState("");
  const [confirming, setConfirming] = useState(false);
  const [reasonError, setReasonError] = useState<string | null>(null);
  const [note, setNote] = useState<{
    tone: "ok" | "error";
    text: string;
    correlationId?: string;
  } | null>(null);
  const [reportedStatus, setReportedStatus] = useState<string | null>(null);
  const actionRef = useRef<HTMLButtonElement>(null);
  const confirmRef = useRef<HTMLButtonElement>(null);
  const shouldRefocus = useRef(false);

  const setStatus = useSetUserStatus(token);
  const isDisabled = record.status === "disabled";
  const unrecoverable = record.status === "pending_deletion" || record.status === "deleted";
  const pending = setStatus.isPending;

  useEffect(() => {
    if (confirming) {
      confirmRef.current?.focus();
      return;
    }
    if (shouldRefocus.current) {
      shouldRefocus.current = false;
      actionRef.current?.focus();
    }
  }, [confirming]);

  function closeConfirm() {
    shouldRefocus.current = true;
    setConfirming(false);
  }

  function beginConfirm(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setNote(null);
    setReportedStatus(null);
    const trimmed = reason.trim();
    if (trimmed.length === 0 || trimmed.length > MAX_REASON) {
      setReasonError("Enter a reason of up to 500 characters.");
      return;
    }
    setReasonError(null);
    setConfirming(true);
  }

  function confirmChange() {
    setStatus.mutate(
      { userId: record.id, currentStatus: record.status, reason: reason.trim() },
      {
        onSuccess: (answer) => {
          setReportedStatus(answer.status);
          setNote({ tone: "ok", text: "This change is audited." });
          shouldRefocus.current = true;
          setConfirming(false);
          setReason("");
          onChanged(answer.status);
        },
        onError: (error) => {
          setConfirming(false);
          if (error instanceof NetworkError) {
            setNote({
              tone: "error",
              text: "Could not reach the gateway. Check your connection and try again.",
            });
            return;
          }
          if (error instanceof ApiError) {
            setNote({
              tone: "error",
              text:
                error.problem.status === 403
                  ? "Your session is not an administrator."
                  : "The account status could not be changed. Re-read the record before trying again.",
              correlationId: error.problem.correlationId,
            });
            return;
          }
          setNote({ tone: "error", text: "The account status could not be changed." });
        },
      },
    );
  }

  function onConfirmKeyDown(event: KeyboardEvent<HTMLDivElement>) {
    if (event.key === "Escape") {
      event.preventDefault();
      closeConfirm();
      return;
    }
    if (event.key !== "Tab") {
      return;
    }
    const buttons = event.currentTarget.querySelectorAll<HTMLButtonElement>("button");
    if (buttons.length === 0) {
      return;
    }
    const first = buttons[0];
    const last = buttons[buttons.length - 1];
    const active = document.activeElement;
    if (event.shiftKey && active === first) {
      event.preventDefault();
      last.focus();
    } else if (!event.shiftKey && active === last) {
      event.preventDefault();
      first.focus();
    }
  }

  if (unrecoverable) {
    return (
      <div data-testid="admin-status-panel" className="flex flex-col gap-2">
        <h4 className="text-small font-semibold text-ink">Status</h4>
        <p data-testid="admin-status-explanatory" className="text-small text-ink-2">
          {record.status === "pending_deletion"
            ? "This account is pending deletion. Disable and restore are not offered while the deletion is in progress."
            : "This account has been erased. There is no account left to disable or restore."}
        </p>
      </div>
    );
  }

  return (
    <form
      data-testid="admin-status-panel"
      onSubmit={beginConfirm}
      noValidate
      className="flex flex-col gap-3"
    >
      <h4 className="text-small font-semibold text-ink">Status</h4>

      <label htmlFor={reasonId} className={labelClass}>
        Reason
        <textarea
          id={reasonId}
          data-testid="admin-status-reason"
          className={`${inputClass} min-h-20`}
          value={reason}
          maxLength={MAX_REASON}
          onChange={(event) => setReason(event.target.value)}
          disabled={pending}
          aria-invalid={reasonError ? true : undefined}
          aria-describedby={reasonError ? reasonErrorId : undefined}
        />
      </label>
      {reasonError ? (
        <p id={reasonErrorId} data-testid="admin-status-reason-error" className={fieldErrorClass}>
          {reasonError}
        </p>
      ) : null}

      {!confirming ? (
        <div>
          <button
            ref={actionRef}
            type="submit"
            data-testid="admin-status-action"
            className={isDisabled ? `${secondaryButtonClass} border-border` : dangerButtonClass}
            disabled={pending}
          >
            {isDisabled ? "Restore account" : "Disable account"}
          </button>
        </div>
      ) : (
        <div
          data-testid="admin-status-confirm"
          role="alertdialog"
          aria-label={isDisabled ? "Confirm restore" : "Confirm disable"}
          className="flex flex-col gap-3 rounded-lg border border-danger bg-surface-warm p-4"
          onKeyDown={onConfirmKeyDown}
        >
          <p className="text-small text-ink-2">{isDisabled ? restoreCopy() : disableCopy()}</p>
          <div className="flex flex-wrap gap-2">
            <button
              ref={confirmRef}
              type="button"
              data-testid="admin-status-confirm-button"
              className={isDisabled ? primaryButtonClass : dangerButtonClass}
              onClick={confirmChange}
              disabled={pending}
            >
              {isDisabled ? "Confirm restore" : "Confirm disable"}
            </button>
            <button
              type="button"
              data-testid="admin-status-cancel"
              className={secondaryButtonClass}
              onClick={closeConfirm}
              disabled={pending}
            >
              Cancel
            </button>
          </div>
        </div>
      )}

      {note?.tone === "ok" && reportedStatus ? (
        <p
          data-testid="admin-status-result"
          role="status"
          aria-live="polite"
          className="text-small text-ink-2"
        >
          Account status is now {reportedStatus}. {note.text}
        </p>
      ) : null}

      {note?.tone === "error" ? (
        <div data-testid="admin-status-error" role="alert" className={alertClass}>
          <p>{note.text}</p>
          {note.correlationId ? (
            <p className="mt-1 font-mono text-meta">Correlation ID {note.correlationId}</p>
          ) : null}
        </div>
      ) : null}
    </form>
  );
}
