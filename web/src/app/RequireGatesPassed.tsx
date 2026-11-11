/**
 * The post-sign-in gate guard (functional-spec §2.1; frontend-components §2).
 *
 * Every product route U8 owns routing for sits inside this guard: children
 * render only once the server-resolved gate state is `passed`. Any unresolved
 * status renders the gate surface instead — the gate screen for outstanding
 * acceptances (AC6.2.2, AC6.5.2), the blocked notice for a restricted
 * jurisdiction (AC6.3.2), the owned-elsewhere destination, the full-page retry
 * on a failed read, or the skeleton while resolving. No branch enters the app
 * on an unresolved status. The guard is a UX affordance only; the server-side
 * consent gate (U4) and the gateway route table remain the enforcement.
 *
 * Requirements: FR19.2, FR19.3, FR19.4, FR19.5, NFR2.6, NFR24.1, NFR25.1.
 */

import type { ReactNode } from "react";
import { lazy, Suspense } from "react";
import { Navigate } from "react-router-dom";
import { useQueryClient } from "@tanstack/react-query";
import { signOut, useSession } from "../auth/session";
import { useI18n } from "../i18n/react";
import { BlockedJurisdictionNotice } from "../features/signup-gates/BlockedJurisdictionNotice";
import { GATE_DESTINATIONS, type GateStatus } from "../features/signup-gates/gateStatus";
import { GateFullPageRetry } from "../features/signup-gates/GateFullPageRetry";
import { gateSessionKey, useGateStatus } from "../features/signup-gates/useGateStatus";

/**
 * The gate surface is loaded on demand so the form, the combobox, and the
 * generated country module stay in an async chunk and never in the entry chunk
 * (performance-design §3, NFR2.1). The guard itself stays tiny.
 */
const LazySignupGatesScreen = lazy(async () => {
  const module = await import("../features/signup-gates/SignupGatesScreen");
  return { default: module.SignupGatesScreen };
});

function GateLoading() {
  const { t } = useI18n();
  return (
    <p
      data-testid="gates-loading"
      role="status"
      aria-live="polite"
      className="animate-pulse text-small text-muted"
    >
      {t("gates.loading")}
    </p>
  );
}

/**
 * A terminal surface for destinations the router does not register yet (U7
 * account recovery and account status). Navigating there would fall through
 * the catch-all back to sign-in and loop, so the guard stops here and offers
 * the one safe action.
 */
function DestinationUnavailable({ onSignOut }: { onSignOut: () => void }) {
  const { t } = useI18n();
  return (
    <section
      data-testid="gates-destination-unavailable"
      role="status"
      aria-live="polite"
      className="mx-auto max-w-xl rounded-lg border border-border bg-surface p-6"
    >
      <h1 className="text-section text-ink">{t("gates.unavailable.title")}</h1>
      <p className="mt-3 text-small text-ink-2">{t("gates.unavailable.body")}</p>
      <button
        type="button"
        data-testid="gates-signout"
        onClick={onSignOut}
        className="mt-6 rounded-xl bg-accent px-4 py-2 text-small font-medium text-accent-on"
      >
        {t("gates.blocked.signOut")}
      </button>
    </section>
  );
}

export interface RequireGatesPassedProps {
  children: ReactNode;
  /** Test/route override of the resolved status; production omits it. */
  status?: GateStatus;
}

export function RequireGatesPassed({ children, status }: RequireGatesPassedProps) {
  const session = useSession();
  const resolved = useGateStatus();
  const queryClient = useQueryClient();
  const current = status ?? resolved;

  switch (current.kind) {
    case "passed":
      return <>{children}</>;
    case "gates-required":
      return (
        <Suspense fallback={<GateLoading />}>
          <LazySignupGatesScreen />
        </Suspense>
      );
    case "jurisdiction-blocked":
      return (
        <BlockedJurisdictionNotice
          continuedUse={current.continuedUse}
          onSignOut={() => void signOut()}
        />
      );
    case "owned-elsewhere":
      if (current.destination === "restore-account" || current.destination === "account-blocked") {
        return <DestinationUnavailable onSignOut={() => void signOut()} />;
      }
      return <Navigate to={GATE_DESTINATIONS[current.destination]} replace />;
    case "error":
      return (
        <GateFullPageRetry
          onRetry={() => {
            void queryClient.invalidateQueries({
              queryKey: gateSessionKey(session.session?.user?.id ?? null),
            });
          }}
        />
      );
    case "loading":
      return <GateLoading />;
  }
}
