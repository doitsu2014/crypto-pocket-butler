//! U6 — the runtime jurisdiction gate (BR1.1–BR1.10).
//!
//! Two use cases sit behind inward-defined ports:
//!
//! - [`CheckJurisdiction`] evaluates a self-declared country and an IP-resolved
//!   country against the active list version and records the decision as
//!   evidence (C8, C9).
//! - [`PublishList`] validates and stores a new immutable list version (W3).
//!
//! Both fail closed. A provider, list, or audit failure is a refusal with a
//! retry path, never an allow (BR1.5, BR1.6, BR1.7).

use std::collections::BTreeMap;
use std::net::IpAddr;
use std::str::FromStr;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use async_trait::async_trait;
use chrono::{DateTime, Duration as ChronoDuration, Utc};
use identity_domain::{
    decide, AuditOutcome, AuditRecord, CountryCode, JurisdictionListEntry, JurisdictionListVersion,
    JurisdictionStatus,
};
use uuid::Uuid;

use crate::common::{AppError, FieldError};
use crate::ports::{Clock, ExternalError, GateRefusal, JurisdictionGate, RepositoryError};

/// Consecutive provider failures that open the circuit (reliability design).
const CIRCUIT_FAILURE_THRESHOLD: u32 = 5;
/// How long an open circuit refuses before one half-open probe is allowed.
const CIRCUIT_OPEN_SECONDS: i64 = 30;

/// The provider and audit latency budgets (performance design NFR2.2–NFR2.4).
#[derive(Debug, Clone, Copy)]
pub struct ProviderBudget {
    /// One provider attempt.
    pub attempt: Duration,
    /// How many times a transient provider failure is retried.
    pub retries: u32,
    /// The cap on the whole provider call, retries included.
    pub total: Duration,
    /// The cap on one audit write attempt.
    pub audit_total: Duration,
}

impl Default for ProviderBudget {
    fn default() -> Self {
        Self {
            attempt: Duration::from_millis(800),
            retries: 1,
            total: Duration::from_millis(1500),
            audit_total: Duration::from_millis(300),
        }
    }
}

/// Resolves an IP address to its country. The adapter owns the provider contract;
/// the use case never sees a provider payload (BR1.3).
#[async_trait]
pub trait GeoLocator: Send + Sync {
    /// Returns the country the provider resolves for `ip`.
    ///
    /// # Errors
    /// Returns [`ExternalError`] when the provider is unreachable, times out, or
    /// answers with a value that is not a country code.
    async fn country_for(&self, ip: &str) -> Result<CountryCode, ExternalError>;
}

/// Reads the active jurisdiction list and stores a new immutable version (W3).
#[async_trait]
pub trait ListRepository: Send + Sync {
    /// Returns the active version, or `None` when no version is published.
    ///
    /// # Errors
    /// Returns [`RepositoryError::InvalidData`] when the stored rows no longer
    /// match their checksum or entry count (BR1.6).
    async fn active(&self) -> Result<Option<JurisdictionListVersion>, RepositoryError>;

    /// Stores `list` as the new active version and supersedes the previous one in
    /// one transaction.
    async fn publish(&self, list: &JurisdictionListVersion) -> Result<(), RepositoryError>;
}

/// Writes one decision's evidence through C9 with an idempotency key, so a retry
/// appends once (BR1.7, NFR2.4).
#[async_trait]
pub trait AuditSink: Send + Sync {
    /// Appends one event.
    ///
    /// # Errors
    /// Returns [`RepositoryError`] when the audit service refuses or is
    /// unreachable.
    async fn record(
        &self,
        record: &AuditRecord,
        correlation_id: &str,
        idempotency_key: &str,
    ) -> Result<(), RepositoryError>;
}

/// The circuit breaker around the geolocation provider (reliability design).
#[derive(Debug, Default)]
struct CircuitState {
    consecutive_failures: u32,
    open_until: Option<DateTime<Utc>>,
    half_open: bool,
}

#[derive(Debug, Default)]
struct ProviderCircuit {
    state: Mutex<CircuitState>,
}

