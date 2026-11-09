import { useState } from "react";
import { Link } from "react-router-dom";
import { gatewayClient } from "../../api/client";
import { problemMessages } from "../../api/errors";
import { useProfile, useSessions } from "../../api/queries";
import type { Profile, SessionInfo } from "../../api/types";
import { ErrorPanel, Loading, PageHeader, QueryBoundary } from "../../components";
import { useI18n } from "../../i18n/react";

export default function SettingsPage({
  profileOverride,
  sessionsOverride,
  updateProfile,
  revokeSession,
}: {
  profileOverride?: Profile;
  sessionsOverride?: SessionInfo[];
  updateProfile?: (patch: { displayName?: string }) => Promise<Profile>;
  revokeSession?: (sessionId: string) => Promise<void>;
} = {}) {
  const { t, format } = useI18n();
  const profileQuery = useProfile(!profileOverride);
  const sessionsQuery = useSessions(!sessionsOverride);

  const profile = profileOverride ?? profileQuery.data;
  const sourceSessions = sessionsOverride ?? sessionsQuery.data ?? [];

  const [nameDraft, setNameDraft] = useState<string | null>(null);
  const displayName = nameDraft ?? profile?.displayName ?? "";
  const [revoked, setRevoked] = useState<string[]>([]);
  const [saveError, setSaveError] = useState<string[]>([]);
  const [saved, setSaved] = useState(false);
  const [sessionError, setSessionError] = useState<unknown>(null);

  const sessions = sourceSessions.filter((session) => !revoked.includes(session.id));

  async function onSaveProfile() {
    setSaveError([]);
    setSaved(false);
    try {
      const next = updateProfile
        ? await updateProfile({ displayName })
        : await gatewayClient.updateProfile({ displayName });
      setNameDraft(next.displayName ?? displayName);
      setSaved(true);
    } catch (error) {
      setSaveError(problemMessages(error));
    }
  }

  async function onRevoke(session: SessionInfo) {
    setSessionError(null);
    try {
      if (revokeSession) {
        await revokeSession(session.id);
      } else {
        await gatewayClient.revokeSession(session.id);
      }
      setRevoked((current) => [...current, session.id]);
    } catch (error) {
      setSessionError(error);
    }
  }

  if (profileQuery.isLoading && !profileOverride) {
    return <Loading />;
  }
  if (profileQuery.error) {
    return <ErrorPanel error={profileQuery.error} onRetry={() => void profileQuery.refetch()} />;
  }

  return (
    <section className="grid gap-8">
      <PageHeader title={t("settings.title")} />

      <div>
        <h2 className="mb-2 font-display text-section font-medium text-ink">
          {t("settings.profile")}
        </h2>
        <div
          data-testid="settings-profile"
          className="grid gap-3 rounded-lg border border-border bg-surface p-4"
        >
          <p data-testid="settings-profile-email" className="text-small text-ink-2">
            {t("auth.email")}: <span className="text-ink">{profile?.email ?? "—"}</span>
          </p>
          <label className="flex flex-col gap-1 text-small font-medium text-ink-2">
            {t("settings.displayName")}
            <input
              data-testid="settings-display-name"
              value={displayName}
              onChange={(event) => setNameDraft(event.target.value)}
              className="max-w-sm rounded-xl border border-border bg-surface px-3 py-2 text-small text-ink"
            />
          </label>
          <button
            type="button"
            data-testid="settings-profile-save"
            onClick={() => void onSaveProfile()}
            className="w-fit rounded-lg bg-accent px-4 py-2 text-small font-medium text-accent-on"
          >
            {t("common.save")}
          </button>
          {saved ? (
            <p data-testid="settings-saved" role="status" className="text-small text-success">
              {t("settings.saved")}
            </p>
          ) : null}
          {saveError.length > 0 ? (
            <ul data-testid="settings-errors" role="alert" className="text-small text-danger">
              {saveError.map((message) => (
                <li key={message}>{message}</li>
              ))}
            </ul>
          ) : null}
        </div>
      </div>

      <div>
        <h2 className="mb-2 font-display text-section font-medium text-ink">
          {t("settings.security")}
        </h2>
        <div
          data-testid="settings-mfa-status"
          className="flex flex-wrap items-center justify-between gap-2 rounded-lg border border-border bg-surface p-4"
        >
          <p className="text-small text-ink-2">
            {t("settings.mfa")}:{" "}
            <span className="text-ink">
              {profile?.mfaEnrolled
                ? t("settings.mfaEnrolled")
                : profile?.mfaRequired
                  ? t("settings.mfaRequired")
                  : t("common.unavailable")}
            </span>
          </p>
          <Link
            to="/settings/mfa"
            data-testid="settings-mfa-link"
            className="rounded-lg bg-surface-warm px-3 py-1 text-small font-medium text-ink"
          >
            {t("settings.enrolMfa")}
          </Link>
        </div>
      </div>

      <div>
        <h2 className="mb-2 font-display text-section font-medium text-ink">
          {t("settings.sessions")}
        </h2>
        {sessionError ? <ErrorPanel error={sessionError} /> : null}
        <QueryBoundary
          loading={sessionsQuery.isLoading && !sessionsOverride}
          error={sessionsQuery.error}
          onRetry={() => void sessionsQuery.refetch()}
        >
          <ul className="mt-2 grid gap-2">
            {sessions.map((session) => (
              <li
                key={session.id}
                data-testid="settings-session-row"
                className="flex flex-wrap items-center justify-between gap-2 rounded-lg border border-border bg-surface p-3"
              >
                <div>
                  <p className="text-small font-medium text-ink">{session.device}</p>
                  <p className="text-small text-meta">
                    {session.location} · {format(session.lastActiveAt)}
                  </p>
                </div>
                <div className="flex items-center gap-2">
                  {session.current ? (
                    <span
                      data-testid="settings-current"
                      className="rounded-full bg-surface-warm px-2 py-0.5 text-overline uppercase tracking-overline text-ink-2"
                    >
                      {t("settings.currentSession")}
                    </span>
                  ) : (
                    <button
                      type="button"
                      data-testid={`settings-revoke-${session.id}`}
                      onClick={() => void onRevoke(session)}
                      className="rounded-lg bg-surface-warm px-3 py-1 text-small font-medium text-ink"
                    >
                      {t("settings.revokeSession")}
                    </button>
                  )}
                </div>
              </li>
            ))}
          </ul>
        </QueryBoundary>
      </div>
    </section>
  );
}
