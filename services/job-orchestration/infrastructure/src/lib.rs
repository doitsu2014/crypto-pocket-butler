//! Job Orchestration adapters: sqlx repositories, the provider client, JWKS
//! validation, the service-token registry and the clock.

use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::Duration as StdDuration;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use job_orchestration_application::ports::{
    AuditClient, AuditEvent, AuthError, Clock, ExternalError, HeartbeatRepository, IdGenerator,
    JobDefinition, LeaseRepository, ReadinessProbe, RegisteredService, RepositoryError,
    RunRepository, ServiceTokenRegistry, TargetAnswer, TargetClient, TokenValidator,
    VerifiedClaims,
};
use job_orchestration_domain::{JobRun, Operation, RunId, RunStatus, Trigger, UserId};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Database and migrations
// ---------------------------------------------------------------------------

/// Connects to the `job_orchestration` schema.
///
/// # Errors
/// Returns the sqlx error when the database is unreachable.
pub async fn connect(database_url: &str, max_connections: u32) -> Result<PgPool, sqlx::Error> {
    PgPoolOptions::new()
        .max_connections(max_connections)
        .min_connections(max_connections.min(5))
        .acquire_timeout(StdDuration::from_secs(5))
        .after_connect(|connection, _meta| {
            Box::pin(async move {
                sqlx::query("SET search_path TO job_orchestration")
                    .execute(&mut *connection)
                    .await?;
                sqlx::query("SET statement_timeout = '10s'")
                    .execute(&mut *connection)
                    .await?;
                Ok(())
            })
        })
        .connect(database_url)
        .await
}

/// Migrations embedded from `services/job-orchestration/migrations`.
pub static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("../migrations");

/// Applies every pending migration.
///
/// # Errors
/// Returns the migration error when a migration fails.
pub async fn migrate(pool: &PgPool) -> Result<(), sqlx::migrate::MigrateError> {
    MIGRATOR.run(pool).await
}

/// Deletes runs older than the retention window, in bounded batches.
///
/// # Errors
/// Returns the sqlx error when a delete fails.
pub async fn prune_runs(pool: &PgPool, older_than_days: i64) -> Result<u64, sqlx::Error> {
    let mut total = 0_u64;
    loop {
        let result = sqlx::query(
            "DELETE FROM job_runs WHERE id IN (
                 SELECT id FROM job_runs
                 WHERE scheduled_for < now() - make_interval(days => $1)
                 LIMIT 50000
             )",
        )
        .bind(older_than_days)
        .execute(pool)
        .await?;
        total += result.rows_affected();
        if result.rows_affected() < 50_000 {
            break;
        }
    }
    sqlx::query("UPDATE prune_state SET last_pruned_at = now() WHERE name = 'job-orchestrator'")
        .execute(pool)
        .await?;
    Ok(total)
}

fn map_error(error: sqlx::Error) -> RepositoryError {
    if let sqlx::Error::Database(database) = &error {
        if let Some(code) = database.code() {
            if code == "23505" {
                return RepositoryError::Conflict("a uniqueness rule was violated".to_string());
            }
            if code == "42501" {
                return RepositoryError::Forbidden("the role lacks the grant".to_string());
            }
        }
    }
    RepositoryError::Unavailable(error.to_string())
}

fn invalid(context: &str, detail: impl std::fmt::Display) -> RepositoryError {
    RepositoryError::InvalidData(format!("{context}: {detail}"))
}

// ---------------------------------------------------------------------------
// Run repository
// ---------------------------------------------------------------------------

const RUN_COLUMNS: &str =
    "id, job, scheduled_for, attempt, status, trigger, actor_id, reason, owner, \
     lease_until, next_attempt_at, started_at, finished_at, response_status, detail";

#[derive(sqlx::FromRow)]
struct RunRow {
    id: Uuid,
    job: String,
    scheduled_for: DateTime<Utc>,
    attempt: i32,
    status: String,
    trigger: String,
    actor_id: Option<Uuid>,
    reason: Option<String>,
    owner: Option<String>,
    lease_until: Option<DateTime<Utc>>,
    next_attempt_at: Option<DateTime<Utc>>,
    started_at: Option<DateTime<Utc>>,
    finished_at: Option<DateTime<Utc>>,
    response_status: Option<i16>,
    detail: Option<String>,
}

