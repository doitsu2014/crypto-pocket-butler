import { Navigate, useLocation } from "react-router-dom";
import type { ReactNode } from "react";
import { useSession, type SessionState } from "../auth/session";
import { isMfaAllowedPath, requiresMfaEnrolment, type MfaProfile } from "../auth/mfa";
import { LimitedSessionProvider, useLimitedSession } from "../auth/limited";
import { useProfile } from "../api/queries";
import { BlockingState, ErrorPanel, Loading } from "../components";

function MfaRedirect({
  profile,
  children,
}: {
  profile: MfaProfile | null | undefined;
  children: ReactNode;
}) {
  const { declined } = useLimitedSession();
  const location = useLocation();
  if (
    requiresMfaEnrolment(profile) &&
    !declined &&
    !isMfaAllowedPath(location.pathname)
  ) {
    return <Navigate to="/settings/mfa" replace />;
  }
  return <>{children}</>;
}

function MfaGate({ children }: { children: ReactNode }) {
  const profile = useProfile(true);

  if (profile.isLoading) {
    return <Loading />;
  }
  if (profile.error) {
    return <ErrorPanel error={profile.error} onRetry={profile.refetch} />;
  }
  return (
    <LimitedSessionProvider profile={profile.data}>
      <MfaRedirect profile={profile.data}>{children}</MfaRedirect>
    </LimitedSessionProvider>
  );
}

/**
 * The signed-in boundary: a loading state, a sign-in redirect with the deep
 * link, the MFA gate, and a recovery surface when the session is lost.
 */
export function RequireSession({
  children,
  sessionOverride,
}: {
  children: ReactNode;
  sessionOverride?: SessionState;
}) {
  const live = useSession();
  const session = sessionOverride ?? live;
  const location = useLocation();

  if (session.status === "loading") {
    return <Loading />;
  }
  if (session.status === "unavailable") {
    return <BlockingState operation="Supabase Auth (C13)" unit="configuration" />;
  }
  if (session.status === "signed-out") {
    return (
      <Navigate
        to="/"
        replace
        state={{ returnTo: `${location.pathname}${location.search}` }}
      />
    );
  }
  return (
    <MfaGate>
      {children}
    </MfaGate>
  );
}
