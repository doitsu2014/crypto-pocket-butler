import { useEffect, useState, type FormEvent, type ReactNode } from "react";
import { resolveSupportRole, type SupportRole, type SupportSession } from "../auth/session";
import {
  alertClass,
  cardClass,
  inputClass,
  labelClass,
  overlineClass,
  primaryButtonClass,
  sectionTitleClass,
} from "./ui";

export interface SupportSignInGateProps {
  session: SupportSession | null;
  onSignIn: (email: string, password: string) => Promise<void>;
  onSignOut: () => void;
  onReady?: (role: SupportRole) => void;
  children: (session: SupportSession, role: SupportRole) => ReactNode;
}

/**
 * BR1.1: only a support or admin role in app_metadata reaches the shell. This
 * gate decides what the UI shows; the gateway is the actual control on every
 * C2 call.
 */
export function SupportSignInGate({
  session,
  onSignIn,
  onSignOut,
  onReady,
  children,
}: SupportSignInGateProps) {
  const [email, setEmail] = useState("");
  const [password, setPassword] = useState("");
  const [isSubmitting, setIsSubmitting] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const role = resolveSupportRole(session?.user ?? null);

  useEffect(() => {
    if (role) {
      onReady?.(role);
    }
  }, [role, onReady]);

  async function submit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (isSubmitting) {
      return;
    }
    setError(null);
    setIsSubmitting(true);
    try {
      await onSignIn(email.trim(), password);
    } catch (caught) {
      setError(caught instanceof Error ? caught.message : "Sign-in failed.");
    } finally {
      setIsSubmitting(false);
    }
  }

  if (!session) {
    return (
      <main className="min-h-screen bg-parchment px-6 py-12 text-ink">
        <div className="mx-auto flex w-full max-w-md flex-col gap-6">
          <header>
            <p className={overlineClass}>Crypto Pocket Butler</p>
            <h1 className={`${sectionTitleClass} mt-1`}>Support console</h1>
            <p className="mt-2 text-body leading-body text-muted">
              Sign in with a support or admin account. The session stays in memory for this tab.
            </p>
          </header>

          <form data-testid="sign-in-form" onSubmit={submit} className={cardClass} noValidate>
            <div className="flex flex-col gap-4">
              <label className={labelClass} htmlFor="sign-in-email">
                Email
                <input
                  id="sign-in-email"
                  data-testid="sign-in-email"
                  type="email"
                  autoComplete="username"
                  required
                  value={email}
                  onChange={(event) => setEmail(event.target.value)}
                  className={inputClass}
                />
              </label>
              <label className={labelClass} htmlFor="sign-in-password">
                Password
                <input
                  id="sign-in-password"
                  data-testid="sign-in-password"
                  type="password"
                  autoComplete="current-password"
                  required
                  value={password}
                  onChange={(event) => setPassword(event.target.value)}
                  className={inputClass}
                />
              </label>
            </div>

            {error ? (
              <p role="alert" className={`${alertClass} mt-4`}>
                {error}
              </p>
            ) : null}

            <button
              type="submit"
              data-testid="sign-in-submit"
              disabled={isSubmitting}
              className={`${primaryButtonClass} mt-6`}
            >
              {isSubmitting ? "Signing in…" : "Sign in"}
            </button>
          </form>
        </div>
      </main>
    );
  }

  if (!role) {
    return (
      <main className="min-h-screen bg-parchment px-6 py-12 text-ink">
        <div className="mx-auto w-full max-w-md">
          <div className={`${cardClass} flex flex-col gap-4`}>
            <p className={overlineClass}>Access denied</p>
            <h1 className={sectionTitleClass}>Not authorised</h1>
            <p className="text-small text-ink-2">
              The support console is limited to support and admin accounts. No account data has
              been loaded.
            </p>
            <button type="button" data-testid="sign-out" onClick={onSignOut} className={primaryButtonClass}>
              Sign out
            </button>
          </div>
        </div>
      </main>
    );
  }

  return <>{children(session, role)}</>;
}
