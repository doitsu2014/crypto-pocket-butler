import type { AdminUserRecord as AdminUserRecordType, AdminUserStatus } from "../../api/types";
import { RoleBadge } from "../../components/RoleBadge";
import { StatusBadge } from "../../components/StatusBadge";
import { cardClass, overlineClass, sectionTitleClass } from "../../components/ui";
import type { StatusPresentation } from "../../domain/presentation";
import { RoleChangeForm } from "./RoleChangeForm";
import { StatusChangePanel } from "./StatusChangePanel";

const statusPresentation: Record<AdminUserStatus, StatusPresentation> = {
  pending: { label: "Pending", tone: "neutral", shape: "ring" },
  active: { label: "Active", tone: "success", shape: "dot" },
  disabled: { label: "Disabled", tone: "warn", shape: "square" },
  pending_deletion: { label: "Pending deletion", tone: "danger", shape: "diamond" },
  deleted: { label: "Deleted", tone: "neutral", shape: "square" },
};

const dateFormat = new Intl.DateTimeFormat(undefined, { dateStyle: "medium", timeStyle: "short" });

function formatTimestamp(value: string): string {
  const parsed = Date.parse(value);
  return Number.isFinite(parsed) ? dateFormat.format(parsed) : "—";
}

export interface AdminUserRecordProps {
  record: AdminUserRecordType;
  signedInEmail?: string;
  token: string;
  onChanged: () => void;
}

/**
 * The read-only record plus the two change controls. The section owns the
 * record and re-reads it after every successful change; nothing here is
 * cached optimistically (NFR24: a change takes effect at the target's next
 * request, so the console shows only what the server answers).
 */
export function AdminUserRecord({ record, signedInEmail, token, onChanged }: AdminUserRecordProps) {
  return (
    <article
      data-testid="admin-user-record"
      className={`${cardClass} mt-4`}
      aria-labelledby="admin-user-record-title"
    >
      <p className={overlineClass}>User record</p>
      <div className="mt-1 flex flex-wrap items-center justify-between gap-2">
        <h3
          id="admin-user-record-title"
          data-testid="admin-user-name"
          className={sectionTitleClass}
        >
          {record.displayName ?? "—"}
        </h3>
        <div className="flex flex-wrap items-center gap-2">
          <RoleBadge role={record.role} />
          <StatusBadge
            presentation={statusPresentation[record.status]}
            testId="admin-user-status"
          />
        </div>
      </div>

      <dl className="mt-4 grid gap-3 sm:grid-cols-2">
        <div>
          <dt className={overlineClass}>Email</dt>
          <dd data-testid="admin-user-email" className="text-small text-ink">
            {record.email ?? "—"}
          </dd>
        </div>
        <div>
          <dt className={overlineClass}>Email state</dt>
          <dd data-testid="admin-user-verified" className="text-small text-ink">
            {record.emailVerified ? "Verified" : "Unverified"}
          </dd>
        </div>
        <div>
          <dt className={overlineClass}>User id</dt>
          <dd data-testid="admin-user-id" className="font-mono text-small text-ink">
            {record.id}
          </dd>
        </div>
        <div>
          <dt className={overlineClass}>Created</dt>
          <dd data-testid="admin-user-created" className="text-small text-ink">
            {formatTimestamp(record.createdAt)}
          </dd>
        </div>
        <div>
          <dt className={overlineClass}>Updated</dt>
          <dd data-testid="admin-user-updated" className="text-small text-ink">
            {formatTimestamp(record.updatedAt)}
          </dd>
        </div>
      </dl>

      <div className="mt-6 flex flex-col gap-6">
        <RoleChangeForm
          record={record}
          signedInEmail={signedInEmail}
          token={token}
          onChanged={onChanged}
        />
        <StatusChangePanel record={record} token={token} onChanged={onChanged} />
      </div>
    </article>
  );
}
