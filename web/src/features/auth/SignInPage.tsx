import { useState, type FormEvent } from "react";
import { Navigate, useLocation } from "react-router-dom";
import { gatewayClient, ApiError } from "../../api/client";
import { useSession, sessionStore, type SessionState } from "../../auth/session";
import { useI18n } from "../../i18n/react";

const EMAIL = /^[^\s@]+@[^\s@]+\.[^\s@]+$/;

interface SignInPageProps {
  sessionOverride?: SessionState;
  signIn?: (email: string, password: string) => Promise<void>;
  signUp?: (email: string, password: string, gateToken?: string) => Promise<void>;
  createGate?: (input: {
    email: string;
    jurisdiction: string;
    ageConfirmed: boolean;
  }) => Promise<{ gateToken: string }>;
}

export default function SignInPage({
  sessionOverride,
  signIn = (email, password) => sessionStore.signIn(email, password),
  signUp = (email, password, gateToken) => sessionStore.signUp(email, password, gateToken),
  createGate = (input) => gatewayClient.createSignupGate(input),
}: SignInPageProps) {
  const live = useSession();
  const session = sessionOverride ?? live;
  const location = useLocation();
  const { t } = useI18n();

  const [mode, setMode] = useState<"signin" | "signup">("signin");
  const [email, setEmail] = useState("");
  const [password, setPassword] = useState("");
  const [gateToken, setGateToken] = useState("");
  const [formError, setFormError] = useState<string | null>(null);
  const [fieldErrors, setFieldErrors] = useState<string[]>([]);
  const [submitting, setSubmitting] = useState(false);

  // A 401 redirect carries the return path as a query because it happens
  // outside the router; an in-app redirect carries it in location state.
  const returnTo =
    (location.state as { returnTo?: string } | null)?.returnTo ??
    new URLSearchParams(location.search).get("returnTo") ??
    undefined;

  if (session.status === "signed-in") {
    return <Navigate to={returnTo ?? "/dashboard"} replace />;
  }

  function validate(): boolean {
    const errors: string[] = [];
    if (!EMAIL.test(email)) {
      errors.push(t("auth.invalidEmail"));
    }
    if (password.length < 8) {
      errors.push(t("auth.passwordTooShort"));
    }
    if (mode === "signup" && !gateToken.trim()) {
      errors.push(t("common.required"));
    }
    setFieldErrors(errors);
    return errors.length === 0;
  }

  async function onSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setFormError(null);
    if (!validate() || submitting) {
      return;
    }
    setSubmitting(true);
    try {
      if (mode === "signup") {
        const gate = await createGate({
          email,
          jurisdiction: "supported",
          ageConfirmed: true,
        });
        await signUp(email, password, gate.gateToken);
      } else {
        await signIn(email, password);
      }
    } catch (error) {
      const problem = error instanceof ApiError ? error.problem : null;
      setFormError(
        problem?.detail ?? (error instanceof Error ? error.message : t("problem.server")),
      );
    } finally {
      setSubmitting(false);
    }
  }

  return (
    <div className="mx-auto grid max-w-5xl gap-8 lg:grid-cols-2">
      <section>
        <p className="text-overline uppercase tracking-overline text-meta">{t("app.name")}</p>
        <h1 className="mt-2 font-display text-display font-medium leading-display text-ink">
          {t("app.tagline")}
        </h1>
        <p className="mt-3 max-w-prose text-body text-muted">{t("onboarding.title")}</p>
      </section>

      <section className="rounded-lg border border-border bg-surface p-6 shadow-[rgba(0,0,0,0.05)_0_4px_24px]">
        <h2 className="font-display text-section font-medium text-ink">
          {mode === "signin" ? t("auth.signIn") : t("auth.signUp")}
        </h2>

        <form data-testid="signin-form" onSubmit={onSubmit} className="mt-4 grid gap-3" noValidate>
          {mode === "signup" ? (
            <label className="flex flex-col gap-1 text-small font-medium text-ink-2">
              {t("auth.gateToken")}
              <input
                data-testid="signup-gate-token"
                value={gateToken}
                onChange={(event) => setGateToken(event.target.value)}
                className="rounded-xl border border-border bg-surface px-3 py-2 text-small text-ink"
              />
            </label>
          ) : null}

          <label className="flex flex-col gap-1 text-small font-medium text-ink-2">
            {t("auth.email")}
            <input
              data-testid="signin-email"
              type="email"
              autoComplete="email"
              value={email}
              onChange={(event) => setEmail(event.target.value)}
              className="rounded-xl border border-border bg-surface px-3 py-2 text-small text-ink"
            />
          </label>

          <label className="flex flex-col gap-1 text-small font-medium text-ink-2">
            {t("auth.password")}
            <input
              data-testid="signin-password"
              type="password"
              autoComplete="current-password"
              value={password}
              onChange={(event) => setPassword(event.target.value)}
              className="rounded-xl border border-border bg-surface px-3 py-2 text-small text-ink"
            />
          </label>

          {fieldErrors.length > 0 ? (
            <ul data-testid="signin-field-errors" role="alert" className="text-small text-danger">
              {fieldErrors.map((message) => (
                <li key={message}>{message}</li>
              ))}
            </ul>
          ) : null}

          {formError ? (
            <p data-testid="signin-error" role="alert" className="text-small text-danger">
              {formError}
            </p>
          ) : null}

          <button
            type="submit"
            data-testid="signin-submit"
            disabled={submitting}
            className="rounded-lg bg-accent px-4 py-2 text-small font-medium text-accent-on disabled:opacity-45"
          >
            {submitting
              ? t("common.loading")
              : mode === "signin"
                ? t("auth.signIn")
                : t("auth.signUp")}
          </button>
        </form>

        <button
          type="button"
          data-testid="signup-toggle"
          onClick={() => {
            setMode(mode === "signin" ? "signup" : "signin");
            setFieldErrors([]);
            setFormError(null);
          }}
          className="mt-3 text-small text-ink-2 underline"
        >
          {mode === "signin" ? t("auth.signUp") : t("auth.signIn")}
        </button>
      </section>
    </div>
  );
}
