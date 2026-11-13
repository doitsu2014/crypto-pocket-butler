import { Navigate } from "react-router-dom";
import { Loading } from "../../components";
import { signOut as defaultSignOut } from "../../auth/session";
import { useI18n } from "../../i18n/react";
import { useAccountStatus } from "../../api/settingsQueries";

/**
 * Blocked account (`/account/blocked`; functional-spec §3.7). Terminal, with
 * two variants chosen from the account status. No settings control and no
 * self-service path: re-enabling is an administrator action.
 */
export default function AccountBlockedPage({
  variant,
  onSignOut,
}: {
  variant?: "disabled" | "deleted";
  onSignOut?: () => void;
} = {}) {
  const { t } = useI18n();
  const statusQuery = useAccountStatus(!variant);
  const signOut = onSignOut ?? (() => void defaultSignOut());

  if (!variant) {
    if (statusQuery.isPending) {
      return <Loading />;
    }
    if (statusQuery.isError) {
      // An unresolved status is never rendered as "disabled": that sentence
      // would be a claim the server has not made, and the server's own detail
      // text stays out of the page.
      return (
        <section role="alert" data-testid="account-blocked-unresolved" className="grid gap-2 text-small text-danger">
          <p>{t("settings.blocked.unresolved")}</p>
          <button
            type="button"
            data-testid="account-blocked-retry"
            onClick={() => void statusQuery.refetch()}
            className="mt-2 rounded-lg bg-surface-warm px-3 py-1 font-medium text-ink"
          >
            {t("common.retry")}
          </button>
        </section>
      );
    }
    const status = statusQuery.data?.status;
    if (status === "active") {
      return <Navigate to="/settings/profile" replace />;
    }
    if (status === "pending_deletion") {
      // A pending-deletion principal restores from its own screen and never
      // sees the blocked page (functional-spec §3.7).
      return <Navigate to="/account/restore" replace />;
    }
    if (status !== "disabled" && status !== "deleted") {
      // An unresolved status is never rendered as "disabled": that sentence
      // would be a claim the server has not made.
      return <Loading />;
    }
  }

  const status = variant ?? statusQuery.data?.status;
  const resolved = status === "deleted" ? "deleted" : "disabled";

  return (
    <section className="grid gap-4">
      <div
        data-testid={`account-blocked-${resolved}`}
        role="status"
        className="rounded-lg border border-border bg-surface p-4 text-small text-ink"
      >
        <p className="font-display text-section font-medium">
          {t(`settings.blocked.${resolved}.title`)}
        </p>
        <p className="mt-1 text-ink-2">{t(`settings.blocked.${resolved}.body`)}</p>
      </div>

      {resolved === "disabled" ? (
        <p data-testid="account-blocked-support" className="max-w-prose text-small text-ink-2">
          {t("settings.blocked.disabled.support")}
        </p>
      ) : null}

      <button
        type="button"
        data-testid="account-blocked-sign-out"
        onClick={signOut}
        className="w-fit rounded-lg bg-surface-warm px-4 py-2 text-small font-medium text-ink"
      >
        {t("settings.restore.signOut")}
      </button>
    </section>
  );
}
