import { createContext, useContext, useMemo, useState, type ReactNode } from "react";
import { requiresMfaEnrolment, type MfaProfile } from "./mfa";

interface LimitedSessionValue {
  /** The user declined MFA enrolment and the session is limited. */
  declined: boolean;
  limited: boolean;
  decline: () => void;
  reset: () => void;
}

const LimitedSessionContext = createContext<LimitedSessionValue>({
  declined: false,
  limited: false,
  decline: () => undefined,
  reset: () => undefined,
});

export function LimitedSessionProvider({
  profile,
  children,
}: {
  profile: MfaProfile | null | undefined;
  children: ReactNode;
}) {
  const [declined, setDeclined] = useState(false);
  const value = useMemo<LimitedSessionValue>(
    () => ({
      declined,
      limited: declined && requiresMfaEnrolment(profile),
      decline: () => setDeclined(true),
      reset: () => setDeclined(false),
    }),
    [declined, profile],
  );
  return (
    <LimitedSessionContext.Provider value={value}>
      {children}
    </LimitedSessionContext.Provider>
  );
}

export function useLimitedSession(): LimitedSessionValue {
  return useContext(LimitedSessionContext);
}
