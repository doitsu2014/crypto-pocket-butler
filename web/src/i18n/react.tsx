import { createContext, useContext, useMemo, type ReactNode } from "react";
import { createTranslator, formatDateTime, type Locale, type Translate } from "./index";

export interface I18nValue {
  locale: Locale;
  timezone: string | null;
  t: Translate;
  format: (iso: string | null | undefined) => string;
}

const FALLBACK: I18nValue = {
  locale: "en",
  timezone: null,
  t: createTranslator("en"),
  format: (iso) => formatDateTime(iso, "en", null),
};

const I18nContext = createContext<I18nValue>(FALLBACK);

export function I18nProvider({
  locale = "en",
  timezone = null,
  children,
}: {
  locale?: Locale;
  timezone?: string | null;
  children: ReactNode;
}) {
  const value = useMemo<I18nValue>(
    () => ({
      locale,
      timezone,
      t: createTranslator(locale),
      format: (iso) => formatDateTime(iso, locale, timezone),
    }),
    [locale, timezone],
  );
  return <I18nContext.Provider value={value}>{children}</I18nContext.Provider>;
}

export function useI18n(): I18nValue {
  return useContext(I18nContext);
}
