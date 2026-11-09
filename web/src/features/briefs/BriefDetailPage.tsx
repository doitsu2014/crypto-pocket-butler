import { useParams } from "react-router-dom";
import { useBrief, useProfile } from "../../api/queries";
import type { Brief } from "../../api/types";
import {
  AsOf,
  DataStatusBadge,
  DataTable,
  ErrorPanel,
  Loading,
  Money,
  PageHeader,
} from "../../components";
import { useI18n } from "../../i18n/react";

export default function BriefDetailPage({
  userId: userIdProp,
  briefId: briefIdProp,
  briefOverride,
}: {
  userId?: string;
  briefId?: string;
  briefOverride?: Brief;
} = {}) {
  const { t, format } = useI18n();
  const params = useParams();
  const briefId = briefIdProp ?? params.briefId;
  const profile = useProfile(!userIdProp);
  const userId = userIdProp ?? profile.data?.userId;
  const query = useBrief(
    briefOverride ? undefined : userId,
    briefOverride ? undefined : briefId,
  );
  const brief = briefOverride ?? query.data;

  if (query.isLoading) {
    return <Loading />;
  }
  if (query.error) {
    return <ErrorPanel error={query.error} onRetry={() => void query.refetch()} />;
  }
  if (!brief) {
    return <Loading />;
  }

  return (
    <section className="grid gap-4">
      <PageHeader
        title={`${t("briefs.title")} · ${brief.period}`}
        description={`${format(brief.periodStart)} – ${format(brief.periodEnd)}`}
      />

      <div className="flex flex-wrap items-center gap-3">
        <span className="text-small text-ink-2">
          {t(`briefs.${brief.status}`)}
        </span>
        <DataStatusBadge status={brief.dataStatus} asOf={brief.asOf} />
        <AsOf at={brief.generatedAt} />
      </div>

      {brief.status === "failed" ? (
        <p data-testid="brief-failure-reason" role="alert" className="rounded-lg border border-danger bg-surface p-3 text-small text-danger">
          {t("briefs.failureReason")}: {brief.failureReason ?? t("common.unavailable")}
        </p>
      ) : null}

      {brief.sections.map((section) => (
        <section key={section.title} className="rounded-lg border border-border bg-surface p-4">
          <h2 className="font-display text-section font-medium text-ink">
            {section.title}
          </h2>
          {section.text ? <p className="mt-2 text-small text-ink-2">{section.text}</p> : null}
          {section.lines && section.lines.length > 0 ? (
            <div className="mt-3">
              <DataTable
                caption={section.title}
                columns={[
                  { key: "asset", header: t("portfolio.asset") },
                  { key: "quantity", header: t("portfolio.quantity"), numeric: true },
                  { key: "value", header: t("portfolio.value"), numeric: true },
                ]}
                rows={section.lines.map((line) => ({
                  id: line.assetId,
                  asset: line.symbol ?? line.assetId,
                  quantity: line.quantity,
                  value: <Money value={line.value} />,
                }))}
              />
            </div>
          ) : null}
        </section>
      ))}
    </section>
  );
}