impl ProviderCircuit {
    /// Whether a provider call may proceed at `now`.
    fn allow(&self, now: DateTime<Utc>) -> bool {
        let mut state = self
            .state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        match state.open_until {
            None => true,
            Some(until) if now >= until => {
                state.half_open = true;
                state.open_until = None;
                true
            }
            Some(_) => false,
        }
    }

    /// Records one successful provider call.
    fn success(&self) {
        let mut state = self
            .state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        state.consecutive_failures = 0;
        state.open_until = None;
        state.half_open = false;
    }

    /// Records one failed provider call, opening the circuit at the threshold.
    fn failure(&self, now: DateTime<Utc>) {
        let mut state = self
            .state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if state.half_open {
            state.half_open = false;
            state.open_until = Some(now + ChronoDuration::seconds(CIRCUIT_OPEN_SECONDS));
            return;
        }
        state.consecutive_failures += 1;
        if state.consecutive_failures >= CIRCUIT_FAILURE_THRESHOLD {
            state.consecutive_failures = 0;
            state.open_until = Some(now + ChronoDuration::seconds(CIRCUIT_OPEN_SECONDS));
        }
    }
}

/// The gate's answer plus the id of the evidence record that backs it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckResult {
    decision: identity_domain::Decision,
    evidence_id: String,
}

impl CheckResult {
    /// The allowed or restricted decision.
    #[must_use]
    pub fn decision(&self) -> &identity_domain::Decision {
        &self.decision
    }

    /// The decision id, also used as the audit idempotency key (BR1.10).
    #[must_use]
    pub fn evidence_id(&self) -> &str {
        &self.evidence_id
    }

    /// Consumes the result into its two parts.
    #[must_use]
    pub fn into_parts(self) -> (identity_domain::Decision, String) {
        (self.decision, self.evidence_id)
    }
}

/// Evaluates a self-declared country and an IP-resolved country against the
/// active list version (BR1.1, C8).
pub struct CheckJurisdiction {
    lists: Arc<dyn ListRepository>,
    locator: Arc<dyn GeoLocator>,
    audit: Arc<dyn AuditSink>,
    clock: Arc<dyn Clock>,
    circuit: ProviderCircuit,
    budget: ProviderBudget,
}

impl CheckJurisdiction {
    /// Creates the use case with the production latency budget.
    #[must_use]
    pub fn new(
        lists: Arc<dyn ListRepository>,
        locator: Arc<dyn GeoLocator>,
        audit: Arc<dyn AuditSink>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self {
            lists,
            locator,
            audit,
            clock,
            circuit: ProviderCircuit::default(),
            budget: ProviderBudget::default(),
        }
    }

    /// Replaces the latency budget. Tests use a smaller budget; production keeps
    /// the default.
    #[must_use]
    pub fn with_budget(mut self, budget: ProviderBudget) -> Self {
        self.budget = budget;
        self
    }

    /// Runs one check.
    ///
    /// # Errors
    /// Returns [`AppError::Validation`] for an invalid country code or IP,
    /// [`AppError::Unavailable`] when the list, provider, or audit sink cannot
    /// serve the decision, and [`AppError::from_repository`] for a storage fault.
    pub async fn check(
        &self,
        country_code: &str,
        ip: &str,
        correlation_id: &str,
    ) -> Result<CheckResult, AppError> {
        let declared = CountryCode::parse(country_code).map_err(AppError::from_domain)?;
        let parsed_ip = IpAddr::from_str(ip.trim()).map_err(|_| {
            AppError::Validation(vec![FieldError::new(
                "ip",
                "must be an IPv4 or IPv6 address",
            )])
        })?;

        let list = self.active_list().await?;
        let ip_country = self.locate(&parsed_ip.to_string()).await?;

        let decision = decide(
            list.status_for(&declared),
            list.status_for(&ip_country),
            list.version(),
        );
        let evidence_id = Uuid::new_v4().to_string();
        self.write_evidence(
            &decision,
            &declared,
            &ip_country,
            correlation_id,
            &evidence_id,
        )
        .await?;

        Ok(CheckResult {
            decision,
            evidence_id,
        })
    }

    async fn active_list(&self) -> Result<JurisdictionListVersion, AppError> {
        match self.lists.active().await {
            Ok(Some(list)) => Ok(list),
            Ok(None) => Err(AppError::Unavailable(
                "no jurisdiction list version is published".to_string(),
            )),
            Err(RepositoryError::InvalidData(detail)) => Err(AppError::Unavailable(detail)),
            Err(error) => Err(AppError::from_repository(error)),
        }
    }

