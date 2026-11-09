import { useParams } from "react-router-dom";
import { useSnapshots } from "../../api/queries";
import type { Snapshot } from "../../api/types";
import {
  AsOf,
  DataStatusBadge,
  DataTable,
  EmptyState,
  ErrorPanel,
  Loading,
  Money,
  PageHeader,
} from "../../components";
import { useI18n } from "../../i18n/react";

export default function HistoryPage({
  portfolioId: portfolioIdProp,
  snapshotsOverride,
}: {
  portfolioId?: string;
  snapshotsOverride?: Snapshot[];
} = {}) {
  const { t } = useI18n();
  const params = useParams();
  const portfolioId = portfolioIdProp ?? params.portfolioId;
  const query = useSnapshots(snapshotsOverride ? undefined : portfolioId);
  const snapshots = snapshotsOverride ?? query.data;

  return (
    <section>
      <PageHeader title={t("portfolio.history")} />

      {query.isLoading ? (
        <Loading />
      ) : query.error ? (
        <ErrorPanel error={query.error} onRetry={() => void query.refetch()} />
      ) : !snapshots || snapshots.length === 0 ? (
        <EmptyState title={t("portfolio.noSnapshots")} />
      ) : (
        <DataTable
          caption={t("portfolio.history")}
          columns={[
            { key: "asOf", header: t("common.asOf") },
            { key: "value", header: t("portfolio.value"), numeric: true },
            { key: "status", header: t("activity.coverage") },
            { key: "verified", header: t("common.confirm") },
          ]}
          rows={snapshots.map((snapshot) => ({
            id: snapshot.id,
            asOf: <AsOf at={snapshot.asOf} />,
            value: <Money value={snapshot.totalValue} currency={snapshot.currency} />,
            status: (
              <DataStatusBadge status={snapshot.dataStatus ?? "complete"} asOf={snapshot.asOf} />
            ),
            verified: snapshot.verified ? t("common.confirm") : "—",
          }))}
        />
      )}
    </section>
  );
}
