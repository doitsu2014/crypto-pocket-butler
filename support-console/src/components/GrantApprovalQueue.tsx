import { useMutation } from "@tanstack/react-query";
import { ApiError, approveSupportGrant, NetworkError, revokeSupportGrant } from "../api/client";
import type { SupportAccessGrant } from "../api/types";
import type { SupportRole } from "../auth/session";
import { grantStatusPresentation } from "../domain/grants";
import { GATEWAY_UNREACHABLE_MESSAGE } from "../domain/presentation";
import { StatusBadge } from "./StatusBadge";
import {
  alertClass,
  cardClass,
  dangerButtonClass,
  overlineClass,
  primaryButtonClass,
  secondaryButtonClass,
  sectionTitleClass,
} from "./ui";

export interface GrantApprovalQueueProps {
  grants: SupportAccessGrant[];
  currentUserId: string;
  role: SupportRole;
  token: string | null;
  onChanged: (grant: SupportAccessGrant) => void;
}

export function GrantApprovalQueue({
  grants,
  currentUserId,
  role,
  token,
  onChanged,
}: GrantApprovalQueueProps) {
  const approve = useMutation<SupportAccessGrant, Error, string>({
    mutationFn: (grantId) => approveSupportGrant(grantId, token),
    onSuccess: (grant) => onChanged(grant),
  });

  const revoke = useMutation<void, Error, SupportAccessGrant>({
    mutationFn: (grant) => revokeSupportGrant(grant.id, token),
    onSuccess: (_result, grant) => onChanged({ ...grant, status: "revoked" }),
  });

  const error = approve.error ?? revoke.error;
  const problem = error instanceof ApiError ? error.problem : null;

  return (
    <section data-testid="grant-approval-queue" className={cardClass}>
      <p className={overlineClass}>Second approval</p>
      <h2 className={`${sectionTitleClass} mt-1`}>Grant approval queue</h2>

      {grants.length === 0 ? (
        <p data-testid="grant-approval-empty" className="mt-4 text-small text-ink-2">
          No pending grants. A requested grant appears here until a second approver accepts it.
        </p>
      ) : (
        <ul className="mt-4 flex flex-col gap-3">
          {grants.map((grant) => {
            const isOwnRequest = grant.requestedBy === currentUserId;
            const canApprove =
              role === "admin" && !isOwnRequest && grant.status === "pending_approval";
            return (
              <li
                key={grant.id}
                data-testid={`grant-approval-row-${grant.id}`}
                className="flex flex-col gap-2 rounded-lg border border-border-soft bg-surface-warm p-4"
              >
                <div className="flex flex-wrap items-center justify-between gap-2">
                  <p className="text-small font-medium text-ink">{grant.reason}</p>
                  <StatusBadge presentation={grantStatusPresentation(grant.status)} />
                </div>
                <p className="text-small text-ink-2">
                  Target user <span className="font-mono text-ink">{grant.userId}</span>
                </p>
                <p className="text-small text-ink-2">
                  Requested by{" "}
                  <span className="font-mono text-ink">{grant.requestedBy ?? "—"}</span>
                </p>
                {role !== "admin" ? (
                  <p className="text-small text-meta">Only an administrator can approve a grant.</p>
                ) : isOwnRequest ? (
                  <p className="text-small text-meta">
                    You cannot approve your own request. A second approver is required.
                  </p>
                ) : null}
                <div className="flex flex-wrap gap-2">
                  <button
                    type="button"
                    data-testid={`grant-approval-approve-${grant.id}`}
                    disabled={!canApprove || approve.isPending}
                    onClick={() => approve.mutate(grant.id)}
                    className={primaryButtonClass}
                  >
                    Approve
                  </button>
                  {role === "admin" && grant.status !== "revoked" && grant.status !== "expired" ? (
                    <button
                      type="button"
                      data-testid={`grant-approval-revoke-${grant.id}`}
                      disabled={revoke.isPending}
                      onClick={() => revoke.mutate(grant)}
                      className={`${dangerButtonClass} bg-surface`}
                    >
                      Revoke
                    </button>
                  ) : null}
                </div>
              </li>
            );
          })}
        </ul>
      )}

      {error ? (
        <p role="alert" className={`${alertClass} mt-4`}>
          {error instanceof NetworkError
            ? GATEWAY_UNREACHABLE_MESSAGE
            : (problem?.title ?? error.message)}
          {problem ? (
            <span className="ml-2 font-mono text-meta">Correlation ID {problem.correlationId}</span>
          ) : null}
        </p>
      ) : null}

      <div className="mt-4">
        <a href="#grants" className={`${secondaryButtonClass} inline-block`}>
          Back to grants
        </a>
      </div>
    </section>
  );
}