impl RunRow {
    fn into_domain(self) -> Result<JobRun, RepositoryError> {
        Ok(JobRun {
            id: RunId::new(self.id),
            job: self.job,
            scheduled_for: self.scheduled_for,
            attempt: u32::try_from(self.attempt)
                .map_err(|_| invalid("read run", "attempt is not positive"))?,
            status: RunStatus::parse(&self.status).map_err(|error| invalid("read run", error))?,
            trigger: Trigger::parse(&self.trigger).map_err(|error| invalid("read run", error))?,
            actor_id: self.actor_id.map(UserId::new),
            reason: self.reason,
            owner: self.owner,
            lease_until: self.lease_until,
            next_attempt_at: self.next_attempt_at,
            started_at: self.started_at,
            finished_at: self.finished_at,
            response_status: self
                .response_status
                .and_then(|status| u16::try_from(status).ok()),
            detail: self.detail,
        })
    }
}

/// sqlx-backed runs.
#[derive(Debug, Clone)]
pub struct PgRunRepository {
    pool: PgPool,
}

impl PgRunRepository {
    /// Creates the repository.
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl RunRepository for PgRunRepository {
    async fn definitions(&self) -> Result<Vec<JobDefinition>, RepositoryError> {
        #[derive(sqlx::FromRow)]
        struct Row {
            name: String,
            provider: String,
            schedule: String,
            timeout_seconds: i32,
            max_attempts: i32,
            enabled: bool,
            last_scheduled_for: Option<DateTime<Utc>>,
        }
        let rows = sqlx::query_as::<_, Row>(
            "SELECT name, provider, schedule, timeout_seconds, max_attempts, enabled, last_scheduled_for
             FROM job_definitions WHERE enabled ORDER BY name",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(map_error)?;
        Ok(rows
            .into_iter()
            .map(|row| JobDefinition {
                name: row.name,
                provider: row.provider,
                schedule: row.schedule,
                timeout_seconds: u64::try_from(row.timeout_seconds).unwrap_or(3_600),
                max_attempts: u32::try_from(row.max_attempts).unwrap_or(5),
                enabled: row.enabled,
                last_scheduled_for: row.last_scheduled_for,
            })
            .collect())
    }

    async fn insert(&self, run: &JobRun) -> Result<(), RepositoryError> {
        sqlx::query(
            "INSERT INTO job_runs (id, job, scheduled_for, attempt, status, trigger, actor_id, reason)
             VALUES ($1, $2, $3, $4, 'pending', $5, $6, $7)
             ON CONFLICT (job, scheduled_for, attempt) DO NOTHING",
        )
        .bind(run.id.as_uuid())
        .bind(&run.job)
        .bind(run.scheduled_for)
        .bind(i32::try_from(run.attempt).unwrap_or(1))
        .bind(run.trigger.as_str())
        .bind(run.actor_id.map(|actor| actor.as_uuid()))
        .bind(&run.reason)
        .execute(&self.pool)
        .await
        .map_err(map_error)?;
        Ok(())
    }

    async fn latest_for_window(
        &self,
        job: &str,
        scheduled_for: DateTime<Utc>,
    ) -> Result<Option<JobRun>, RepositoryError> {
        let row = sqlx::query_as::<_, RunRow>(&format!(
            "SELECT {RUN_COLUMNS} FROM job_runs
             WHERE job = $1 AND scheduled_for = $2
             ORDER BY attempt DESC LIMIT 1"
        ))
        .bind(job)
        .bind(scheduled_for)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_error)?;
        row.map(RunRow::into_domain).transpose()
    }

    async fn window_exists(
        &self,
        job: &str,
        scheduled_for: DateTime<Utc>,
    ) -> Result<bool, RepositoryError> {
        sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS (
                 SELECT 1 FROM job_runs
                 WHERE job = $1 AND scheduled_for = $2
                   AND (status IN ('succeeded', 'skipped', 'abandoned', 'pending', 'running')
                        OR (status = 'failed' AND next_attempt_at IS NOT NULL))
             )",
        )
        .bind(job)
        .bind(scheduled_for)
        .fetch_one(&self.pool)
        .await
        .map_err(map_error)
    }

