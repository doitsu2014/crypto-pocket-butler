import { useState } from "react";
import { Link } from "react-router-dom";
import { DataTable, Loading, PageHeader } from "../../components";
import { useI18n } from "../../i18n/react";
import { gatewayClient } from "../../api/client";
import { useConsentHistory } from "../../api/settingsQueries";
import type { ExportJob } from "../../api/types";
import { correlationOf, problemMessage } from "./problemText";

function sleep(ms: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

/**
 * Privacy panel (functional-spec §3.4, X4 + the unchanged X7 export). It reads
 * the consent history, starts and polls an export exactly as the old page did,
 * and links to the delete page — it never initiates a deletion itself.
 */
export default function PrivacyPanel({ pollInterval = 1000 }: { pollInterval?: number } = {}) {
  const { t, format } = useI18n();
  const historyQuery = useConsentHistory();
  const [job, setJob] = useState<ExportJob | null>(null);
  const [exportError, setExportError] = useState<unknown>(null);
  const [working, setWorking] = useState(false);

  async function onExport() {
    setExportError(null);
    setWorking(true);
    try {
      let current = await gatewayClient.startExport();
      let attempts = 0;
      while (
        pollInterval > 0 &&
        (current.status === "queued" || current.status === "running") &&
        attempts < 30
      ) {
        await sleep(pollInterval);
        current = await gatewayClient.getExport(current.id);
        attempts += 1;
      }
      setJob(current);
    } catch (error) {
      setExportError(error);
    } finally {
      setWorking(false);
    }
  }

  const events = historyQuery.data?.events ?? [];

  return (
    <section className="grid gap-8">
      <PageHeader title={t("settings.privacy.title")} />

      <div className="grid gap-2">
        <h2 className="font-display text-section font-medium text-ink">
          {t("privacy.consentHistory")}
        </h2>
        {historyQuery.isLoading ? (
          <Loading />
        ) : historyQuery.isError ? (
          <div role="alert" data-testid="privacy-consent-error" className="text-small text-danger">
            <p>{problemMessage(historyQuery.error, t)}</p>
            <button
              type="button"
              data-testid="privacy-consent-retry"
              onClick={() => void historyQuery.refetch()}
              className="mt-2 rounded-lg bg-surface-warm px-3 py-1 font-medium text-ink"
            >
              {t("common.retry")}
            </button>
          </div>
        ) : events.length === 0 ? (
          <p data-testid="privacy-no-consent" className="text-small text-muted">
            {t("privacy.noConsent")}
          </p>
        ) : (
          <div data-testid="privacy-consent">
            <DataTable
              caption={t("privacy.consentHistory")}
              columns={[
                { key: "kind", header: t("settings.privacy.kind") },
                { key: "action", header: t("settings.privacy.action") },
                { key: "occurredAt", header: t("settings.privacy.occurredAt") },
                { key: "policyVersion", header: t("settings.privacy.policyVersion") },
              ]}
              rows={events.map((event) => ({
                id: event.id,
                kind: t(`settings.consent.kind.${event.kind}`),
                action: t(`settings.privacy.action.${event.action}`),
                occurredAt: format(event.occurredAt),
                policyVersion: event.policyVersion,
              }))}
            />
          </div>
        )}
      </div>

      <div className="grid gap-2">
        <h2 className="font-display text-section font-medium text-ink">{t("privacy.export")}</h2>
        <button
          type="button"
          data-testid="privacy-export"
          disabled={working}
          onClick={() => void onExport()}
          className="w-fit rounded-lg bg-accent px-4 py-2 text-small font-medium text-accent-on disabled:opacity-45"
        >
          {working ? t("common.loading") : t("privacy.export")}
        </button>
        {job ? (
          <div className="grid gap-2 text-small text-ink-2">
            <p data-testid="privacy-export-progress">
              {t("privacy.exportProgress")}: {job.status} · {job.progress}%
            </p>
            {job.downloadUrl ? (
              <a
                data-testid="privacy-download"
                href={job.downloadUrl}
                className="w-fit rounded-lg bg-surface-warm px-3 py-1 text-small font-medium text-ink underline"
              >
                {t("privacy.download")}
              </a>
            ) : null}
          </div>
        ) : null}
        {exportError ? (
          <p role="alert" data-testid="privacy-export-error" className="text-small text-danger">
            {problemMessage(exportError, t)}
            {correlationOf(exportError) ? (
              <span className="ml-2 text-meta">
                {t("common.correlationId")}: {correlationOf(exportError)}
              </span>
            ) : null}
          </p>
        ) : null}
      </div>

      <div
        data-testid="privacy-danger-zone"
        className="grid gap-2 rounded-lg border border-danger p-4"
      >
        <h2 className="font-display text-section font-medium text-danger">
          {t("settings.privacy.dangerZone")}
        </h2>
        <p className="max-w-prose text-small text-ink-2">{t("settings.privacy.dangerZoneBody")}</p>
        <Link
          to="/settings/delete"
          data-testid="privacy-delete-link"
          className="w-fit rounded-lg bg-danger px-4 py-2 text-small font-medium text-accent-on"
        >
          {t("settings.privacy.deleteAccount")}
        </Link>
      </div>
    </section>
  );
}
