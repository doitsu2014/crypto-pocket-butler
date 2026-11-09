import { createContext, useContext, useMemo, useState, type ReactNode } from "react";
import { resolveLocale, type Locale } from "./index";
import { I18nProvider } from "./react";

interface LocaleValue {
  locale: Locale;
  timezone: string | null;
  setLocale: (locale: Locale) => void;
  setTimezone: (timezone: string | null) => void;
}

const LocaleContext = createContext<LocaleValue>({
  locale: "en",
  timezone: null,
  setLocale: () => undefined,
  setTimezone: () => undefined,
});

export function LocaleProvider({
  initialLocale,
  initialTimezone = null,
  children,
}: {
  initialLocale?: Locale;
  initialTimezone?: string | null;
  children: ReactNode;
}) {
  const detected = useMemo(() => {
    const params =
      typeof window === "undefined" ? null : new URLSearchParams(window.location.search);
    return (
      initialLocale ??
      resolveLocale({
        urlLang: params?.get("lang") ?? null,
        browser: typeof navigator === "undefined" ? null : navigator.language,
      })
    );
  }, [initialLocale]);
  const [locale, setLocale] = useState<Locale>(detected);
  const [timezone, setTimezone] = useState<string | null>(initialTimezone);

  const value = useMemo<LocaleValue>(
    () => ({ locale, timezone, setLocale, setTimezone }),
    [locale, timezone],
  );

  return (
    <LocaleContext.Provider value={value}>
      <I18nProvider locale={locale} timezone={timezone}>
        {children}
      </I18nProvider>
    </LocaleContext.Provider>
  );
}

export function useLocale(): LocaleValue {
  return useContext(LocaleContext);
}
