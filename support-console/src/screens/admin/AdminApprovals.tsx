import { ApiError, NetworkError } from "../../api/client";
import type { SupportAccessGrant } from "../../api/types";
import type { SupportRole } from "../../auth/session";
import { GrantApprovalQueue } from "../../components/GrantApprovalQueue";
import { alertClass, cardClass, sectionTitleClass } from "../../components/ui";
import { usePendingGrants } from "../../hooks/useAdminUsers";

export interface AdminApprovalsProps {
  currentUserId: string;
  role: SupportRole;
  token: string;
  onChanged: (grant: SupportAccessGrant) => void;
}

function errorMessage(error: unknown): string {
  if (error instanceof NetworkError) {
    return "Could not reach the gateway. Check your connection and try again.";
  }
  if (error instanceof ApiError && error.problem.status === 403) {
    return "Your session is not an administrator.";
  }
  return "The pending grants could not be read.";
}

/**
 * The approvals panel reads the pending grants from the route (W5), not from
 * the shell's session-scoped array: a fresh admin session must see the
 * requests it did not create. A failure is an inline error with a retry, never
 * an empty queue that would read as "nothing is waiting".
 */
export function AdminApprovals({ currentUserId, role, token, onChanged }: AdminApprovalsProps) {
  const query = usePendingGrants(token);
  const problem = query.error instanceof ApiError ? query.error.problem : null;

  if (query.error) {
    return (
      <section data-testid="admin-approvals" className={cardClass}>
        <h3 className={sectionTitleClass}>Approvals</h3>
        <div data-testid="admin-approvals-error" role="alert" className={`${alertClass} mt-3`}>
          <p>{errorMessage(query.error)}</p>
          {problem ? (
            <p className="mt-1 font-mono text-meta">Correlation ID {problem.correlationId}</p>
          ) : null}
          <button
            type="button"
            data-testid="admin-approvals-retry"
            onClick={() => void query.refetch()}
            className="mt-2 underline"
          >
            Try again
          </button>
        </div>
      </section>
    );
  }

  if (query.isPending) {
    return (
      <section data-testid="admin-approvals" className={cardClass}>
        <h3 className={sectionTitleClass}>Approvals</h3>
        <p role="status" className="mt-3 text-small text-muted">
          Reading pending grants…
        </p>
      </section>
    );
  }

  return (
    <section data-testid="admin-approvals" className="flex flex-col gap-4">
      <GrantApprovalQueue
        grants={query.data ?? []}
        currentUserId={currentUserId}
        role={role}
        token={token}
        onChanged={(grant) => {
          onChanged(grant);
          void query.refetch();
        }}
      />
    </section>
  );
}
