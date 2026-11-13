//! Postgres implementations of the jurisdiction ports.
//!
//! - [`JurisdictionRepository`] reads the pre-sign-up gate's age and disclosure
//!   rules (BR7.1–BR7.3).
//! - [`PgJurisdictionListRepository`] reads the active runtime list version and
//!   stores a new immutable one (BR1.6, BR1.10, W3).

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use identity_application::jurisdiction::ListRepository;
use identity_application::ports::{JurisdictionRepository, RepositoryError};
use identity_domain::{
    CountryCode, JurisdictionListEntry, JurisdictionListVersion, JurisdictionRule,
    JurisdictionStatus,
};
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

// ---------------------------------------------------------------------------
// U6 — the versioned jurisdiction list (BR1.6, BR1.10, W3)
// ---------------------------------------------------------------------------

/// sqlx-backed list versions and their rules.
#[derive(Debug, Clone)]
pub struct PgJurisdictionListRepository {
    pool: PgPool,
}

impl PgJurisdictionListRepository {
    /// Creates the repository over a pool.
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[derive(sqlx::FromRow)]
struct VersionRow {
    version: String,
    updated_at: DateTime<Utc>,
    entry_count: i32,
    checksum: String,
}

#[derive(sqlx::FromRow)]
struct EntryRow {
    country_code: String,
    status: String,
    source: String,
    updated_at: DateTime<Utc>,
}

impl EntryRow {
    fn into_domain(self) -> Result<JurisdictionListEntry, RepositoryError> {
        Ok(JurisdictionListEntry {
            code: CountryCode::parse(&self.country_code)
                .map_err(|error| invalid_data("read jurisdiction list entry", error))?,
            status: JurisdictionStatus::parse(&self.status)
                .map_err(|error| invalid_data("read jurisdiction list entry", error))?,
            source: self.source,
            updated_at: self.updated_at,
        })
    }
}

#[async_trait]
impl ListRepository for PgJurisdictionListRepository {
    async fn active(&self) -> Result<Option<JurisdictionListVersion>, RepositoryError> {
        let version = sqlx::query_as::<_, VersionRow>(
            "SELECT version, updated_at, entry_count, checksum
             FROM jurisdiction_list_versions
             WHERE status = 'active'
             ORDER BY updated_at DESC, version DESC
             LIMIT 1",
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|error| map_sqlx_error("find active jurisdiction list", error))?;
        let Some(version) = version else {
            return Ok(None);
        };

        let rows = sqlx::query_as::<_, EntryRow>(
            "SELECT country_code, status, source, updated_at
             FROM jurisdiction_list_rules
             WHERE version = $1
             ORDER BY country_code",
        )
        .bind(&version.version)
        .fetch_all(&self.pool)
        .await
        .map_err(|error| map_sqlx_error("read jurisdiction list rules", error))?;

        if i32::try_from(rows.len()).ok() != Some(version.entry_count) {
            return Err(RepositoryError::InvalidData(format!(
                "read jurisdiction list: version {} stored {} entries but its rules number {}",
                version.version,
                version.entry_count,
                rows.len()
            )));
        }

        let entries = rows
            .into_iter()
            .map(EntryRow::into_domain)
            .collect::<Result<Vec<_>, _>>()?;

        let list = JurisdictionListVersion::from_stored(
            version.version,
            version.updated_at,
            &version.checksum,
            entries,
        )
        .map_err(|error| invalid_data("read jurisdiction list", error))?;
        Ok(Some(list))
    }

    async fn publish(&self, list: &JurisdictionListVersion) -> Result<(), RepositoryError> {
        let mut transaction = self
            .pool
            .begin()
            .await
            .map_err(|error| map_sqlx_error("begin jurisdiction publish", error))?;

        sqlx::query(
            "UPDATE jurisdiction_list_versions
             SET status = 'superseded'
             WHERE status = 'active'",
        )
        .execute(&mut *transaction)
        .await
        .map_err(|error| map_sqlx_error("supersede the previous jurisdiction list", error))?;

        sqlx::query(
            "INSERT INTO jurisdiction_list_versions
                 (version, updated_at, entry_count, checksum, status)
             VALUES ($1, $2, $3, $4, 'active')",
        )
        .bind(list.version())
        .bind(list.updated_at())
        .bind(i32::try_from(list.entry_count()).unwrap_or(i32::MAX))
        .bind(list.checksum())
        .execute(&mut *transaction)
        .await
        .map_err(|error| map_sqlx_error("insert the jurisdiction list version", error))?;

        for entry in list.entries() {
            sqlx::query(
                "INSERT INTO jurisdiction_list_rules
                     (version, country_code, status, source, updated_at)
                 VALUES ($1, $2, $3, $4, $5)",
            )
            .bind(list.version())
            .bind(entry.code.as_str())
            .bind(entry.status.as_str())
            .bind(entry.source.trim())
            .bind(entry.updated_at)
            .execute(&mut *transaction)
            .await
            .map_err(|error| map_sqlx_error("insert a jurisdiction list rule", error))?;
        }

        transaction
            .commit()
            .await
            .map_err(|error| map_sqlx_error("commit the jurisdiction publish", error))
    }
}
