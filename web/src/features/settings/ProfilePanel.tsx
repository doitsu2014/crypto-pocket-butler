import { useState } from "react";
import { Loading, PageHeader, StatusPill } from "../../components";
import { useI18n } from "../../i18n/react";
import { useProfile, useSaveDisplayName, useUseProviderName } from "../../api/settingsQueries";
import { correlationOf, fieldErrorsOf, isConflict, problemMessage } from "./problemText";

/**
 * Profile panel (functional-spec §3.1, P1/P2). The input owns a draft only
 * while it is dirty; after a save the panel re-renders from the server's
 * answer, so the input is never the source of truth.
 */
export default function ProfilePanel() {
  const { t } = useI18n();
  const profileQuery = useProfile();
  const saveName = useSaveDisplayName();
  const useProvider = useUseProviderName();

  const [draft, setDraft] = useState<string | null>(null);
  const [saveError, setSaveError] = useState<unknown>(null);
  const [resetError, setResetError] = useState<unknown>(null);

  if (profileQuery.isLoading) {
    return <Loading />;
  }

  if (profileQuery.isError || !profileQuery.data) {
    return (
      <section role="alert" data-testid="settings-profile-error" className="text-small text-danger">
        <p>{problemMessage(profileQuery.error, t)}</p>
        <button
          type="button"
          data-testid="settings-profile-retry"
          onClick={() => void profileQuery.refetch()}
          className="mt-2 rounded-lg bg-surface-warm px-3 py-1 font-medium text-ink"
        >
          {t("common.retry")}
        </button>
      </section>
    );
  }

  const profile = profileQuery.data;
  const value = draft ?? profile.displayName ?? "";
  const fieldErrors = fieldErrorsOf(saveError);

  async function onSave() {
    setSaveError(null);
    try {
      await saveName.mutateAsync(value.trim());
      setDraft(null);
    } catch (error) {
      setSaveError(error);
    }
  }

  async function onUseProviderName() {
    setResetError(null);
    try {
      await useProvider.mutateAsync();
      setDraft(null);
    } catch (error) {
      setResetError(error);
    }
  }

  return (
    <section className="grid gap-8">
      <PageHeader title={t("settings.profile.title")} />

      <div className="grid gap-2">
        <label htmlFor="settings-display-name" className="text-small font-medium text-ink">
          {t("settings.profile.displayName")}
        </label>
        <input
          id="settings-display-name"
          data-testid="settings-display-name"
          value={value}
          maxLength={200}
          onChange={(event) => setDraft(event.target.value)}
          aria-describedby={fieldErrors.displayName ? "settings-name-error" : undefined}
          className="w-full max-w-sm rounded-lg border border-border bg-surface px-3 py-2 text-small text-ink"
        />
        {fieldErrors.displayName ? (
          <p
            id="settings-name-error"
            data-testid="settings-name-error"
            className="text-small text-danger"
          >
            {fieldErrors.displayName}
          </p>
        ) : saveError ? (
          <p data-testid="settings-name-error" className="text-small text-danger">
            {problemMessage(saveError, t)}
            {correlationOf(saveError) ? (
              <span className="ml-2 text-meta">
                {t("common.correlationId")}: {correlationOf(saveError)}
              </span>
            ) : null}
          </p>
        ) : null}
        <div className="mt-1 flex flex-wrap gap-2">
          <button
            type="button"
            data-testid="settings-profile-save"
            disabled={saveName.isPending}
            onClick={() => void onSave()}
            className="w-fit rounded-lg bg-accent px-4 py-2 text-small font-medium text-accent-on disabled:opacity-45"
          >
            {t("settings.profile.save")}
          </button>
          {profile.displayNameOverridden ? (
            <button
              type="button"
              data-testid="settings-reset-name"
              disabled={useProvider.isPending || profile.providerDisplayName === null}
              onClick={() => void onUseProviderName()}
              className="w-fit rounded-lg bg-surface-warm px-4 py-2 text-small font-medium text-ink disabled:opacity-45"
            >
              {t("settings.profile.resetName")}
            </button>
          ) : null}
        </div>
        {profile.displayNameOverridden && profile.providerDisplayName === null ? (
          <p data-testid="settings-reset-hint" className="text-small text-muted">
            {t("settings.profile.resetHint")}
          </p>
        ) : null}
        {resetError ? (
          isConflict(resetError) ? (
            <p data-testid="settings-reset-conflict" role="status" className="text-small text-warn">
              {t("settings.profile.resetConflict")}
            </p>
          ) : (
            <p data-testid="settings-reset-error" className="text-small text-danger">
              {problemMessage(resetError, t)}
            </p>
          )
        ) : null}
      </div>

      <div className="grid gap-1">
        <span className="text-small font-medium text-ink">{t("settings.profile.email")}</span>
        <p data-testid="settings-email" className="text-small text-ink-2">
          {profile.email ?? "—"}
        </p>
        {profile.emailVerified ? (
          <span data-testid="settings-email-verified" className="text-small text-success">
            {t("settings.profile.verified")}
          </span>
        ) : null}
      </div>

      <div className="grid gap-1">
        <span className="text-small font-medium text-ink">{t("settings.profile.status")}</span>
        <StatusPill status={profile.status} />
      </div>
    </section>
  );
}
