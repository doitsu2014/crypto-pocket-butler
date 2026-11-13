import { NavLink } from "react-router-dom";
import { useI18n } from "../../i18n/react";

const ITEMS = [
  { to: "/settings/profile", key: "settings.nav.profile", testId: "settings-nav-profile" },
  {
    to: "/settings/preferences",
    key: "settings.nav.preferences",
    testId: "settings-nav-preferences",
  },
  { to: "/settings/consent", key: "settings.nav.consent", testId: "settings-nav-consent" },
  { to: "/settings/privacy", key: "settings.nav.privacy", testId: "settings-nav-privacy" },
];

/**
 * The four settings destinations as plain navigation (not a tablist). The
 * active link carries `aria-current="page"`, which NavLink sets itself.
 */
export function SettingsNav() {
  const { t } = useI18n();
  return (
    <nav aria-label={t("settings.nav.label")} data-testid="settings-nav">
      <ul className="flex flex-wrap gap-2 md:flex-col">
        {ITEMS.map((item) => (
          <li key={item.to}>
            <NavLink
              to={item.to}
              data-testid={item.testId}
              className={({ isActive }) =>
                `block rounded-lg px-3 py-2 text-small font-medium ${
                  isActive
                    ? "bg-surface-warm text-ink"
                    : "text-ink-2 hover:bg-surface-warm hover:text-ink"
                }`
              }
            >
              {t(item.key)}
            </NavLink>
          </li>
        ))}
      </ul>
    </nav>
  );
}
