//! Postgres implementation of [`DeliveryRepository`].

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use reporting_application::ports::{DeliveryRepository, RepositoryError};
use reporting_domain::{BriefDelivery, BriefDeliveryId, BriefId, DeliveryChannel, DeliveryStatus};
use sqlx::PgPool;
use uuid::Uuid;

use super::{invalid_data, map_error};

const DELIVERY_COLUMNS: &str = "id, brief_id, channel, alert_id, status, submitted_at";

/// sqlx-backed delivery store.
#[derive(Debug, Clone)]
pub struct PgDeliveryRepository {
    pool: PgPool,
}

impl PgDeliveryRepository {
    /// Creates the repository over a pool.
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[derive(sqlx::FromRow)]
struct DeliveryRow {
    id: Uuid,
    brief_id: Uuid,
    channel: String,
    alert_id: Option<String>,
    status: String,
    submitted_at: DateTime<Utc>,
}

impl DeliveryRow {
    fn into_domain(self) -> Result<BriefDelivery, RepositoryError> {
        let channel = DeliveryChannel::parse(&self.channel)
            .map_err(|error| invalid_data("read brief delivery", error))?;
        let status = DeliveryStatus::parse(&self.status)
            .map_err(|error| invalid_data("read brief delivery", error))?;
        BriefDelivery::new(
            BriefDeliveryId::new(self.id),
            BriefId::new(self.brief_id),
            channel,
            self.alert_id,
            status,
            self.submitted_at,
        )
        .map_err(|error| invalid_data("read brief delivery", error))
    }
}

#[async_trait]
impl DeliveryRepository for PgDeliveryRepository {
    async fn upsert(&self, delivery: &BriefDelivery) -> Result<(), RepositoryError> {
        sqlx::query(
            "INSERT INTO brief_deliveries
                 (id, brief_id, channel, alert_id, status, submitted_at)
             VALUES ($1, $2, $3, $4, $5, $6)
             ON CONFLICT (brief_id, channel) DO UPDATE
             SET alert_id = EXCLUDED.alert_id,
                 status = EXCLUDED.status,
                 submitted_at = EXCLUDED.submitted_at",
        )
        .bind(delivery.id().as_uuid())
        .bind(delivery.brief_id().as_uuid())
        .bind(delivery.channel().as_str())
        .bind(delivery.alert_id())
        .bind(delivery.status().as_str())
        .bind(delivery.submitted_at())
        .execute(&self.pool)
        .await
        .map_err(map_error)?;
        Ok(())
    }

    async fn list_for_brief(
        &self,
        brief_id: BriefId,
    ) -> Result<Vec<BriefDelivery>, RepositoryError> {
        let rows = sqlx::query_as::<_, DeliveryRow>(&format!(
            "SELECT {DELIVERY_COLUMNS} FROM brief_deliveries
             WHERE brief_id = $1
             ORDER BY submitted_at ASC"
        ))
        .bind(brief_id.as_uuid())
        .fetch_all(&self.pool)
        .await
        .map_err(map_error)?;
        rows.into_iter().map(DeliveryRow::into_domain).collect()
    }
}
