import { Link, NavLink, Outlet, useLocation, useNavigate } from "react-router-dom";
import { signOut } from "../auth/session";
import { useLimitedSession } from "../auth/limited";
import { useSession, type SessionState } from "../auth/session";
import { useLocale } from "../i18n/LocaleProvider";
import { useI18n } from "../i18n/react";
import type { Locale } from "../i18n";

const NAV = [
  ["/dashboard", "nav.dashboard"],
  ["/portfolios", "nav.portfolios"],
  ["/connect", "nav.connect"],
  ["/activity", "nav.activity"],
  ["/briefs", "nav.briefs"],
  ["/notifications", "nav.notifications"],
  ["/settings", "nav.settings"],
  ["/privacy", "nav.privacy"],
  ["/help", "nav.help"],
] as const;

function isAuthRoute(pathname: string): boolean {
  return pathname === "/" || pathname === "/verify-email";
}

export function AppShell({ sessionOverride }: { sessionOverride?: SessionState }) {
  const live = useSession();
  const session = sessionOverride ?? live;
  const location = useLocation();
  const navigate = useNavigate();
  const { locale, setLocale } = useLocale();
  const { t } = useI18n();
  const { limited } = useLimitedSession();
  const authRoute = isAuthRoute(location.pathname);

  async function onSignOut() {
    await signOut();
    navigate("/", { replace: true });
  }

  return (
    <div className="min-h-screen bg-parchment text-ink">
      <a
        href="#main"
        className="sr-only focus:not-sr-only focus:absolute focus:left-4 focus:top-4 focus:rounded-lg focus:bg-accent focus:px-3 focus:py-2 focus:text-small focus:text-accent-on"
      >
        {t("nav.dashboard")}
      </a>

      <header className="border-b border-border bg-surface">
        <div className="mx-auto flex max-w-6xl flex-wrap items-center gap-4 px-4 py-3">
          <Link
            to={session.status === "signed-in" ? "/dashboard" : "/"}
            className="font-display text-lg font-medium text-ink"
            data-testid="app-brand"
          >
            {t("app.name")}
          </Link>

          {!authRoute && session.status === "signed-in" ? (
            <nav aria-label="Main" className="flex flex-wrap items-center gap-1">
              {NAV.map(([href, key]) => (
                <NavLink
                  key={href}
                  to={href}
                  className={({ isActive }) =>
                    `rounded-lg px-2 py-1 text-small ${
                      isActive ? "text-accent underline" : "text-ink-2 hover:text-ink"
                    }`
                  }
                >
                  {t(key)}
                </NavLink>
              ))}
            </nav>
          ) : (
            <span className="text-small text-muted">{t("app.tagline")}</span>
          )}

          <div className="ml-auto flex items-center gap-3">
            <label className="text-small text-muted">
              <span className="sr-only">{t("onboarding.locale")}</span>
              <select
                data-testid="locale-switch"
                value={locale}
                onChange={(event) => setLocale(event.target.value as Locale)}
                className="rounded-lg border border-border bg-surface px-2 py-1 text-small text-ink"
              >
                <option value="en">English</option>
                <option value="vi">Tiếng Việt</option>
              </select>
            </label>
            {session.status === "signed-in" ? (
              <button
                type="button"
                data-testid="sign-out"
                onClick={() => {
                  void onSignOut();
                }}
                className="rounded-lg bg-surface-warm px-3 py-1 text-small font-medium text-ink"
              >
                {t("nav.signOut")}
              </button>
            ) : null}
          </div>
        </div>
      </header>

      {limited ? (
        <p
          data-testid="mfa-limited-banner"
          role="status"
          className="border-b border-warn bg-surface-warm px-4 py-2 text-center text-small text-ink-2"
        >
          {t("settings.mfaLimited")}
        </p>
      ) : null}

      <main id="main" className="mx-auto w-full max-w-6xl px-4 py-8">
        <Outlet />
      </main>
    </div>
  );
}
