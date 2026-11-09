import { useState, type FormEvent } from "react";
import { ApiError, gatewayClient } from "../../api/client";
import { useI18n } from "../../i18n/react";

const EMAIL = /^[^\s@]+@[^\s@]+\.[^\s@]+$/;

export default function VerifyEmailPage({
  initialEmail = "",
  resend = (email: string) => gatewayClient.resendVerification(email),
}: {
  initialEmail?: string;
  resend?: (email: string) => Promise<void>;
}) {
  const { t } = useI18n();
  const [email, setEmail] = useState(initialEmail);
  const [state, setState] = useState<"idle" | "sending" | "sent">("idle");
  const [error, setError] = useState<string | null>(null);
  const [retryAfter, setRetryAfter] = useState<string | null>(null);

  async function onSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setError(null);
    setRetryAfter(null);
    if (!EMAIL.test(email)) {
      setError(t("auth.invalidEmail"));
      return;
    }
    setState("sending");
    try {
      await resend(email);
      setState("sent");
    } catch (caught) {
      setState("idle");
      if (caught instanceof ApiError && caught.problem.status === 429) {
        setRetryAfter(caught.problem.detail ?? t("common.retry"));
        setError(t("problem.server"));
      } else {
        setError(
          caught instanceof ApiError
            ? (caught.problem.detail ?? caught.problem.title)
            : caught instanceof Error
              ? caught.message
              : t("problem.server"),
        );
      }
    }
  }

  return (
    <section className="mx-auto max-w-xl rounded-lg border border-border bg-surface p-8">
      <h1 className="font-display text-section font-medium text-ink">
        {t("auth.verifyTitle")}
      </h1>
      <p className="mt-2 text-body text-muted">{t("auth.verifyBody")}</p>

      <form data-testid="verify-form" onSubmit={onSubmit} className="mt-6 grid gap-3" noValidate>
        <label className="flex flex-col gap-1 text-small font-medium text-ink-2">
          {t("auth.email")}
          <input
            data-testid="verify-email"
            type="email"
            value={email}
            onChange={(event) => setEmail(event.target.value)}
            className="rounded-xl border border-border bg-surface px-3 py-2 text-small text-ink"
          />
        </label>
        <button
          type="submit"
          data-testid="verify-resend"
          disabled={state === "sending"}
          className="w-fit rounded-lg bg-accent px-4 py-2 text-small font-medium text-accent-on disabled:opacity-45"
        >
          {state === "sending" ? t("common.loading") : t("auth.resend")}
        </button>
      </form>

      {state === "sent" ? (
        <p data-testid="verify-sent" role="status" className="mt-4 text-small text-success">
          {t("auth.verifyBody")}
        </p>
      ) : null}
      {error ? (
        <p data-testid="verify-error" role="alert" className="mt-4 text-small text-danger">
          {error}
        </p>
      ) : null}
      {retryAfter ? (
        <p data-testid="verify-retry-after" className="mt-1 text-small text-meta">
          {retryAfter}
        </p>
      ) : null}
    </section>
  );
}
