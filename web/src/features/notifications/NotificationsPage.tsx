import { useState } from "react";
import { gatewayClient } from "../../api/client";
import { problemMessages } from "../../api/errors";
import { useNotificationPreferences, useNotifications, useProfile } from "../../api/queries";
import type {
  NotificationItem,
  NotificationPreference,
  NotificationPreferences,
} from "../../api/types";
import { ErrorPanel, Loading, PageHeader } from "../../components";
import { useI18n } from "../../i18n/react";

function toggleChannel(
  preferences: NotificationPreferences,
  type: string,
  channel: "inApp" | "email",
  value: boolean,
): NotificationPreferences {
  return {
    types: preferences.types.map((entry) =>
      entry.type === type ? { ...entry, [channel]: value } : entry,
    ),
  };
}

export default function NotificationsPage({
  userId: userIdProp,
  notificationsOverride,
  preferencesOverride,
  savePreferences,
}: {
  userId?: string;
  notificationsOverride?: NotificationItem[];
  preferencesOverride?: NotificationPreferences;
  savePreferences?: (preferences: NotificationPreferences) => Promise<NotificationPreferences>;
} = {}) {
  const { t, format } = useI18n();
  const profile = useProfile(!userIdProp && !notificationsOverride && !preferencesOverride);
  const userId = userIdProp ?? profile.data?.userId;

  const feed = useNotifications(notificationsOverride ? undefined : userId);
  const preferencesQuery = useNotificationPreferences(preferencesOverride ? undefined : userId);

  const [draft, setDraft] = useState<NotificationPreferences | null>(null);
  const [errors, setErrors] = useState<string[]>([]);
  const [saved, setSaved] = useState(false);
  const [saving, setSaving] = useState(false);

  const preferences = draft ?? preferencesOverride ?? preferencesQuery.data ?? null;
  const items = notificationsOverride ?? feed.data ?? [];

  async function onSave() {
    setErrors([]);
    setSaved(false);
    if (!preferences || saving) {
      return;
    }
    setSaving(true);
    try {
      const next = savePreferences
        ? await savePreferences(preferences)
        : await gatewayClient.updateNotificationPreferences(userId as string, preferences);
      setDraft(next);
      setSaved(true);
    } catch (error) {
      setErrors(problemMessages(error));
    } finally {
      setSaving(false);
    }
  }

  return (
    <section className="grid gap-8">
      <PageHeader title={t("notifications.title")} />

      <div>
        <h2 className="mb-2 font-display text-section font-medium text-ink">
          {t("notifications.title")}
        </h2>
        {feed.isLoading && !notificationsOverride ? (
          <Loading />
        ) : feed.error ? (
          <ErrorPanel error={feed.error} onRetry={() => void feed.refetch()} />
        ) : items.length === 0 ? (
          <p data-testid="notifications-empty" className="text-small text-muted">
            {t("notifications.noNotifications")}
          </p>
        ) : (
          <ul className="grid gap-2">
            {items.map((item) => (
              <li
                key={item.id}
                data-testid="notification-row"
                className="rounded-lg border border-border bg-surface p-4"
              >
                <div className="flex flex-wrap items-center justify-between gap-2">
                  <h3 className="font-medium text-ink">{item.title}</h3>
                  <span className="text-small text-meta">{format(item.createdAt)}</span>
                </div>
                <p className="mt-1 text-small text-ink-2">{item.body}</p>
                {!item.read ? (
                  <span
                    data-testid="notification-unread"
                    className="mt-2 inline-block rounded-full bg-surface-warm px-2 py-0.5 text-overline uppercase tracking-overline text-ink-2"
                  >
                    {t("notifications.unread")}
                  </span>
                ) : null}
              </li>
            ))}
          </ul>
        )}
      </div>

      <div>
        <h2 className="mb-2 font-display text-section font-medium text-ink">
          {t("notifications.preferences")}
        </h2>
        {preferencesQuery.isLoading && !preferencesOverride ? (
          <Loading />
        ) : preferencesQuery.error ? (
          <ErrorPanel
            error={preferencesQuery.error}
            onRetry={() => void preferencesQuery.refetch()}
          />
        ) : preferences ? (
          <div data-testid="notification-preferences" className="grid gap-3">
            {preferences.types.map((entry: NotificationPreference) => (
              <div
                key={entry.type}
                className="flex flex-wrap items-center gap-4 rounded-lg border border-border bg-surface p-3"
              >
                <span className="min-w-40 font-medium text-ink">{entry.type}</span>
                <label className="flex items-center gap-2 text-small text-ink-2">
                  <input
                    type="checkbox"
                    data-testid={`pref-${entry.type}-inapp`}
                    checked={entry.inApp}
                    onChange={(event) =>
                      setDraft(
                        toggleChannel(preferences, entry.type, "inApp", event.target.checked),
                      )
                    }
                  />
                  {t("notifications.inApp")}
                </label>
                <label className="flex items-center gap-2 text-small text-ink-2">
                  <input
                    type="checkbox"
                    data-testid={`pref-${entry.type}-email`}
                    checked={entry.email}
                    disabled={entry.mandatory}
                    onChange={(event) =>
                      setDraft(
                        toggleChannel(preferences, entry.type, "email", event.target.checked),
                      )
                    }
                  />
                  {t("notifications.email")}
                </label>
                {entry.mandatory ? (
                  <span data-testid="pref-mandatory-lock" className="text-small text-warn">
                    {t("notifications.mandatoryLocked")}
                  </span>
                ) : null}
              </div>
            ))}

            <button
              type="button"
              data-testid="notifications-save"
              disabled={saving}
              onClick={() => void onSave()}
              className="w-fit rounded-lg bg-accent px-4 py-2 text-small font-medium text-accent-on disabled:opacity-45"
            >
              {saving ? t("common.loading") : t("common.save")}
            </button>

            {saved ? (
              <p
                data-testid="notifications-saved"
                role="status"
                className="text-small text-success"
              >
                {t("notifications.saved")}
              </p>
            ) : null}

            {errors.length > 0 ? (
              <ul
                data-testid="notifications-errors"
                role="alert"
                className="text-small text-danger"
              >
                {errors.map((message) => (
                  <li key={message}>{message}</li>
                ))}
              </ul>
            ) : null}
          </div>
        ) : null}
      </div>
    </section>
  );
}