    async fn pending(&self, limit: i64) -> Result<Vec<JobRun>, RepositoryError> {
        let rows = sqlx::query_as::<_, RunRow>(&format!(
            "SELECT {RUN_COLUMNS} FROM job_runs
             WHERE status = 'pending'
             ORDER BY scheduled_for ASC LIMIT $1"
        ))
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(map_error)?;
        rows.into_iter().map(RunRow::into_domain).collect()
    }

    async fn claim(
        &self,
        id: RunId,
        owner: &str,
        lease_until: DateTime<Utc>,
        started_at: DateTime<Utc>,
    ) -> Result<bool, RepositoryError> {
        let result = sqlx::query(
            "UPDATE job_runs
             SET status = 'running', owner = $2, lease_until = $3, started_at = $4
             WHERE id = $1 AND status = 'pending'",
        )
        .bind(id.as_uuid())
        .bind(owner)
        .bind(lease_until)
        .bind(started_at)
        .execute(&self.pool)
        .await
        .map_err(map_error)?;
        Ok(result.rows_affected() == 1)
    }

    async fn finish(
        &self,
        run: &JobRun,
        owner: &str,
        _now: DateTime<Utc>,
    ) -> Result<bool, RepositoryError> {
        let result = sqlx::query(
            "UPDATE job_runs
             SET status = $2, response_status = $3, detail = $4, finished_at = $5,
                 next_attempt_at = $6, owner = NULL, lease_until = NULL
             WHERE id = $1 AND owner = $7 AND status = 'running'",
        )
        .bind(run.id.as_uuid())
        .bind(run.status.as_str())
        .bind(
            run.response_status
                .map(|status| i16::try_from(status).unwrap_or(0)),
        )
        .bind(&run.detail)
        .bind(run.finished_at)
        .bind(run.next_attempt_at)
        .bind(owner)
        .execute(&self.pool)
        .await
        .map_err(map_error)?;
        Ok(result.rows_affected() == 1)
    }

    async fn reap_expired(&self, now: DateTime<Utc>) -> Result<Vec<JobRun>, RepositoryError> {
        let rows = sqlx::query_as::<_, RunRow>(&format!(
            "UPDATE job_runs
             SET status = 'failed', detail = 'lease expired', finished_at = $1,
                 next_attempt_at = $1, owner = NULL, lease_until = NULL
             WHERE status = 'running' AND lease_until <= $1
             RETURNING {RUN_COLUMNS}"
        ))
        .bind(now)
        .fetch_all(&self.pool)
        .await
        .map_err(map_error)?;
        rows.into_iter().map(RunRow::into_domain).collect()
    }

    async fn due_retries(&self, now: DateTime<Utc>) -> Result<Vec<JobRun>, RepositoryError> {
        let rows = sqlx::query_as::<_, RunRow>(&format!(
            "SELECT {RUN_COLUMNS} FROM job_runs
             WHERE status = 'failed' AND next_attempt_at IS NOT NULL AND next_attempt_at <= $1
             ORDER BY next_attempt_at ASC LIMIT 200"
        ))
        .bind(now)
        .fetch_all(&self.pool)
        .await
        .map_err(map_error)?;
        rows.into_iter().map(RunRow::into_domain).collect()
    }

    async fn list_for_job(
        &self,
        job: &str,
        from: Option<DateTime<Utc>>,
        to: Option<DateTime<Utc>>,
        limit: i64,
    ) -> Result<Vec<JobRun>, RepositoryError> {
        let rows = sqlx::query_as::<_, RunRow>(&format!(
            "SELECT {RUN_COLUMNS} FROM job_runs
             WHERE job = $1
               AND ($2::timestamptz IS NULL OR scheduled_for >= $2::timestamptz)
               AND ($3::timestamptz IS NULL OR scheduled_for <= $3::timestamptz)
             ORDER BY scheduled_for DESC, attempt DESC LIMIT $4"
        ))
        .bind(job)
        .bind(from)
        .bind(to)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(map_error)?;
        rows.into_iter().map(RunRow::into_domain).collect()
    }

