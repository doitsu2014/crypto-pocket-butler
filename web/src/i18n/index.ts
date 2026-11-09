import en from "./en.json";
import vi from "./vi.json";

export type Locale = "en" | "vi";

export type Translate = (key: string, params?: Record<string, string | number>) => string;

export interface LocaleInput {
  /** `?lang=` in the URL, before sign-in. */
  urlLang?: string | null;
  /** The server-side preference, after sign-in. */
  preference?: string | null;
  /** `navigator.language`. */
  browser?: string | null;
}

const RESOURCES: Record<Locale, Record<string, string>> = {
  en: en as Record<string, string>,
  vi: vi as Record<string, string>,
};

const LOCALES: Record<Locale, string> = {
  en: "en-US",
  vi: "vi-VN",
};

function normalise(value: string | null | undefined): Locale | null {
  if (!value) {
    return null;
  }
  const lower = value.toLowerCase();
  if (lower === "vi" || lower.startsWith("vi-")) {
    return "vi";
  }
  if (lower === "en" || lower.startsWith("en-")) {
    return "en";
  }
  return null;
}

/**
 * Chooses the locale for a surface (BR6.1): the URL wins before sign-in, then
 * the server preference, then the browser language, defaulting to English.
 */
export function resolveLocale(input: LocaleInput): Locale {
  return (
    normalise(input.urlLang) ?? normalise(input.preference) ?? normalise(input.browser) ?? "en"
  );
}

/** A translator that falls back to English for a missing key. */
export function createTranslator(locale: Locale): Translate {
  return (key, params) => {
    const template = RESOURCES[locale][key] ?? RESOURCES.en[key] ?? key;
    if (!params) {
      return template;
    }
    return template.replace(/\{(\w+)\}/g, (match, name: string) => {
      const value = params[name];
      return value === undefined ? match : String(value);
    });
  };
}

/** The Intl locale identifier for a UI locale. */
export function intlLocale(locale: Locale): string {
  return LOCALES[locale];
}

/**
 * Formats an instant in the user's locale and timezone. A missing or invalid
 * instant is an em dash, never a fabricated date (BR2.1).
 */
export function formatDateTime(
  iso: string | null | undefined,
  locale: Locale,
  timezone?: string | null,
): string {
  if (!iso) {
    return "—";
  }
  const date = new Date(iso);
  if (Number.isNaN(date.getTime())) {
    return "—";
  }
  return new Intl.DateTimeFormat(intlLocale(locale), {
    dateStyle: "medium",
    timeStyle: "short",
    timeZone: timezone ?? undefined,
  }).format(date);
}

export interface ProblemLike {
  type: string;
  title: string;
  status: number;
  detail?: string | null;
  correlationId?: string;
}

/**
 * Maps an RFC 7807 problem to a localised message by problem type or status,
 * keeping the server's title/detail when no mapping exists (BR7.1).
 */
export function localiseProblem(problem: ProblemLike, t: Translate): string {
  const type = problem.type.toLowerCase();
  if (problem.status === 401 || type.includes("unauthorized")) {
    return t("problem.unauthorized");
  }
  if (problem.status === 403 || type.includes("forbidden")) {
    return t("problem.forbidden");
  }
  if (problem.status === 409 || type.includes("conflict")) {
    return t("problem.conflict");
  }
  if (problem.status === 422 || type.includes("validation")) {
    return t("problem.validation");
  }
  if (problem.status === 501 || type.includes("not-implemented")) {
    return t("problem.unimplemented");
  }
  if (problem.status >= 500) {
    return t("problem.server");
  }
  return problem.detail?.trim() ? problem.detail : problem.title;
}
