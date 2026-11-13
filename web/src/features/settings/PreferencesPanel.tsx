import { useMemo, useState } from "react";
import { Loading, PageHeader } from "../../components";
import { useI18n } from "../../i18n/react";
import { usePreferences, useSavePreferences } from "../../api/settingsQueries";
import type { EffectivePreferences, PreferencesPatch } from "../../api/types";
import { correlationOf, fieldErrorsOf, problemMessage } from "./problemText";

const LOCALES = ["en", "vi"];
const CURRENCIES = ["USD", "EUR", "GBP", "VND"];
const FALLBACK_TIMEZONES = ["UTC", "Europe/Paris", "Asia/Ho_Chi_Minh", "America/New_York"];

function allTimezones(): string[] {
  const intl = Intl as unknown as { supportedValuesOf?: (key: string) => string[] };
  try {
    return intl.supportedValuesOf?.("timeZone") ?? FALLBACK_TIMEZONES;
  } catch {
    return FALLBACK_TIMEZONES;
  }
}

interface Draft {
  locale: string;
  baseCurrency: string;
  timezone: string;
  email: boolean;
  push: boolean;
}

function draftOf(preferences: EffectivePreferences): Draft {
  return {
    locale: preferences.locale.value,
    baseCurrency: preferences.baseCurrency.value,
    timezone: preferences.timezone.value,
    email: preferences.notifications.email.value,
    push: preferences.notifications.push.value,
  };
}

function changedPatch(preferences: EffectivePreferences, draft: Draft): PreferencesPatch {
  const patch: PreferencesPatch = {};
  if (draft.locale !== preferences.locale.value) {
    patch.locale = draft.locale;
  }
  if (draft.baseCurrency !== preferences.baseCurrency.value) {
    patch.baseCurrency = draft.baseCurrency;
  }
  if (draft.timezone !== preferences.timezone.value) {
    patch.timezone = draft.timezone;
  }
  const notifications: { email?: boolean; push?: boolean } = {};
  if (draft.email !== preferences.notifications.email.value) {
    notifications.email = draft.email;
  }
  if (draft.push !== preferences.notifications.push.value) {
    notifications.push = draft.push;
  }
  if (Object.keys(notifications).length > 0) {
    patch.notifications = notifications;
  }
  return patch;
}

function sourceLabel(source: string, t: (key: string) => string): string {
  return source === "user"
    ? t("settings.preferences.source.user")
    : t("settings.preferences.source.default");
}

/**
 * Preferences panel (functional-spec §3.2, X2). One Save writes only the
 * fields the person changed; every value carries the server's `source` so a
 * choice is distinguishable from a default.
 */
