import { useState, type FormEvent } from "react";
import { gatewayClient } from "../../api/client";
import { problemMessages } from "../../api/errors";
import { useArticles, useIncidents } from "../../api/queries";
import type { Incident, SupportArticle, SupportRequestInput } from "../../api/types";
import { ErrorPanel, Loading, PageHeader } from "../../components";
import { useI18n } from "../../i18n/react";
import { detectSecret } from "../../support/redact";

const INCIDENT_TONE = {
  sev1: "text-danger",
  sev2: "text-warn",
  sev3: "text-ink-2",
} as const;

export default function HelpPage({
  articlesOverride,
  incidentsOverride,
  createRequest,
}: {
  articlesOverride?: SupportArticle[];
  incidentsOverride?: Incident[];
  createRequest?: (input: SupportRequestInput) => Promise<{ id: string }>;
} = {}) {
  const { t, format } = useI18n();
  const articlesQuery = useArticles(!articlesOverride);
  const incidentsQuery = useIncidents(!incidentsOverride);
  const articles = articlesOverride ?? articlesQuery.data ?? [];
  const incidents = incidentsOverride ?? incidentsQuery.data ?? [];

  const [subject, setSubject] = useState("");
  const [body, setBody] = useState("");
  const [errors, setErrors] = useState<string[]>([]);
  const [sent, setSent] = useState(false);
  const [sending, setSending] = useState(false);

  async function onSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setErrors([]);
    setSent(false);
    if (sending) {
      return;
    }
    if (!subject.trim() || !body.trim()) {
      setErrors([t("common.required")]);
      return;
    }
    const secret = detectSecret(`${subject}\n${body}`);
    if (secret.found) {
      setErrors([t("help.secretBlocked")]);
      return;
    }
    setSending(true);
    try {
      if (createRequest) {
        await createRequest({ subject, body });
      } else {
        await gatewayClient.createSupportRequest({ subject, body });
      }
      setSent(true);
      setSubject("");
      setBody("");
    } catch (error) {
      setErrors(problemMessages(error));
    } finally {
      setSending(false);
    }
  }

  return (
    <section className="grid gap-8">
      <PageHeader title={t("help.title")} />

      <div>
        <h2 className="mb-2 font-display text-section font-medium text-ink">
          {t("help.incidents")}
        </h2>
        {incidentsQuery.isLoading && !incidentsOverride ? (
          <Loading />
        ) : incidentsQuery.error ? (
          <ErrorPanel error={incidentsQuery.error} onRetry={() => void incidentsQuery.refetch()} />
        ) : incidents.length === 0 ? (
          <p data-testid="help-no-incidents" className="text-small text-muted">
            {t("help.noIncidents")}
          </p>
        ) : (
          <ul className="grid gap-2">
            {incidents.map((incident) => (
              <li
                key={incident.id}
                data-testid="help-incident"
                className="rounded-lg border border-border bg-surface p-3 text-small"
              >
                <p className="font-medium text-ink">{incident.title}</p>
                <p className={`text-small ${INCIDENT_TONE[incident.severity]}`}>
                  {t("help.severity")}: {incident.severity} · {t("help.status")}: {incident.status}{" "}
                  · {format(incident.declaredAt)}
                </p>
              </li>
            ))}
          </ul>
        )}
      </div>

      <div>
        <h2 className="mb-2 font-display text-section font-medium text-ink">
          {t("help.articles")}
        </h2>
        {articlesQuery.isLoading && !articlesOverride ? (
          <Loading />
        ) : articlesQuery.error ? (
          <ErrorPanel error={articlesQuery.error} onRetry={() => void articlesQuery.refetch()} />
        ) : articles.length === 0 ? (
          <p className="text-small text-muted">{t("common.empty")}</p>
        ) : (
          <ul className="grid gap-2">
            {articles.map((article) => (
              <li
                key={article.id}
                data-testid="help-article"
                className="rounded-lg border border-border bg-surface p-4"
              >
                <h3 className="font-display text-small font-medium text-ink">{article.title}</h3>
                <p className="mt-1 text-small text-ink-2">{article.summary}</p>
                <details className="mt-2 text-small text-ink-2">
                  <summary className="cursor-pointer text-accent">{t("help.read")}</summary>
                  <p className="mt-1">{article.body}</p>
                </details>
              </li>
            ))}
          </ul>
        )}
      </div>

      <div>
        <h2 className="mb-2 font-display text-section font-medium text-ink">
          {t("help.supportRequest")}
        </h2>
        <form
          data-testid="help-form"
          onSubmit={onSubmit}
          className="grid max-w-2xl gap-3 rounded-lg border border-border bg-surface p-4"
        >
          <label className="flex flex-col gap-1 text-small font-medium text-ink-2">
            {t("help.subject")}
            <input
              data-testid="help-subject"
              value={subject}
              onChange={(event) => setSubject(event.target.value)}
              className="rounded-xl border border-border bg-surface px-3 py-2 text-small text-ink"
            />
          </label>
          <label className="flex flex-col gap-1 text-small font-medium text-ink-2">
            {t("help.body")}
            <textarea
              data-testid="help-body"
              rows={4}
              value={body}
              onChange={(event) => setBody(event.target.value)}
              className="rounded-xl border border-border bg-surface px-3 py-2 text-small text-ink"
            />
          </label>
          <button
            type="submit"
            data-testid="help-submit"
            disabled={sending}
            className="w-fit rounded-lg bg-accent px-4 py-2 text-small font-medium text-accent-on disabled:opacity-45"
          >
            {sending ? t("common.loading") : t("help.supportRequest")}
          </button>
        </form>

        {errors.length > 0 ? (
          <ul data-testid="help-errors" role="alert" className="mt-3 text-small text-danger">
            {errors.map((message) => (
              <li key={message}>{message}</li>
            ))}
          </ul>
        ) : null}
        {sent ? (
          <p data-testid="help-sent" role="status" className="mt-3 text-small text-success">
            {t("help.supportRequest")}
          </p>
        ) : null}
      </div>
    </section>
  );
}
