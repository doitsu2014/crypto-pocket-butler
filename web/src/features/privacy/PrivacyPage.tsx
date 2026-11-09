import { useState } from "react";
import { ApiError, gatewayClient } from "../../api/client";
import { problemMessages } from "../../api/errors";
import { useConsent } from "../../api/queries";
import type { ConsentRecord, DeletionRequest, ExportJob } from "../../api/types";
import { ConfirmDialog, DataTable, ErrorPanel, Loading, PageHeader } from "../../components";
import { localiseProblem, type Translate } from "../../i18n";
import { useI18n } from "../../i18n/react";

function sleep(ms: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

function describe(error: unknown, t: Translate): string {
  if (error instanceof ApiError) {
    return localiseProblem(error.problem, t);
  }
  return problemMessages(error).join(" · ");
}

export default function PrivacyPage({
  consentOverride,
  startExport,
  getExport,
  requestDeletion,
  pollInterval = 1000,
}: {
  consentOverride?: ConsentRecord[];
  startExport?: () => Promise<ExportJob>;
  getExport?: (exportId: string) => Promise<ExportJob>;
  requestDeletion?: () => Promise<DeletionRequest>;
  pollInterval?: number;
} = {}) {
  const { t, format } = useI18n();
  const consentQuery = useConsent(!consentOverride);
  const consent = consentOverride ?? consentQuery.data ?? [];

  const [job, setJob] = useState<ExportJob | null>(null);
  const [deletion, setDeletion] = useState<DeletionRequest | null>(null);
  const [confirmOpen, setConfirmOpen] = useState(false);
  const [errors, setErrors] = useState<string[]>([]);
  const [working, setWorking] = useState(false);

  async function onExport() {
    setErrors([]);
    setWorking(true);
    try {
      let current = startExport ? await startExport() : await gatewayClient.startExport();
      let attempts = 0;
      while (
        pollInterval > 0 &&
        (current.status === "queued" || current.status === "running") &&
        attempts < 30
      ) {
        await sleep(pollInterval);
        current = getExport
          ? await getExport(current.id)
          : await gatewayClient.getExport(current.id);
        attempts += 1;
      }
      setJob(current);
    } catch (error) {
      setErrors([describe(error, t)]);
    } finally {
      setWorking(false);
    }
  }

  async function onDelete() {
    setErrors([]);
    try {
      const next = requestDeletion
        ? await requestDeletion()
        : await gatewayClient.requestDeletion();
      setDeletion(next);
      setConfirmOpen(false);
    } catch (error) {
      setErrors([describe(error, t)]);
    }
  }

  return (
    <section className="grid gap-8">
      <PageHeader title={t("privacy.title")} />

      <div>
        <h2 className="mb-2 font-display text-section font-medium text-ink">
          {t("privacy.consentHistory")}
        </h2>
        {consentQuery.isLoading && !consentOverride ? (
          <Loading />
        ) : consentQuery.error ? (
          <ErrorPanel error={consentQuery.error} onRetry={() => void consentQuery.refetch()} />
        ) : consent.length === 0 ? (
          <p data-testid="privacy-no-consent" className="text-small text-muted">
            {t("privacy.noConsent")}
          </p>
        ) : (
          <div data-testid="privacy-consent">
            <DataTable
              caption={t("privacy.consentHistory")}
              columns={[
                { key: "purpose", header: t("privacy.purpose") },
                { key: "granted", header: t("privacy.granted") },
                { key: "basis", header: t("privacy.legalBasis") },
                { key: "recordedAt", header: t("privacy.recordedAt") },
              ]}
              rows={consent.map((record) => ({
                id: `${record.purpose}-${record.recordedAt}`,
                purpose: record.purpose,
                granted: record.granted ? t("privacy.granted") : t("privacy.notGranted"),
                basis: record.legalBasis,
                recordedAt: format(record.recordedAt),
              }))}
            />
          </div>
        )}
      </div>

      <div>
        <h2 className="mb-2 font-display text-section font-medium text-ink">
          {t("privacy.export")}
        </h2>
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
          <div className="mt-3 grid gap-2 text-small text-ink-2">
            <p data-testid="privacy-export-progress">
              {t("privacy.exportProgress")}: {job.status} · {job.progress}%
            </p>
            {job.status === "complete" && job.downloadUrl ? (
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
      </div>

      <div>
        <h2 className="mb-2 font-display text-section font-medium text-ink">
          {t("privacy.deletion")}
        </h2>
        <p className="mb-3 max-w-prose text-small text-muted">{t("privacy.deletionWarning")}</p>
        <button
          type="button"
          data-testid="privacy-delete"
          onClick={() => setConfirmOpen(true)}
          className="w-fit rounded-lg bg-danger px-4 py-2 text-small font-medium text-accent-on"
        >
          {t("privacy.deletion")}
        </button>

        {deletion ? (
          <p
            data-testid="privacy-deletion-status"
            role="status"
            className="mt-3 text-small text-ink-2"
          >
            {t("privacy.deletion")}: {deletion.status}
          </p>
        ) : null}
      </div>

      {errors.length > 0 ? (
        <ul data-testid="privacy-errors" role="alert" className="text-small text-danger">
          {errors.map((message) => (
            <li key={message}>{message}</li>
          ))}
        </ul>
      ) : null}

      <ConfirmDialog
        open={confirmOpen}
        title={t("privacy.deletion")}
        description={t("privacy.deletionWarning")}
        confirmLabel={t("common.confirm")}
        onConfirm={() => void onDelete()}
        onCancel={() => setConfirmOpen(false)}
      />
    </section>
  );
}
