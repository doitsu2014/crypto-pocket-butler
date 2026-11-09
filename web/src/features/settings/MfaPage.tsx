import { useState } from "react";
import { useNavigate } from "react-router-dom";
import { gatewayClient } from "../../api/client";
import { useProfile } from "../../api/queries";
import type { Profile, RecoveryCodes } from "../../api/types";
import { useLimitedSession } from "../../auth/limited";
import { ErrorPanel, Loading, PageHeader } from "../../components";
import { useI18n } from "../../i18n/react";

type Enrolment = { secret: string; otpauthUrl: string };

export default function MfaPage({
  profileOverride,
  enrol,
  recovery,
  decline,
}: {
  profileOverride?: Profile;
  enrol?: () => Promise<Enrolment>;
  recovery?: () => Promise<RecoveryCodes>;
  decline?: () => void;
} = {}) {
  const { t } = useI18n();
  const navigate = useNavigate();
  const limited = useLimitedSession();
  const profileQuery = useProfile(!profileOverride);
  const profile = profileOverride ?? profileQuery.data;

  const [enrolment, setEnrolment] = useState<Enrolment | null>(null);
  const [codes, setCodes] = useState<string[] | null>(null);
  const [error, setError] = useState<unknown>(null);
  const [declined, setDeclined] = useState(false);
  const [busy, setBusy] = useState(false);

  async function onEnrol() {
    setError(null);
    setBusy(true);
    try {
      const next = enrol ? await enrol() : await gatewayClient.enrolMfa();
      setEnrolment(next);
    } catch (caught) {
      setError(caught);
    } finally {
      setBusy(false);
    }
  }

  async function onRecovery() {
    setError(null);
    setBusy(true);
    try {
      const next = recovery ? await recovery() : await gatewayClient.recoveryCodes();
      setCodes(next.codes);
    } catch (caught) {
      setError(caught);
    } finally {
      setBusy(false);
    }
  }

  function onDecline() {
    if (decline) {
      decline();
    } else {
      limited.decline();
    }
    setDeclined(true);
    navigate("/dashboard");
  }

  if (profileQuery.isLoading && !profileOverride) {
    return <Loading />;
  }

  return (
    <section className="mx-auto grid max-w-2xl gap-6">
      <PageHeader
        title={t("settings.mfa")}
        description={profile?.mfaEnrolled ? t("settings.mfaEnrolled") : t("settings.mfaRequired")}
      />

      {error ? <ErrorPanel error={error} /> : null}

      {declined ? (
        <p
          data-testid="mfa-declined"
          role="status"
          className="rounded-lg border border-warn bg-surface-warm p-3 text-small text-ink-2"
        >
          {t("settings.mfaLimited")}
        </p>
      ) : null}

      <div className="grid gap-3 rounded-lg border border-border bg-surface p-4">
        <button
          type="button"
          data-testid="mfa-enrol"
          disabled={busy}
          onClick={() => void onEnrol()}
          className="w-fit rounded-lg bg-accent px-4 py-2 text-small font-medium text-accent-on disabled:opacity-45"
        >
          {t("settings.enrolMfa")}
        </button>

        {enrolment ? (
          <div className="grid gap-2 text-small text-ink-2">
            <p>
              {t("settings.secret")}:{" "}
              <span data-testid="mfa-secret" className="font-mono text-ink">
                {enrolment.secret}
              </span>
            </p>
            <p className="break-all font-mono text-meta">{enrolment.otpauthUrl}</p>
            <button
              type="button"
              data-testid="mfa-recovery"
              disabled={busy}
              onClick={() => void onRecovery()}
              className="w-fit rounded-lg bg-surface-warm px-3 py-1 text-small font-medium text-ink disabled:opacity-45"
            >
              {t("settings.recoveryCodes")}
            </button>
          </div>
        ) : null}

        {codes ? (
          <div className="grid gap-2">
            <p className="text-small font-medium text-ink">{t("settings.recoveryCodes")}</p>
            <ul
              data-testid="mfa-recovery-codes"
              className="grid grid-cols-2 gap-1 font-mono text-small text-ink"
            >
              {codes.map((code) => (
                <li key={code}>{code}</li>
              ))}
            </ul>
            <p data-testid="mfa-recovery-once" className="text-small text-warn">
              {t("settings.recoveryOnce")}
            </p>
          </div>
        ) : null}
      </div>

      <button
        type="button"
        data-testid="mfa-decline"
        onClick={onDecline}
        className="w-fit text-small text-ink-2 underline"
      >
        {t("settings.declineMfa")}
      </button>
    </section>
  );
}