    async fn latest_per_job(&self) -> Result<Vec<JobRun>, RepositoryError> {
        let rows = sqlx::query_as::<_, RunRow>(&format!(
            "SELECT DISTINCT ON (job) {RUN_COLUMNS} FROM job_runs
             ORDER BY job, scheduled_for DESC, attempt DESC"
        ))
        .fetch_all(&self.pool)
        .await
        .map_err(map_error)?;
        rows.into_iter().map(RunRow::into_domain).collect()
    }

    async fn set_watermark(&self, job: &str, window: DateTime<Utc>) -> Result<(), RepositoryError> {
        sqlx::query(
            "UPDATE job_definitions
             SET last_scheduled_for = $2
             WHERE name = $1 AND (last_scheduled_for IS NULL OR last_scheduled_for < $2)",
        )
        .bind(job)
        .bind(window)
        .execute(&self.pool)
        .await
        .map_err(map_error)?;
        Ok(())
    }

    async fn prune_before(&self, cutoff: DateTime<Utc>) -> Result<u64, RepositoryError> {
        let result = sqlx::query(
            "DELETE FROM job_runs WHERE id IN (
                 SELECT id FROM job_runs WHERE scheduled_for < $1 LIMIT 50000
             )",
        )
        .bind(cutoff)
        .execute(&self.pool)
        .await
        .map_err(map_error)?;
        Ok(result.rows_affected())
    }
}

// ---------------------------------------------------------------------------
// Lease and heartbeat
// ---------------------------------------------------------------------------

/// sqlx-backed orchestrator lease.
#[derive(Debug, Clone)]
pub struct PgLeaseRepository {
    pool: PgPool,
}

impl PgLeaseRepository {
    /// Creates the repository.
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl LeaseRepository for PgLeaseRepository {
    async fn claim_or_renew(
        &self,
        owner: &str,
        lease_until: DateTime<Utc>,
        now: DateTime<Utc>,
    ) -> Result<bool, RepositoryError> {
        let result = sqlx::query(
            "UPDATE orchestrator_lease
             SET owner = $1, lease_until = $2
             WHERE name = 'job-orchestrator'
               AND (owner = $1 OR lease_until IS NULL OR lease_until <= $3)",
        )
        .bind(owner)
        .bind(lease_until)
        .bind(now)
        .execute(&self.pool)
        .await
        .map_err(map_error)?;
        Ok(result.rows_affected() == 1)
    }

    async fn owned(&self, owner: &str, now: DateTime<Utc>) -> Result<bool, RepositoryError> {
        sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS (
                 SELECT 1 FROM orchestrator_lease
                 WHERE name = 'job-orchestrator' AND owner = $1 AND lease_until > $2
             )",
        )
        .bind(owner)
        .bind(now)
        .fetch_one(&self.pool)
        .await
        .map_err(map_error)
    }
}

/// sqlx-backed heartbeat.
#[derive(Debug, Clone)]
pub struct PgHeartbeatRepository {
    pool: PgPool,
}

