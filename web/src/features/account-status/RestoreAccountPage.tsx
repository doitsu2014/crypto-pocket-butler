import { useState } from "react";
import { Navigate, useNavigate } from "react-router-dom";
import { Loading } from "../../components";
import { signOut as defaultSignOut } from "../../auth/session";
import { useI18n } from "../../i18n/react";
import { useAccountStatus, useCancelDeletion, useDeletionRequest } from "../../api/settingsQueries";
import { isGone, problemMessage } from "../settings/problemText";

/**
 * Restore account (`/account/restore`; functional-spec §3.6, D1/D2/D0).
 *
 * The banner's primary content is the exact date the account is erased. A
 * `410` closes the window for good; a `404` means another tab already restored
 * or the erasure finished, so the page re-resolves D0 and routes by status.
 */
export default function RestoreAccountPage({ onSignOut }: { onSignOut?: () => void } = {}) {
  const { t, format } = useI18n();
  const navigate = useNavigate();
  const deletionQuery = useDeletionRequest();
  const statusQuery = useAccountStatus();
  const cancelDeletion = useCancelDeletion();
  const [closed, setClosed] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const signOut = onSignOut ?? (() => void defaultSignOut());

  if (deletionQuery.isLoading || (deletionQuery.data === null && statusQuery.isLoading)) {
    return <Loading />;
  }

  if (deletionQuery.isError) {
    return (
      <section role="alert" data-testid="restore-error" className="text-small text-danger">
        <p>{problemMessage(deletionQuery.error, t)}</p>
        <button
          type="button"
          data-testid="restore-retry"
          onClick={() => void deletionQuery.refetch()}
          className="mt-2 rounded-lg bg-surface-warm px-3 py-1 font-medium text-ink"
        >
          {t("common.retry")}
        </button>
      </section>
    );
  }

  const deletion = deletionQuery.data ?? null;

  if (!deletion && !closed) {
    const status = statusQuery.data?.status;
    if (status === "active") {
      return <Navigate to="/settings/profile" replace />;
    }
    if (status === "disabled" || status === "deleted") {
      return <Navigate to="/account/blocked" replace />;
    }
    if (status === "pending_deletion" || statusQuery.isError) {
      return (
        <section className="grid gap-4" data-testid="restore-no-request">
          <p className="text-small text-ink-2">{t("settings.restore.noRequest")}</p>
          <button
            type="button"
            data-testid="restore-sign-out"
            onClick={signOut}
            className="w-fit rounded-lg bg-surface-warm px-4 py-2 text-small font-medium text-ink"
          >
            {t("settings.restore.signOut")}
          </button>
        </section>
      );
    }
    return <Loading />;
  }

  async function onRestore() {
    setError(null);
    try {
      await cancelDeletion.mutateAsync();
      navigate("/settings/profile");
    } catch (caught) {
      if (isGone(caught)) {
        setClosed(true);
        return;
      }
      // A lost answer is re-read through D1 before it is reported: if the
      // request is gone, the page re-resolves from D0 on the next render
      // (active → profile, disabled/deleted → blocked, pending → no request)
      // instead of inventing a failure another tab already resolved.
      const refreshed = await deletionQuery.refetch();
      if (refreshed.data === null) {
        return;
      }
      setError(problemMessage(caught, t));
    }
  }

  if (closed) {
    return (
      <section className="grid gap-4">
        <div
          data-testid="restore-closed"
          role="status"
          aria-live="polite"
          className="rounded-lg border border-border bg-surface p-4 text-small text-ink"
        >
          <p>
            {t("settings.restore.closed", {
              date: deletion ? format(deletion.recoveryEndsAt) : "—",
            })}
          </p>
        </div>
        <button
          type="button"
          data-testid="restore-sign-out"
          onClick={signOut}
          className="w-fit rounded-lg bg-surface-warm px-4 py-2 text-small font-medium text-ink"
        >
          {t("settings.restore.signOut")}
        </button>
      </section>
    );
  }

  return (
    <section className="grid gap-6">
      <div
        data-testid="recovery-banner"
        role="status"
        aria-live="polite"
        className="rounded-lg border border-warn bg-surface-warm p-4 text-small text-ink"
      >
        <p>
          {t("settings.restore.banner", {
            date: deletion ? format(deletion.recoveryEndsAt) : "—",
          })}
        </p>
      </div>

      <div className="flex flex-wrap items-center gap-3">
        <button
          type="button"
          data-testid="restore-account"
          disabled={cancelDeletion.isPending}
          onClick={() => void onRestore()}
          className="w-fit rounded-lg bg-accent px-4 py-2 text-small font-medium text-accent-on disabled:opacity-45"
        >
          {t("settings.restore.action")}
        </button>
        <button
          type="button"
          data-testid="restore-sign-out"
          onClick={signOut}
          className="w-fit rounded-lg bg-surface-warm px-4 py-2 text-small font-medium text-ink"
        >
          {t("settings.restore.signOut")}
        </button>
      </div>

      {error ? (
        <p role="alert" data-testid="restore-action-error" className="text-small text-danger">
          {error}
        </p>
      ) : null}
    </section>
  );
}