    async fn locate(&self, ip: &str) -> Result<CountryCode, AppError> {
        let now = self.clock.now();
        if !self.circuit.allow(now) {
            return Err(AppError::Unavailable(
                "the geolocation circuit is open".to_string(),
            ));
        }

        let deadline = tokio::time::Instant::now() + self.budget.total;
        let mut attempt = 0u32;
        loop {
            let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
            if remaining.is_zero() {
                self.circuit.failure(self.clock.now());
                return Err(AppError::Unavailable(
                    "the geolocation provider exceeded its budget".to_string(),
                ));
            }
            let attempt_budget = remaining.min(self.budget.attempt);
            match tokio::time::timeout(attempt_budget, self.locator.country_for(ip)).await {
                Ok(Ok(country)) => {
                    self.circuit.success();
                    return Ok(country);
                }
                Ok(Err(error)) => {
                    let retryable = matches!(error, ExternalError::Unavailable(_));
                    if retryable && attempt < self.budget.retries {
                        attempt += 1;
                        continue;
                    }
                    self.circuit.failure(self.clock.now());
                    return Err(AppError::from_external(error));
                }
                Err(_) => {
                    if attempt < self.budget.retries {
                        attempt += 1;
                        continue;
                    }
                    self.circuit.failure(self.clock.now());
                    return Err(AppError::Unavailable(
                        "the geolocation provider timed out".to_string(),
                    ));
                }
            }
        }
    }

    async fn write_evidence(
        &self,
        decision: &identity_domain::Decision,
        declared: &CountryCode,
        ip_country: &CountryCode,
        correlation_id: &str,
        evidence_id: &str,
    ) -> Result<(), AppError> {
        let outcome = if decision.is_allowed() {
            AuditOutcome::Success
        } else {
            AuditOutcome::Denied
        };
        let restricted_by = decision
            .restricted_by()
            .map_or("none", |source| source.as_str());
        let context = BTreeMap::from([
            ("countryCode".to_string(), declared.as_str().to_string()),
            ("ipCountry".to_string(), ip_country.as_str().to_string()),
            (
                "listVersion".to_string(),
                decision.list_version().to_string(),
            ),
            ("restrictedBy".to_string(), restricted_by.to_string()),
        ]);
        let record = AuditRecord::new(
            None,
            "identity.jurisdiction.check",
            declared.as_str(),
            outcome,
            self.clock.now(),
            context,
        )
        .map_err(AppError::from_domain)?;

        write_evidence(
            self.audit.as_ref(),
            &record,
            correlation_id,
            evidence_id,
            self.budget.audit_total,
        )
        .await
    }
}

/// The registry calls the same gate in-process before it writes a user record.
#[async_trait]
impl JurisdictionGate for CheckJurisdiction {
    async fn clear(
        &self,
        country_code: &str,
        ip: &str,
        correlation_id: &str,
    ) -> Result<(), GateRefusal> {
        match self.check(country_code, ip, correlation_id).await {
            Ok(result) if result.decision().is_allowed() => Ok(()),
            Ok(result) => Err(GateRefusal::Denied(format!(
                "the jurisdiction list version '{}' restricts this provider",
                result.decision().list_version()
            ))),
            Err(AppError::Validation(errors)) => Err(GateRefusal::Invalid(
                errors
                    .iter()
                    .map(|error| format!("{}: {}", error.field, error.reason))
                    .collect::<Vec<_>>()
                    .join("; "),
            )),
            Err(error) => Err(GateRefusal::Unavailable(error.to_string())),
        }
    }
}

/// One country supplied to a publish.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PublishEntry {
    /// The country code as supplied by Compliance.
    pub country_code: String,
    /// `allowed` or `restricted`.
    pub status: String,
    /// Who maintains the entry.
    pub source: String,
}

/// A request to publish a complete list version.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PublishRequest {
    /// The monotonic version label.
    pub version: String,
    /// The complete set of country rules.
    pub entries: Vec<PublishEntry>,
    /// A caller-computed digest to verify against the entries, when supplied.
    pub expected_checksum: Option<String>,
}

