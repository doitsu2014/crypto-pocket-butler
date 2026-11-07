//! Postgres implementation of [`AggregateRepository`].

use analytics_application::ports::{AggregateRepository, RepositoryError};
use analytics_domain::{Metric, MetricAggregate, MetricAggregateId, Period};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use sqlx::{PgPool, Postgres, QueryBuilder};
use uuid::Uuid;

use super::{invalid_data, map_sqlx_error};

const AGGREGATE_COLUMNS: &str = "id, metric, period, period_start, cohort, value, computed_at";

/// sqlx-backed aggregate store.
#[derive(Debug, Clone)]
pub struct PgAggregateRepository {
    pool: PgPool,
}

impl PgAggregateRepository {
    /// Creates the repository over a pool.
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[derive(sqlx::FromRow)]
struct AggregateRow {
    id: Uuid,
    metric: String,
    period: String,
    period_start: DateTime<Utc>,
    cohort: String,
    value: Decimal,
    computed_at: DateTime<Utc>,
}

impl AggregateRow {
    fn into_domain(self) -> Result<MetricAggregate, RepositoryError> {
        MetricAggregate::from_stored(
            MetricAggregateId::new(self.id),
            &self.metric,
            &self.period,
            self.period_start,
            &self.cohort,
            self.value,
            self.computed_at,
        )
        .map_err(|error| invalid_data("read aggregate", error))
    }
}

#[async_trait]
impl AggregateRepository for PgAggregateRepository {
    async fn upsert(&self, aggregate: &MetricAggregate) -> Result<(), RepositoryError> {
        sqlx::query(
            "INSERT INTO metric_aggregates
                 (id, metric, period, period_start, cohort, value, computed_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7)
             ON CONFLICT (metric, period, cohort, period_start)
             DO UPDATE SET value = EXCLUDED.value, computed_at = EXCLUDED.computed_at",
        )
        .bind(aggregate.id().as_uuid())
        .bind(aggregate.metric().as_str())
        .bind(aggregate.period().as_str())
        .bind(aggregate.period_start())
        .bind(aggregate.cohort())
        .bind(aggregate.value())
        .bind(aggregate.computed_at())
        .execute(&self.pool)
        .await
        .map_err(|error| map_sqlx_error("upsert aggregate", error))?;
        Ok(())
    }

    async fn list(
        &self,
        metric: Option<Metric>,
        period: Option<Period>,
        from: Option<DateTime<Utc>>,
        to: Option<DateTime<Utc>>,
        limit: u32,
    ) -> Result<Vec<MetricAggregate>, RepositoryError> {
        let mut builder = QueryBuilder::<Postgres>::new(format!(
            "SELECT {AGGREGATE_COLUMNS} FROM metric_aggregates WHERE 1 = 1"
        ));
        if let Some(metric) = metric {
            builder.push(" AND metric = ");
            builder.push_bind(metric.as_str());
        }
        if let Some(period) = period {
            builder.push(" AND period = ");
            builder.push_bind(period.as_str());
        }
        if let Some(from) = from {
            builder.push(" AND period_start >= ");
            builder.push_bind(from);
        }
        if let Some(to) = to {
            builder.push(" AND period_start < ");
            builder.push_bind(to);
        }
        builder.push(" ORDER BY period_start DESC, metric, cohort LIMIT ");
        builder.push_bind(i64::from(limit.max(1)));

        let rows = builder
            .build_query_as::<AggregateRow>()
            .fetch_all(&self.pool)
            .await
            .map_err(|error| map_sqlx_error("list aggregates", error))?;
        rows.into_iter().map(AggregateRow::into_domain).collect()
    }
}
