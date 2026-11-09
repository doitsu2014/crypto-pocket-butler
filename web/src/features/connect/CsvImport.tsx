import { useState, type ChangeEvent, type FormEvent } from "react";
import { gatewayClient } from "../../api/client";
import { problemMessages } from "../../api/errors";
import type { ImportBatch } from "../../api/types";
import { useI18n } from "../../i18n/react";

const MAX_BYTES = 5 * 1024 * 1024;

function sleep(ms: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

export default function CsvImport({
  importFile = (file: File) => gatewayClient.createImport(file),
  pollImport = (importId: string) => gatewayClient.getImport(importId),
  pollInterval = 1000,
}: {
  importFile?: (file: File) => Promise<ImportBatch>;
  pollImport?: (importId: string) => Promise<ImportBatch>;
  pollInterval?: number;
} = {}) {
  const { t } = useI18n();
  const [file, setFile] = useState<File | null>(null);
  const [batch, setBatch] = useState<ImportBatch | null>(null);
  const [errors, setErrors] = useState<string[]>([]);
  const [uploading, setUploading] = useState(false);

  function onFile(event: ChangeEvent<HTMLInputElement>) {
    setErrors([]);
    setBatch(null);
    const selected = event.target.files?.[0] ?? null;
    if (!selected) {
      setFile(null);
      return;
    }
    const looksCsv =
      selected.name.toLowerCase().endsWith(".csv") ||
      selected.type === "text/csv" ||
      selected.type === "text/plain";
    if (!looksCsv) {
      setErrors([`${t("connect.csvFile")}: .csv`]);
      setFile(null);
      return;
    }
    if (selected.size > MAX_BYTES) {
      setErrors([`${t("connect.csvFile")}: ≤ 5 MB`]);
      setFile(null);
      return;
    }
    setFile(selected);
  }

  async function onSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setErrors([]);
    if (!file) {
      setErrors([t("common.required")]);
      return;
    }
    setUploading(true);
    try {
      let current = await importFile(file);
      let attempts = 0;
      while (
        pollInterval > 0 &&
        (current.status === "pending" || current.status === "processing") &&
        attempts < 30
      ) {
        await sleep(pollInterval);
        current = await pollImport(current.id);
        attempts += 1;
      }
      setBatch(current);
    } catch (error) {
      setErrors(problemMessages(error));
    } finally {
      setUploading(false);
    }
  }

  return (
    <div data-testid="csv-import" className="grid gap-3">
      <form onSubmit={onSubmit} className="grid gap-3">
        <label className="flex flex-col gap-1 text-small font-medium text-ink-2">
          {t("connect.csvFile")}
          <input
            data-testid="csv-file"
            type="file"
            onChange={onFile}
            className="rounded-xl border border-border bg-surface px-3 py-2 text-small"
          />
        </label>
        <button
          type="submit"
          data-testid="csv-submit"
          disabled={uploading || !file}
          className="w-fit rounded-lg bg-accent px-4 py-2 text-small font-medium text-accent-on disabled:opacity-45"
        >
          {uploading ? t("common.loading") : t("connect.csv")}
        </button>
      </form>

      {errors.length > 0 ? (
        <ul data-testid="csv-errors" role="alert" className="text-small text-danger">
          {errors.map((message) => (
            <li key={message}>{message}</li>
          ))}
        </ul>
      ) : null}

      {batch ? (
        <section
          data-testid="csv-result"
          className="rounded-lg border border-border bg-surface p-4"
        >
          <p className="text-small text-ink-2">
            {t("connect.accepted")}: <span className="font-mono">{batch.accepted}</span> ·{" "}
            {t("connect.skipped")}: <span className="font-mono">{batch.skipped}</span>
          </p>
          {batch.errors.length > 0 ? (
            <ul data-testid="csv-line-errors" className="mt-2 text-small text-danger">
              {batch.errors.map((entry) => (
                <li key={`${entry.line}-${entry.reason}`}>
                  {t("connect.lineErrors")} {entry.line}: {entry.reason}
                </li>
              ))}
            </ul>
          ) : null}
        </section>
      ) : null}
    </div>
  );
}