/// What a successful publish returns.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PublishedList {
    version: String,
    entry_count: usize,
    checksum: String,
}

impl PublishedList {
    /// The published version label.
    #[must_use]
    pub fn version(&self) -> &str {
        &self.version
    }

    /// How many countries the version names.
    #[must_use]
    pub fn entry_count(&self) -> usize {
        self.entry_count
    }

    /// The digest over the ordered entries.
    #[must_use]
    pub fn checksum(&self) -> &str {
        &self.checksum
    }
}

/// Validates and stores a new immutable list version (W3).
pub struct PublishList {
    lists: Arc<dyn ListRepository>,
    audit: Arc<dyn AuditSink>,
    clock: Arc<dyn Clock>,
    budget: ProviderBudget,
}

impl PublishList {
    /// Creates the use case with the production latency budget.
    #[must_use]
    pub fn new(
        lists: Arc<dyn ListRepository>,
        audit: Arc<dyn AuditSink>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self {
            lists,
            audit,
            clock,
            budget: ProviderBudget::default(),
        }
    }

    /// Runs one publish.
    ///
    /// # Errors
    /// Returns [`AppError::Validation`] for an invalid country code, status,
    /// empty list, duplicate country, or mismatched checksum, and
    /// [`AppError::Unavailable`] when the store or the audit sink cannot serve
    /// the publish.
    pub async fn publish(
        &self,
        request: PublishRequest,
        correlation_id: &str,
    ) -> Result<PublishedList, AppError> {
        let now = self.clock.now();
        let entries = request
            .entries
            .into_iter()
            .map(|entry| {
                Ok(JurisdictionListEntry {
                    code: CountryCode::parse(&entry.country_code).map_err(AppError::from_domain)?,
                    status: JurisdictionStatus::parse(&entry.status)
                        .map_err(AppError::from_domain)?,
                    source: entry.source,
                    updated_at: now,
                })
            })
            .collect::<Result<Vec<_>, AppError>>()?;
        let list = JurisdictionListVersion::new(request.version, now, entries)
            .map_err(AppError::from_domain)?;

        if let Some(expected) = request.expected_checksum {
            if expected.trim() != list.checksum() {
                return Err(AppError::Validation(vec![FieldError::new(
                    "checksum",
                    "does not match the entries supplied",
                )]));
            }
        }

        self.lists
            .publish(&list)
            .await
            .map_err(AppError::from_repository)?;

        let context = BTreeMap::from([
            ("entryCount".to_string(), list.entry_count().to_string()),
            ("checksum".to_string(), list.checksum().to_string()),
        ]);
        let record = AuditRecord::new(
            None,
            "identity.jurisdiction.publish",
            list.version(),
            AuditOutcome::Success,
            now,
            context,
        )
        .map_err(AppError::from_domain)?;
        write_evidence(
            self.audit.as_ref(),
            &record,
            correlation_id,
            list.checksum(),
            self.budget.audit_total,
        )
        .await?;

        Ok(PublishedList {
            version: list.version().to_string(),
            entry_count: list.entry_count(),
            checksum: list.checksum().to_string(),
        })
    }
}

/// Writes evidence with one retry on a transient failure, reusing the same
/// idempotency key so a retry appends once (NFR2.4).
async fn write_evidence(
    audit: &dyn AuditSink,
    record: &AuditRecord,
    correlation_id: &str,
    idempotency_key: &str,
    budget: Duration,
) -> Result<(), AppError> {
    match tokio::time::timeout(
        budget,
        audit.record(record, correlation_id, idempotency_key),
    )
    .await
    {
        Ok(Ok(())) => return Ok(()),
        Ok(Err(error)) if !matches!(error, RepositoryError::Unavailable(_)) => {
            return Err(AppError::from_repository(error));
        }
        Ok(Err(_)) | Err(_) => {}
    }

    match tokio::time::timeout(
        budget,
        audit.record(record, correlation_id, idempotency_key),
    )
    .await
    {
        Ok(Ok(())) => Ok(()),
        Ok(Err(error)) => Err(AppError::from_repository(error)),
        Err(_) => Err(AppError::Unavailable(
            "the audit write exceeded its budget".to_string(),
        )),
    }
}
