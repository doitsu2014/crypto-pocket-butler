import { useState } from "react";
import { useNavigate } from "react-router-dom";
import { ConfirmDialog, Loading, PageHeader } from "../../components";
import { useI18n } from "../../i18n/react";
import {
  useCancelDeletion,
  useDeletionRequest,
  useRequestDeletion,
} from "../../api/settingsQueries";
import { OpenRequestNotice } from "./OpenRequestNotice";
import { correlationOf, isGone, problemMessage } from "./problemText";

/**
 * Delete account page (`/settings/delete`; functional-spec §3.5, D1/D2/D3).
 *
 * No open request: consequences, the retained-records sentence, and one
 * destructive control gated by a confirmation dialog. Open request: the shared
 * notice with the exact recovery date and Restore.
 */
export default function DeleteAccountPage() {
  const { t } = useI18n();
  const navigate = useNavigate();
  const deletionQuery = useDeletionRequest();
  const requestDeletion = useRequestDeletion();
  const cancelDeletion = useCancelDeletion();
  const [confirmOpen, setConfirmOpen] = useState(false);
  const [actionError, setActionError] = useState<string | null>(null);

  if (deletionQuery.isLoading) {
    return <Loading />;
  }

  if (deletionQuery.isError) {
    return (
      <section role="alert" data-testid="delete-error" className="text-small text-danger">
        <p>{problemMessage(deletionQuery.error, t)}</p>
        <button
          type="button"
          data-testid="delete-retry"
          onClick={() => void deletionQuery.refetch()}
          className="mt-2 rounded-lg bg-surface-warm px-3 py-1 font-medium text-ink"
        >
          {t("common.retry")}
        </button>
      </section>
    );
  }

  const deletion = deletionQuery.data ?? null;

  async function onConfirm() {
    setActionError(null);
    try {
      await requestDeletion.mutateAsync();
      setConfirmOpen(false);
    } catch (error) {
      setConfirmOpen(false);
      // D3 answers 409 when a request is already open, and a lost answer may
      // still have created one: re-read D1 before showing anything, so an open
      // request renders as the notice with its recovery date rather than as an
      // error (functional-spec §10, NFR2.5, NFR24.5).
      const refreshed = await deletionQuery.refetch();
      if (refreshed.data) {
        return;
      }
      setActionError(
        `${problemMessage(error, t)}${
          correlationOf(error) ? ` · ${t("common.correlationId")}: ${correlationOf(error)}` : ""
        }`,
      );
    }
  }

  async function onRestore() {
    setActionError(null);
    try {
      await cancelDeletion.mutateAsync();
      navigate("/settings/profile");
    } catch (error) {
      // The same re-read applies to D2: a lost answer may still have cancelled
      // the request, so D1 decides what the page shows. A `410` is not a lost
      // answer — the window closed — and stays visible as the failure it is.
      const refreshed = await deletionQuery.refetch();
      if (refreshed.data === null && !isGone(error)) {
        return;
      }
      setActionError(
        `${problemMessage(error, t)}${
          correlationOf(error) ? ` · ${t("common.correlationId")}: ${correlationOf(error)}` : ""
        }`,
      );
    }
  }

  return (
    <section className="grid gap-8">
      <PageHeader title={t("settings.delete.title")} />

      {deletion ? (
        <OpenRequestNotice
          request={deletion}
          onRestore={() => void onRestore()}
          pending={cancelDeletion.isPending}
          error={actionError}
        />
      ) : (
        <>
          <div data-testid="delete-consequences" className="grid gap-2">
            <h2 className="font-display text-section font-medium text-ink">
              {t("settings.delete.consequencesTitle")}
            </h2>
            <ul className="grid list-disc gap-1 pl-5 text-small text-ink-2">
              <li>{t("settings.delete.consequenceWindow")}</li>
              <li>{t("settings.delete.consequenceErase")}</li>
              <li>{t("settings.delete.consequenceRestore")}</li>
            </ul>
          </div>

          <div className="grid gap-2">
            <h2 className="font-display text-section font-medium text-ink">
              {t("settings.delete.retainedTitle")}
            </h2>
            <p data-testid="deletion-retained-policy" className="max-w-prose text-small text-muted">
              {t("settings.delete.retained")}
            </p>
          </div>

          <div className="flex flex-wrap items-center gap-3">
            <button
              type="button"
              data-testid="settings-delete-open"
              onClick={() => setConfirmOpen(true)}
              className="w-fit rounded-lg bg-danger px-4 py-2 text-small font-medium text-accent-on"
            >
              {t("settings.delete.action")}
            </button>
          </div>

          {actionError ? (
            <p role="alert" data-testid="delete-action-error" className="text-small text-danger">
              {actionError}
            </p>
          ) : null}

          <ConfirmDialog
            open={confirmOpen}
            title={t("settings.delete.confirmTitle")}
            description={t("settings.delete.confirmBody")}
            confirmLabel={t("settings.delete.action")}
            onConfirm={() => void onConfirm()}
            onCancel={() => setConfirmOpen(false)}
          />
        </>
      )}
    </section>
  );
}
