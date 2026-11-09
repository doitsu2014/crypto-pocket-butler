import { useState } from "react";
import { Link } from "react-router-dom";
import { useBriefs, useProfile } from "../../api/queries";
import type { Brief } from "../../api/types";
import {
  EmptyState,
  ErrorPanel,
  Loading,
  PageHeader,
} from "../../components";
import { useI18n } from "../../i18n/react";

const STATUS_KEY = {
  pending: "briefs.pending",
  generated: "briefs.generated",
  failed: "briefs.failed",
} as const;

const STATUS_TONE = {
  pending: "text-warn",
  generated: "text-success",
  failed: "text-danger",
} as const;

export default function BriefsPage({
  userId: userIdProp,
  briefsOverride,
}: {
  userId?: string;
  briefsOverride?: Brief[];
} = {}) {
  const { t, format } = useI18n();
  const profile = useProfile(!userIdProp);
  const userId = userIdProp ?? profile.data?.userId;
  const [period, setPeriod] = useState<"daily" | "weekly">("daily");
  const query = useBriefs(briefsOverride ? undefined : userId, period);
  const briefs = briefsOverride ?? query.data;

  return (
    <section>
      <PageHeader
        title={t("briefs.title")}
        action={
          <label className="text-small text-ink-2">
            {t("briefs.period")}
            <select
              data-testid="briefs-period"
              value={period}
              onChange={(event) =>
                setPeriod(event.target.value as "daily" | "weekly")
              }
              className="ml-2 rounded-xl border border-border bg-surface px-3 py-1.5 text-small"
            >
              <option value="daily">Daily</option>
              <option value="weekly">Weekly</option>
            </select>
          </label>
        }
      />

      {query.isLoading ? (
        <Loading />
      ) : query.error ? (
        <ErrorPanel error={query.error} onRetry={() => void query.refetch()} />
      ) : !briefs || briefs.length === 0 ? (
        <EmptyState title={t("briefs.noBriefs")} />
      ) : (
        <ul className="grid gap-2">
          {briefs.map((brief) => (
            <li
              key={brief.id}
              data-testid="brief-row"
              className="flex flex-wrap items-center justify-between gap-2 rounded-lg border border-border bg-surface p-4"
            >
              <div>
                <Link
                  to={`/briefs/${brief.id}`}
                  className="font-display text-section font-medium text-ink underline"
                >
                  {brief.period}
                </Link>
                <p className="text-small text-meta">
                  {format(brief.periodStart)} – {format(brief.periodEnd)}
                </p>
              </div>
              <span className={`text-small ${STATUS_TONE[brief.status]}`}>
                {t(STATUS_KEY[brief.status])}
              </span>
              {brief.status === "failed" && brief.failureReason ? (
                <p data-testid="brief-failure" className="w-full text-small text-danger">
                  {t("briefs.failureReason")}: {brief.failureReason}
                </p>
              ) : null}
            </li>
          ))}
        </ul>
      )}
    </section>
  );
}
