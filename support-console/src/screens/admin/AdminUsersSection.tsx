import { useEffect, useState } from "react";
import { useQueryClient } from "@tanstack/react-query";
import { ApiError } from "../../api/client";
import type { AdminUserRecord as AdminUserRecordType, SupportAccessGrant } from "../../api/types";
import type { SupportRole } from "../../auth/session";
import { cardClass, sectionTitleClass } from "../../components/ui";
import { useAdminLookup } from "../../hooks/useAdminUsers";
import { AdminApprovals } from "./AdminApprovals";
import { AdminUserLookup } from "./AdminUserLookup";
import { AdminUserRecord } from "./AdminUserRecord";

export interface AdminUsersSectionProps {
  token: string;
  role: SupportRole;
  currentUserId: string;
  signedInEmail?: string;
  onGrantChanged: (grant: SupportAccessGrant) => void;
}

/**
 * The administrator's user surface: an exact-email lookup (the address travels
 * in the body, D-S1), the read-only record with its two audited change
 * controls, and the pending-grant approval queue. Only administrators reach
 * it, so it is rendered lazily by the shell and leaves no looked-up address in
 * the query cache once it unmounts.
 */
export function AdminUsersSection({
  token,
  role,
  currentUserId,
  signedInEmail,
  onGrantChanged,
}: AdminUsersSectionProps) {
  const queryClient = useQueryClient();
  const [email, setEmail] = useState<string | null>(null);
  const [record, setRecord] = useState<AdminUserRecordType | null>(null);
  const [notFound, setNotFound] = useState(false);

  const lookup = useAdminLookup(token, email ?? "", email !== null);

  useEffect(
    () => () => {
      queryClient.removeQueries({ queryKey: ["admin"] });
    },
    [queryClient],
  );

  useEffect(() => {
    if (lookup.data) {
      setRecord(lookup.data);
      setNotFound(false);
      return;
    }
    if (lookup.error instanceof ApiError && lookup.error.problem.status === 404) {
      setRecord(null);
      setNotFound(true);
      return;
    }
    if (lookup.error) {
      setRecord(null);
      setNotFound(false);
    }
  }, [lookup.data, lookup.error]);

  if (role !== "admin") {
    return null;
  }

  function submitLookup(nextEmail: string) {
    setRecord(null);
    setNotFound(false);
    setEmail(nextEmail);
  }

  function retryLookup() {
    void lookup.refetch();
  }

  function refetchRecord() {
    void lookup.refetch();
  }

  return (
    <section
      data-testid="admin-users-section"
      aria-labelledby="admin-users-title"
      className="flex flex-col gap-4"
    >
      <div className={cardClass}>
        <p className="text-overline font-medium uppercase tracking-overline text-meta">
          Administration
        </p>
        <h2 id="admin-users-title" className={`${sectionTitleClass} mt-1`}>
          Admin users
        </h2>
        <p className="mt-2 text-small text-ink-2">
          Look a user up by the exact address they signed up with, then change their role or account
          status. Every change is audited.
        </p>

        <div className="mt-4">
          <AdminUserLookup
            onSubmit={submitLookup}
            loading={lookup.isFetching}
            error={lookup.error}
            notFound={notFound}
            onRetry={email ? retryLookup : undefined}
          />
        </div>

        {record ? (
          <AdminUserRecord
            record={record}
            signedInEmail={signedInEmail}
            token={token}
            onChanged={refetchRecord}
          />
        ) : null}
      </div>

      <AdminApprovals
        currentUserId={currentUserId}
        role={role}
        token={token}
        onChanged={onGrantChanged}
      />
    </section>
  );
}

export default AdminUsersSection;
