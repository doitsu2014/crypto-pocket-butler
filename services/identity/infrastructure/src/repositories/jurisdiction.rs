//! Postgres implementation of [`JurisdictionRepository`].

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use identity_application::ports::{JurisdictionRepository, RepositoryError};
use identity_domain::{CountryCode, JurisdictionRule};
use sqlx::PgPool;

use super::{invalid_data, map_sqlx_error};

/// sqlx-backed jurisdiction rules.
#[derive(Debug, Clone)]
pub struct PgJurisdictionRepository {
    pool: PgPool,
}

impl PgJurisdictionRepository {
    /// Creates the repository over a pool.
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[derive(sqlx::FromRow)]
struct RuleRow {
    code: String,
    allowed: bool,
    minimum_age: i32,
    disclosures_required: bool,
    updated_at: DateTime<Utc>,
}

impl RuleRow {
    fn into_domain(self) -> Result<JurisdictionRule, RepositoryError> {
        Ok(JurisdictionRule {
            code: CountryCode::parse(&self.code)
                .map_err(|error| invalid_data("read jurisdiction rule", error))?,
            allowed: self.allowed,
            minimum_age: u32::try_from(self.minimum_age)
                .map_err(|error| invalid_data("read jurisdiction rule", error))?,
            disclosures_required: self.disclosures_required,
            updated_at: self.updated_at,
        })
    }
}

#[async_trait]
impl JurisdictionRepository for PgJurisdictionRepository {
    async fn find_by_code(&self, code: &str) -> Result<Option<JurisdictionRule>, RepositoryError> {
        let row = sqlx::query_as::<_, RuleRow>(
            "SELECT code, allowed, minimum_age, disclosures_required, updated_at
             FROM jurisdiction_rules WHERE code = $1",
        )
        .bind(code.trim().to_uppercase())
        .fetch_optional(&self.pool)
        .await
        .map_err(|error| map_sqlx_error("find jurisdiction rule", error))?;
        row.map(RuleRow::into_domain).transpose()
    }
}