export default function PreferencesPanel() {
  const { t } = useI18n();
  const preferencesQuery = usePreferences();
  const savePreferences = useSavePreferences();
  const timezones = useMemo(allTimezones, []);

  const [draft, setDraft] = useState<Draft | null>(null);
  const [saveError, setSaveError] = useState<unknown>(null);
  const [saved, setSaved] = useState(false);
  const [timezoneOpen, setTimezoneOpen] = useState(false);
  const [activeIndex, setActiveIndex] = useState<number | null>(null);

  if (preferencesQuery.isLoading) {
    return <Loading />;
  }

  if (preferencesQuery.isError || !preferencesQuery.data) {
    return (
      <section role="alert" data-testid="preferences-error" className="text-small text-danger">
        <p>{problemMessage(preferencesQuery.error, t)}</p>
        <button
          type="button"
          data-testid="preferences-retry"
          onClick={() => void preferencesQuery.refetch()}
          className="mt-2 rounded-lg bg-surface-warm px-3 py-1 font-medium text-ink"
        >
          {t("common.retry")}
        </button>
      </section>
    );
  }

  const preferences = preferencesQuery.data;
  const current = draft ?? draftOf(preferences);
  const fieldErrors = fieldErrorsOf(saveError);
  const filteredTimezones = timezones
    .filter((zone) => zone.toLowerCase().includes(current.timezone.toLowerCase()))
    .slice(0, 50);

  function update(next: Partial<Draft>) {
    setDraft({ ...current, ...next });
    setSaved(false);
  }

  async function onSave() {
    setSaveError(null);
    setSaved(false);
    try {
      await savePreferences.mutateAsync(changedPatch(preferences, current));
      setDraft(null);
      setSaved(true);
    } catch (error) {
      setSaveError(error);
    }
  }

  return (
    <section className="grid gap-8">
      <PageHeader title={t("settings.preferences.title")} />

      <div className="grid gap-5">
        <div className="grid gap-1">
          <label htmlFor="preferences-locale" className="text-small font-medium text-ink">
            {t("settings.preferences.locale")}
          </label>
          <select
            id="preferences-locale"
            data-testid="preferences-locale"
            aria-describedby={fieldErrors.locale ? "preferences-error-locale" : undefined}
            value={current.locale}
            onChange={(event) => update({ locale: event.target.value })}
            className="w-full max-w-xs rounded-lg border border-border bg-surface px-3 py-2 text-small text-ink"
          >
            {LOCALES.map((locale) => (
              <option key={locale} value={locale}>
                {t(`settings.preferences.locale.${locale}`)}
              </option>
            ))}
          </select>
          <span data-testid="preferences-locale-source" className="text-meta text-muted">
            {sourceLabel(preferences.locale.source, t)}
          </span>
          {fieldErrors.locale ? (
            <p
              id="preferences-error-locale"
              data-testid="preferences-error-locale"
              className="text-small text-danger"
            >
              {fieldErrors.locale}
            </p>
          ) : null}
        </div>

        <div className="grid gap-1">
          <label htmlFor="preferences-base-currency" className="text-small font-medium text-ink">
            {t("settings.preferences.baseCurrency")}
          </label>
          <select
            id="preferences-base-currency"
            data-testid="preferences-base-currency"
            aria-describedby={
              fieldErrors.baseCurrency ? "preferences-error-baseCurrency" : undefined
            }
            value={current.baseCurrency}
            onChange={(event) => update({ baseCurrency: event.target.value })}
            className="w-full max-w-xs rounded-lg border border-border bg-surface px-3 py-2 text-small text-ink"
          >
            {CURRENCIES.map((currency) => (
              <option key={currency} value={currency}>
                {currency}
              </option>
            ))}
          </select>
          <span data-testid="preferences-base-currency-source" className="text-meta text-muted">
            {sourceLabel(preferences.baseCurrency.source, t)}
          </span>
          {fieldErrors.baseCurrency ? (
            <p
              id="preferences-error-baseCurrency"
              data-testid="preferences-error-baseCurrency"
              className="text-small text-danger"
            >
              {fieldErrors.baseCurrency}
            </p>
          ) : null}
        </div>

        <div className="relative grid gap-1">
          <label htmlFor="preferences-timezone" className="text-small font-medium text-ink">
            {t("settings.preferences.timezone")}
          </label>
          <input
            id="preferences-timezone"
            data-testid="preferences-timezone"
            role="combobox"
            aria-expanded={timezoneOpen}
            aria-controls="preferences-timezone-list"
            aria-autocomplete="list"
            aria-activedescendant={
              timezoneOpen && activeIndex !== null && filteredTimezones[activeIndex]
                ? `preferences-timezone-option-${activeIndex}`
                : undefined
            }
            aria-describedby={
              fieldErrors.timezone ? "preferences-error-timezone" : undefined
            }
            value={current.timezone}
            onChange={(event) => {
              update({ timezone: event.target.value });
              setActiveIndex(null);
              setTimezoneOpen(true);
            }}
            onFocus={() => setTimezoneOpen(true)}
            onBlur={() => setTimezoneOpen(false)}
            onKeyDown={(event) => {
              if (event.key === "ArrowDown") {
                event.preventDefault();
                setTimezoneOpen(true);
                setActiveIndex((index) =>
                  index === null
                    ? 0
                    : Math.min(index + 1, Math.max(filteredTimezones.length - 1, 0)),
                );
              } else if (event.key === "ArrowUp") {
                event.preventDefault();
                setActiveIndex((index) =>
                  index === null ? 0 : Math.max(index - 1, 0),
                );
              } else if (event.key === "Enter") {
                const zone =
                  timezoneOpen && activeIndex !== null
                    ? filteredTimezones[activeIndex]
                    : undefined;
                if (zone) {
                  event.preventDefault();
                  update({ timezone: zone });
                  setTimezoneOpen(false);
                }
              } else if (event.key === "Escape") {
                setTimezoneOpen(false);
              }
            }}
            className="w-full max-w-xs rounded-lg border border-border bg-surface px-3 py-2 text-small text-ink"
          />
          <span data-testid="preferences-timezone-source" className="text-meta text-muted">
            {sourceLabel(preferences.timezone.source, t)}
          </span>
          {timezoneOpen && filteredTimezones.length > 0 ? (
            <ul
              id="preferences-timezone-list"
              role="listbox"
              data-testid="preferences-timezone-list"
              className="absolute top-full z-10 mt-1 max-h-56 w-full max-w-xs overflow-auto rounded-lg border border-border bg-surface py-1"
            >
              {filteredTimezones.map((zone, index) => (
                <li
                  key={zone}
                  id={`preferences-timezone-option-${index}`}
                  role="option"
                  aria-selected={index === activeIndex}
                  onMouseDown={(event) => event.preventDefault()}
                  onClick={() => {
                    update({ timezone: zone });
                    setTimezoneOpen(false);
                  }}
                  className="cursor-pointer px-3 py-1 text-small text-ink-2 hover:bg-surface-warm"
                >
                  {zone}
                </li>
              ))}
            </ul>
          ) : null}
          {fieldErrors.timezone ? (
            <p
              id="preferences-error-timezone"
              data-testid="preferences-error-timezone"
              className="text-small text-danger"
            >
              {fieldErrors.timezone}
            </p>
          ) : null}
        </div>

        <div className="grid gap-3">
          <span className="text-small font-medium text-ink">
            {t("settings.preferences.notifications")}
          </span>
          {(
            [
              ["email", current.email, "preferences-email"],
              ["push", current.push, "preferences-push"],
            ] as const
          ).map(([channel, checked, testId]) => (
            <div key={channel} className="flex items-center gap-3">
              <button
                type="button"
                role="switch"
                aria-checked={checked}
                aria-label={t(`settings.preferences.channel.${channel}`)}
                aria-describedby={
                  fieldErrors[`notifications.${channel}`]
                    ? `preferences-error-notifications.${channel}`
                    : undefined
                }
                data-testid={testId}
                onClick={() =>
                  update(channel === "email" ? { email: !checked } : { push: !checked })
                }
                className={`relative h-5 w-9 rounded-full border border-border ${
                  checked ? "bg-accent" : "bg-surface-warm"
                }`}
              >
                <span
                  aria-hidden="true"
                  className={`absolute top-0.5 h-3.5 w-3.5 rounded-full bg-surface ${
                    checked ? "right-0.5" : "left-0.5"
                  }`}
                />
              </button>
              <span className="text-small text-ink">
                {t(`settings.preferences.channel.${channel}`)}
              </span>
              <span data-testid={`preferences-${channel}-state`} className="text-meta text-ink-2">
                {checked ? t("common.on") : t("common.off")}
              </span>
              {fieldErrors[`notifications.${channel}`] ? (
                <span
                  id={`preferences-error-notifications.${channel}`}
                  data-testid={`preferences-error-notifications.${channel}`}
                  className="text-small text-danger"
                >
                  {fieldErrors[`notifications.${channel}`]}
                </span>
              ) : null}
            </div>
          ))}
        </div>
      </div>

      <div className="flex flex-wrap items-center gap-3">
        <button
          type="button"
          data-testid="preferences-save"
          disabled={savePreferences.isPending}
          onClick={() => void onSave()}
          className="w-fit rounded-lg bg-accent px-4 py-2 text-small font-medium text-accent-on disabled:opacity-45"
        >
          {t("common.save")}
        </button>
        {saved ? (
          <span data-testid="preferences-saved" role="status" className="text-small text-success">
            {t("settings.saved")}
          </span>
        ) : null}
      </div>

      {saveError && Object.keys(fieldErrors).length === 0 ? (
        <p role="alert" data-testid="preferences-save-error" className="text-small text-danger">
          {problemMessage(saveError, t)}
          {correlationOf(saveError) ? (
            <span className="ml-2 text-meta">
              {t("common.correlationId")}: {correlationOf(saveError)}
            </span>
          ) : null}
        </p>
      ) : null}
    </section>
  );
}
