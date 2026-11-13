import { useId, useState, type FormEvent } from "react";
import { ApiError, NetworkError } from "../../api/client";
import type { AdminUserRecord, ConsoleRole, RoleChangeResult } from "../../api/types";
import {
  alertClass,
  fieldErrorClass,
  inputClass,
  labelClass,
  primaryButtonClass,
} from "../../components/ui";
import { CONSOLE_ROLES, useChangeRole } from "../../hooks/useAdminUsers";

export interface RoleChangeFormProps {
  record: AdminUserRecord;
  signedInEmail?: string;
  token: string;
  onChanged: (result: RoleChangeResult) => void;
}

const MAX_REASON = 500;

function sessionLine(): string {
  return "Your session is not an administrator.";
}

/**
 * A role change is audited and takes effect at the target's next request
 * (NFR24), so the form states the transition and never claims the change
 * landed until the server answers with its previous and new role.
 */
export function RoleChangeForm({ record, signedInEmail, token, onChanged }: RoleChangeFormProps) {
  const roleId = useId();
  const reasonId = useId();
  const reasonErrorId = useId();
  const noticeId = useId();
  const [newRole, setNewRole] = useState<ConsoleRole>(record.role);
  const [reason, setReason] = useState("");
  const [note, setNote] = useState<{
    tone: "ok" | "error";
    text: string;
    correlationId?: string;
  } | null>(null);
  const [roleError, setRoleError] = useState<string | null>(null);
  const [reasonError, setReasonError] = useState<string | null>(null);
  const [result, setResult] = useState<RoleChangeResult | null>(null);

  const changeRole = useChangeRole(token);
  const isSelf = Boolean(record.email && signedInEmail && record.email === signedInEmail);
  const roleDiffers = newRole !== record.role;
  const reasonTrimmed = reason.trim();
  const canSubmit = roleDiffers && reasonTrimmed.length > 0 && reasonTrimmed.length <= MAX_REASON;

  function submit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setRoleError(null);
    setReasonError(null);
    setNote(null);
    setResult(null);

    if (!roleDiffers) {
      setRoleError("Choose a different role.");
      return;
    }
    if (reasonTrimmed.length === 0 || reasonTrimmed.length > MAX_REASON) {
      setReasonError("Enter a reason of up to 500 characters.");
      return;
    }

    changeRole.mutate(
      { userId: record.id, body: { newRole, reason: reasonTrimmed } },
      {
        onSuccess: (answer) => {
          setResult(answer);
          setNote({ tone: "ok", text: "This change is audited." });
          onChanged(answer);
        },
        onError: (error) => {
          if (error instanceof NetworkError) {
            setNote({
              tone: "error",
              text: "Could not reach the gateway. Check your connection and try again.",
            });
            return;
          }
          if (error instanceof ApiError) {
            const fields = error.problem.errors ?? [];
            const roleField = fields.find((item) => item.field === "newRole");
            const reasonField = fields.find((item) => item.field === "reason");
            if (roleField) {
              setRoleError(roleField.reason);
            }
            if (reasonField) {
              setReasonError(reasonField.reason);
            }
            const text =
              error.problem.status === 403
                ? sessionLine()
                : (roleField?.reason ??
                  fields[0]?.reason ??
                  "The role could not be changed. Nothing was saved.");
            setNote({ tone: "error", text, correlationId: error.problem.correlationId });
            return;
          }
          setNote({ tone: "error", text: "The role could not be changed. Nothing was saved." });
        },
      },
    );
  }

  return (
    <form
      data-testid="admin-role-form"
      onSubmit={submit}
      noValidate
      className="flex flex-col gap-3"
    >
      <h4 className="text-small font-semibold text-ink">Role</h4>

      {isSelf ? (
        <p data-testid="admin-role-self-warning" className="text-small text-warn">
          This is the account you are signed in with.
        </p>
      ) : null}

      <label htmlFor={roleId} className={labelClass}>
        New role
        <select
          id={roleId}
          data-testid="admin-role-select"
          className={inputClass}
          value={newRole}
          onChange={(event) => setNewRole(event.target.value as ConsoleRole)}
          disabled={changeRole.isPending}
        >
          {CONSOLE_ROLES.map((role) => (
            <option key={role} value={role}>
              {role}
            </option>
          ))}
        </select>
      </label>

      <label htmlFor={reasonId} className={labelClass}>
        Reason
        <textarea
          id={reasonId}
          data-testid="admin-role-reason"
          className={`${inputClass} min-h-20`}
          value={reason}
          maxLength={MAX_REASON}
          onChange={(event) => setReason(event.target.value)}
          disabled={changeRole.isPending}
          aria-invalid={reasonError ? true : undefined}
          aria-describedby={reasonError ? reasonErrorId : undefined}
        />
      </label>
      {reasonError ? (
        <p id={reasonErrorId} data-testid="admin-role-reason-error" className={fieldErrorClass}>
          {reasonError}
        </p>
      ) : null}

      <div>
        <button
          type="submit"
          data-testid="admin-role-submit"
          className={primaryButtonClass}
          disabled={!canSubmit || changeRole.isPending}
        >
          {changeRole.isPending ? "Changing role…" : "Change role"}
        </button>
      </div>

      {note?.tone === "ok" && result ? (
        <p
          id={noticeId}
          data-testid="admin-role-result"
          role="status"
          aria-live="polite"
          className="text-small text-ink-2"
        >
          {result.previousRole} → {result.newRole}. {note.text}
        </p>
      ) : null}

      {note?.tone === "error" ? (
        <div data-testid="admin-role-error" role="alert" className={alertClass}>
          <p>{note.text}</p>
          {note.correlationId ? (
            <p className="mt-1 font-mono text-meta">Correlation ID {note.correlationId}</p>
          ) : null}
        </div>
      ) : roleError ? (
        <p data-testid="admin-role-error" className={fieldErrorClass}>
          {roleError}
        </p>
      ) : null}
    </form>
  );
}
