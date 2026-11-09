import { useEffect, useState } from "react";
import { clearSupportSession, setSupportSession, type SupportSession } from "../auth/session";
import { SupportSignInGate } from "../components/SupportSignInGate";
import type { SupportAuthClient } from "../auth/supabase";
import { cardClass, sectionTitleClass } from "../components/ui";
import { SupportShell } from "./SupportShell";

export interface SupportAppProps {
  initialSession?: SupportSession | null;
  authClient?: SupportAuthClient | null;
}

export default function SupportApp({ initialSession = null, authClient = null }: SupportAppProps) {
  const [session, setSession] = useState<SupportSession | null>(initialSession);

  useEffect(() => {
    if (initialSession) {
      setSupportSession(initialSession);
    }
  }, [initialSession]);

  async function signIn(email: string, password: string): Promise<void> {
    if (!authClient) {
      throw new Error("Sign-in is not configured.");
    }
    const next = await authClient.signIn(email, password);
    setSupportSession(next);
    setSession(next);
  }

  async function signOut(): Promise<void> {
    clearSupportSession();
    setSession(null);
    if (authClient) {
      try {
        await authClient.signOut();
      } catch (error) {
        console.error("Support sign-out could not reach the auth service.", error);
      }
    }
  }

  if (!session && !authClient) {
    return (
      <main className="min-h-screen bg-parchment px-6 py-12 text-ink">
        <div className="mx-auto w-full max-w-md">
          <div role="alert" className={cardClass}>
            <h1 className={sectionTitleClass}>Sign-in is not configured</h1>
            <p className="mt-2 text-small text-ink-2">
              Set the Supabase public URL and anon key at build time, then reload the console.
            </p>
          </div>
        </div>
      </main>
    );
  }

  return (
    <SupportSignInGate session={session} onSignIn={signIn} onSignOut={signOut}>
      {(activeSession, role) => (
        <SupportShell session={activeSession} role={role} onSignOut={signOut} />
      )}
    </SupportSignInGate>
  );
}