impl PgHeartbeatRepository {
    /// Creates the repository.
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl HeartbeatRepository for PgHeartbeatRepository {
    async fn write(&self, owner: &str, now: DateTime<Utc>) -> Result<(), RepositoryError> {
        sqlx::query(
            "UPDATE heartbeat SET updated_at = $2, owner = $1
             WHERE name = 'job-orchestrator'",
        )
        .bind(owner)
        .bind(now)
        .execute(&self.pool)
        .await
        .map_err(map_error)?;
        Ok(())
    }

    async fn age_seconds(&self, now: DateTime<Utc>) -> Result<Option<i64>, RepositoryError> {
        let updated: Option<DateTime<Utc>> =
            sqlx::query_scalar("SELECT updated_at FROM heartbeat WHERE name = 'job-orchestrator'")
                .fetch_optional(&self.pool)
                .await
                .map_err(map_error)?;
        Ok(updated.map(|updated| (now - updated).num_seconds().max(0)))
    }
}

// ---------------------------------------------------------------------------
// Provider client
// ---------------------------------------------------------------------------

/// The C11 provider client.
pub struct HttpTargetClient {
    client: reqwest::Client,
    service_urls: BTreeMap<String, String>,
    token: String,
}

impl HttpTargetClient {
    /// Creates the client.
    #[must_use]
    pub fn new(service_urls: BTreeMap<String, String>, token: impl Into<String>) -> Self {
        Self {
            client: reqwest::Client::builder()
                .connect_timeout(StdDuration::from_secs(1))
                .build()
                .unwrap_or_else(|_| reqwest::Client::new()),
            service_urls,
            token: token.into(),
        }
    }
}

#[derive(Deserialize)]
struct TargetBody {
    #[serde(default)]
    status: Option<String>,
    #[serde(default)]
    #[serde(rename = "runId")]
    run_id: Option<String>,
}

#[async_trait]
impl TargetClient for HttpTargetClient {
    async fn run(
        &self,
        provider: &str,
        job: &str,
        _scheduled_for: DateTime<Utc>,
        owner: &str,
        idempotency_key: &str,
        timeout_seconds: u64,
    ) -> Result<TargetAnswer, ExternalError> {
        let Some(base) = self.service_urls.get(provider) else {
            return Err(ExternalError::Unavailable(format!(
                "no URL is configured for provider {provider}"
            )));
        };
        let url = format!("{}/v1/internal/run/{job}", base.trim_end_matches('/'));
        let response = self
            .client
            .post(url)
            .bearer_auth(&self.token)
            .header("idempotency-key", idempotency_key)
            .json(&serde_json::json!({ "owner": owner }))
            .timeout(StdDuration::from_secs(timeout_seconds))
            .send()
            .await
            .map_err(|error| ExternalError::Unavailable(error.to_string()))?;
        let status = response.status().as_u16();
        match status {
            200 | 204 => {
                let body = response.json::<TargetBody>().await.ok();
                match body.and_then(|body| body.status) {
                    Some(label) if label == "failed" => Ok(TargetAnswer::Failed),
                    Some(label) if label == "skipped" => Ok(TargetAnswer::Skipped),
                    _ => Ok(TargetAnswer::Succeeded),
                }
            }
            202 => Ok(TargetAnswer::AlreadyRunning),
            409 => Ok(TargetAnswer::LeaseHeld),
            400..=499 => Ok(TargetAnswer::Refused(status)),
            _ => Ok(TargetAnswer::Unavailable),
        }
    }
}

/// Exposed for a focused test.
#[must_use]
pub fn target_run_id(body: &str) -> Option<String> {
    serde_json::from_str::<TargetBody>(body)
        .ok()
        .and_then(|body| body.run_id)
}

// ---------------------------------------------------------------------------
// Audit client
// ---------------------------------------------------------------------------

/// Appends C10 events.
pub struct HttpAuditClient {
    client: reqwest::Client,
    base_url: String,
    token: String,
}

impl HttpAuditClient {
    /// Creates the client.
    #[must_use]
    pub fn new(base_url: impl Into<String>, token: impl Into<String>) -> Self {
        Self {
            client: reqwest::Client::builder()
                .connect_timeout(StdDuration::from_secs(1))
                .timeout(StdDuration::from_secs(5))
                .build()
                .unwrap_or_else(|_| reqwest::Client::new()),
            base_url: base_url.into(),
            token: token.into(),
        }
    }
}

#[async_trait]
impl AuditClient for HttpAuditClient {
    async fn append(&self, event: &AuditEvent) -> Result<(), RepositoryError> {
        let url = format!("{}/v1/events", self.base_url.trim_end_matches('/'));
        let body = serde_json::json!({
            "actorId": event.actor_id.map(|actor| actor.to_string()),
            "actorKind": if event.actor_id.is_some() { "user" } else { "system" },
            "action": event.action,
            "target": event.target,
            "occurredAt": event.occurred_at,
            "context": {},
            "outcome": event.outcome,
            "sensitivity": "routine",
        });
        let response = self
            .client
            .post(url)
            .bearer_auth(&self.token)
            .json(&body)
            .send()
            .await
            .map_err(|error| RepositoryError::Unavailable(error.to_string()))?;
        if response.status().is_success() {
            Ok(())
        } else {
            Err(RepositoryError::Unavailable(format!(
                "the audit service answered {}",
                response.status()
            )))
        }
    }
}

// ---------------------------------------------------------------------------
// JWT validation
// ---------------------------------------------------------------------------

/// Validates Supabase JWTs.
pub struct JwksValidator {
    client: reqwest::Client,
    jwks_url: String,
    issuer: String,
    audience: String,
}

impl JwksValidator {
    /// Creates the validator.
    #[must_use]
    pub fn new(
        url: impl Into<String>,
        issuer: impl Into<String>,
        audience: impl Into<String>,
    ) -> Self {
        Self {
            client: reqwest::Client::builder()
                .timeout(StdDuration::from_secs(5))
                .build()
                .unwrap_or_else(|_| reqwest::Client::new()),
            jwks_url: url.into(),
            issuer: issuer.into(),
            audience: audience.into(),
        }
    }
}

#[derive(Deserialize)]
struct Jwks {
    keys: Vec<Jwk>,
}

#[derive(Deserialize)]
struct Jwk {
    kid: Option<String>,
    kty: String,
    n: Option<String>,
    e: Option<String>,
}

#[derive(Deserialize)]
struct Claims {
    sub: String,
    #[serde(default)]
    app_metadata: serde_json::Value,
}

#[async_trait]
impl TokenValidator for JwksValidator {
    async fn validate(&self, token: &str) -> Result<VerifiedClaims, AuthError> {
        let token = token.trim();
        if token.is_empty() {
            return Err(AuthError::Missing);
        }
        let header = jsonwebtoken::decode_header(token)
            .map_err(|error| AuthError::Invalid(error.to_string()))?;
        let jwks = self
            .client
            .get(&self.jwks_url)
            .send()
            .await
            .map_err(|error| AuthError::Unavailable(error.to_string()))?
            .json::<Jwks>()
            .await
            .map_err(|error| AuthError::Unavailable(error.to_string()))?;
        let key = jwks
            .keys
            .iter()
            .filter(|key| key.kty == "RSA")
            .find(|key| match (&header.kid, &key.kid) {
                (Some(kid), Some(candidate)) => kid == candidate,
                (None, _) => true,
                _ => false,
            })
            .ok_or_else(|| AuthError::Invalid("no matching signing key".to_string()))?;
        let decoding = jsonwebtoken::DecodingKey::from_rsa_components(
            key.n.as_deref().unwrap_or_default(),
            key.e.as_deref().unwrap_or_default(),
        )
        .map_err(|error| AuthError::Invalid(error.to_string()))?;
        let mut validation = jsonwebtoken::Validation::new(jsonwebtoken::Algorithm::RS256);
        validation.set_issuer(&[self.issuer.as_str()]);
        validation.set_audience(&[self.audience.as_str()]);
        validation.leeway = 30;
        let data = jsonwebtoken::decode::<Claims>(token, &decoding, &validation)
            .map_err(|error| AuthError::Invalid(error.to_string()))?;
        let user_id = UserId::parse(&data.claims.sub)
            .map_err(|_| AuthError::Invalid("the subject is not a user id".to_string()))?;
        let role = data
            .claims
            .app_metadata
            .get("role")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("user")
            .to_string();
        Ok(VerifiedClaims { user_id, role })
    }
}

/// The role read from a claims JSON document, ignoring `user_metadata` and the
/// top-level `role` claim. Exposed for a focused test.
#[must_use]
pub fn role_from_claims(claims: &serde_json::Value) -> String {
    claims
        .get("app_metadata")
        .and_then(|metadata| metadata.get("role"))
        .and_then(serde_json::Value::as_str)
        .unwrap_or("user")
        .to_string()
}

// ---------------------------------------------------------------------------
// Service-token registry
// ---------------------------------------------------------------------------

/// Hashes a token with SHA-256.
#[must_use]
pub fn hash_token(token: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(token.trim().as_bytes());
    hex::encode(hasher.finalize())
}

/// Resolves inbound tokens from the registry table.
#[derive(Debug, Clone)]
pub struct PgServiceTokenRegistry {
    pool: PgPool,
}

impl PgServiceTokenRegistry {
    /// Creates the registry.
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl ServiceTokenRegistry for PgServiceTokenRegistry {
    async fn service_for(&self, token: &str) -> Result<Option<RegisteredService>, RepositoryError> {
        let row: Option<(String, Vec<String>)> = sqlx::query_as(
            "SELECT caller, operations FROM service_token_registry
             WHERE token_hash = $1 AND active",
        )
        .bind(hash_token(token))
        .fetch_optional(&self.pool)
        .await
        .map_err(map_error)?;
        let Some((caller, operations)) = row else {
            return Ok(None);
        };
        let operations = operations
            .into_iter()
            .filter_map(|label| match label.as_str() {
                "smoke" => Some(Operation::Smoke),
                _ => None,
            })
            .collect();
        Ok(Some(RegisteredService {
            name: caller,
            operations,
        }))
    }
}

// ---------------------------------------------------------------------------
// Readiness, clock, ids
// ---------------------------------------------------------------------------

/// Checks the service's own database.
#[derive(Debug, Clone)]
pub struct PgReadinessProbe {
    pool: PgPool,
}

impl PgReadinessProbe {
    /// Creates the probe.
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl ReadinessProbe for PgReadinessProbe {
    async fn check(&self) -> Result<(), String> {
        sqlx::query_scalar::<_, i32>("SELECT 1")
            .fetch_one(&self.pool)
            .await
            .map_err(|error| error.to_string())?;
        let applied: i64 =
            sqlx::query_scalar("SELECT count(*) FROM _sqlx_migrations WHERE success")
                .fetch_one(&self.pool)
                .await
                .map_err(|error| error.to_string())?;
        if applied == 0 {
            return Err("no migration has been applied".to_string());
        }
        Ok(())
    }
}

/// The production clock.
#[derive(Debug, Default, Clone, Copy)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> DateTime<Utc> {
        Utc::now()
    }
}

/// Generates UUIDv4 identifiers.
#[derive(Debug, Default, Clone, Copy)]
pub struct UuidGenerator;

impl IdGenerator for UuidGenerator {
    fn next(&self) -> Uuid {
        Uuid::new_v4()
    }
}

/// Coerces a pool into the shared trait object used by the service crate.
#[must_use]
pub fn shared<T>(value: T) -> Arc<T> {
    Arc::new(value)
}

#[cfg(test)]
mod tests {
    use super::{hash_token, role_from_claims, target_run_id};

    #[test]
    fn the_role_comes_only_from_app_metadata() {
        let claims = serde_json::json!({
            "role": "authenticated",
            "user_metadata": { "role": "admin" },
            "app_metadata": { "role": "support" }
        });
        assert_eq!(role_from_claims(&claims), "support");
    }

    #[test]
    fn a_user_writable_role_is_not_authority() {
        let claims = serde_json::json!({
            "role": "admin",
            "user_metadata": { "role": "admin" }
        });
        assert_eq!(role_from_claims(&claims), "user");
    }

    #[test]
    fn a_202_body_carries_a_run_id() {
        assert_eq!(target_run_id(r#"{"runId":"abc"}"#), Some("abc".to_string()));
        assert_eq!(target_run_id("{}"), None);
    }

    #[test]
    fn a_token_hash_is_sha256_hex() {
        assert_eq!(hash_token("abc").len(), 64);
        assert_eq!(hash_token("abc"), hash_token(" abc "));
    }
}
