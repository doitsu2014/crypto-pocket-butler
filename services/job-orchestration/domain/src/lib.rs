//! Job Orchestration domain: the schedule grammar, the job catalogue, run records
//! and their state machine, and the internal service allow-list.
//!
//! Entities, value types and business rules only. No framework, no sibling crate.

use chrono::{DateTime, Duration, NaiveDate, NaiveTime, TimeZone, Utc};
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Errors and identifiers
// ---------------------------------------------------------------------------

/// A business-rule violation, expressed without transport or storage detail.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum DomainError {
    /// A required field was absent or blank.
    #[error("{field} is required")]
    Required { field: &'static str },
    /// A field carried a value outside its allowed shape.
    #[error("{field}: {reason}")]
    Invalid { field: &'static str, reason: String },
    /// A state-machine transition was not allowed.
    #[error("{entity} cannot move from {from} to {to}")]
    InvalidTransition {
        entity: &'static str,
        from: &'static str,
        to: &'static str,
    },
    /// A rule refused the action.
    #[error("{reason}")]
    Denied { reason: String },
}

macro_rules! uuid_id {
    ($name:ident, $entity:literal) => {
        #[doc = concat!("Identifier for a ", $entity, ".")]
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
        pub struct $name(Uuid);
        impl $name {
            /// Wraps an existing UUID.
            #[must_use]
            pub fn new(value: Uuid) -> Self {
                Self(value)
            }
            /// Generates a random identifier.
            #[must_use]
            pub fn generate() -> Self {
                Self(Uuid::new_v4())
            }
            /// The underlying UUID.
            #[must_use]
            pub fn as_uuid(&self) -> Uuid {
                self.0
            }
            /// Parses a UUID string.
            ///
            /// # Errors
            /// Returns [`DomainError::Invalid`] when the text is not a UUID.
            pub fn parse(raw: &str) -> Result<Self, DomainError> {
                Uuid::parse_str(raw.trim())
                    .map(Self)
                    .map_err(|_| DomainError::Invalid {
                        field: $entity,
                        reason: "is not a UUID".to_string(),
                    })
            }
        }
        impl std::fmt::Display for $name {
            fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(formatter, "{}", self.0)
            }
        }
    };
}

uuid_id!(RunId, "run");
uuid_id!(UserId, "user");

// ---------------------------------------------------------------------------
// Schedule
// ---------------------------------------------------------------------------

/// The job's schedule.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Schedule {
    /// Every N minutes, anchored to the Unix epoch (1–60).
    Every { minutes: u32 },
    /// Every hour at the given UTC minute (0–59).
    Hourly { minute: u32 },
    /// Every day at the given UTC hour and minute.
    Daily { hour: u32, minute: u32 },
}

impl Schedule {
    /// Parses `every:<minutes>`, `hourly:<minute>`, or `daily:<HH:MM>`.
    ///
    /// # Errors
    /// Returns [`DomainError::Invalid`] for any other shape or an out-of-range
    /// value.
    pub fn parse(raw: &str) -> Result<Self, DomainError> {
        let invalid = |reason: &str| DomainError::Invalid {
            field: "schedule",
            reason: reason.to_string(),
        };
        let (kind, value) = raw
            .trim()
            .split_once(':')
            .ok_or_else(|| invalid("must be every:N, hourly:M, or daily:HH:MM"))?;
        match kind {
            "every" => {
                let minutes: u32 = value.parse().map_err(|_| invalid("every: needs minutes"))?;
                if (1..=60).contains(&minutes) {
                    Ok(Self::Every { minutes })
                } else {
                    Err(invalid("every: must be between 1 and 60 minutes"))
                }
            }
            "hourly" => {
                let minute: u32 = value
                    .parse()
                    .map_err(|_| invalid("hourly: needs a minute"))?;
                if minute < 60 {
                    Ok(Self::Hourly { minute })
                } else {
                    Err(invalid("hourly: must be between 0 and 59"))
                }
            }
            "daily" => {
                let (hour, minute) = value
                    .split_once(':')
                    .ok_or_else(|| invalid("daily: needs HH:MM"))?;
                let hour: u32 = hour.parse().map_err(|_| invalid("daily: needs HH:MM"))?;
                let minute: u32 = minute.parse().map_err(|_| invalid("daily: needs HH:MM"))?;
                if hour < 24 && minute < 60 {
                    Ok(Self::Daily { hour, minute })
                } else {
                    Err(invalid("daily: is not a valid UTC time"))
                }
            }
            _ => Err(invalid("must be every:N, hourly:M, or daily:HH:MM")),
        }
    }

    /// The most recent boundary at or before `now`, as whole-second UTC.
    ///
    /// `Every:N` is anchored to the Unix epoch; `hourly` to the hour; `daily` to
    /// the given UTC time. The second is always truncated to zero, so two
    /// computations of one window produce the same canonical text (BR1.4).
    #[must_use]
    pub fn window(self, now: DateTime<Utc>) -> DateTime<Utc> {
        let epoch = now.timestamp();
        match self {
            Self::Every { minutes } => {
                let step = i64::from(minutes) * 60;
                let floored = epoch - epoch.rem_euclid(step);
                DateTime::from_timestamp(floored, 0).unwrap_or(now)
            }
            Self::Hourly { minute } => {
                let hour_start = epoch - epoch.rem_euclid(3_600);
                let candidate =
                    DateTime::from_timestamp(hour_start + i64::from(minute) * 60, 0).unwrap_or(now);
                if candidate <= now {
                    candidate
                } else {
                    candidate - Duration::hours(1)
                }
            }
            Self::Daily { hour, minute } => {
                let date: NaiveDate = now.date_naive();
                let time = NaiveTime::from_hms_opt(hour, minute, 0).unwrap_or_default();
                let candidate = Utc.from_utc_datetime(&date.and_time(time));
                if candidate <= now {
                    candidate
                } else {
                    candidate - Duration::days(1)
                }
            }
        }
    }

    /// The window before `window`.
    #[must_use]
    pub fn previous_window(self, window: DateTime<Utc>) -> DateTime<Utc> {
        match self {
            Self::Every { minutes } => window - Duration::minutes(i64::from(minutes)),
            Self::Hourly { .. } => window - Duration::hours(1),
            Self::Daily { .. } => window - Duration::days(1),
        }
    }

    /// The next window after `window`.
    #[must_use]
    pub fn next_window(self, window: DateTime<Utc>) -> DateTime<Utc> {
        match self {
            Self::Every { minutes } => window + Duration::minutes(i64::from(minutes)),
            Self::Hourly { .. } => window + Duration::hours(1),
            Self::Daily { .. } => window + Duration::days(1),
        }
    }

    /// The instant a failed attempt for `window` may be retried (BR2.1).
    #[must_use]
    pub fn retry_at(self, window: DateTime<Utc>) -> DateTime<Utc> {
        match self {
            Self::Every { minutes } if minutes < 10 => window + Duration::minutes(5),
            _ => self.next_window(window),
        }
    }
}

// ---------------------------------------------------------------------------
// Job catalogue and run records
// ---------------------------------------------------------------------------

/// The fixed job catalogue, each naming its owning provider (BR6.1).
pub const CATALOGUE: [(&str, &str); 14] = [
    ("account-sync", "account"),
    ("market-data-refresh", "market-data"),
    ("market-data-fx", "market-data"),
    ("portfolio-valuation", "portfolio"),
    ("portfolio-snapshot", "portfolio"),
    ("ledger-reconcile", "ledger"),
    ("ledger-ingest", "ledger"),
    ("analytics-aggregate", "analytics"),
    ("analytics-flag-expiry", "analytics"),
    ("notification-delivery", "notification"),
    ("notification-digest", "notification"),
    ("brief-generation", "reporting"),
    ("audit-privacy-sweep", "identity"),
    ("audit-retention", "audit"),
];

/// Whether `name` is in the catalogue (BR6.1).
#[must_use]
pub fn is_known_job(name: &str) -> bool {
    CATALOGUE.iter().any(|(job, _)| *job == name)
}

/// The provider service key for a job, when it is known.
#[must_use]
pub fn provider_for(name: &str) -> Option<&'static str> {
    CATALOGUE
        .iter()
        .find(|(job, _)| *job == name)
        .map(|(_, provider)| *provider)
}

/// The state of a run attempt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunStatus {
    /// Queued for the executor.
    Pending,
    /// A permit was acquired and the call is in flight.
    Running,
    /// The provider reported success.
    Succeeded,
    /// This attempt failed and may be retried.
    Failed,
    /// The provider was already running the window (202) or skipped it.
    Skipped,
    /// No attempts remain.
    Abandoned,
}

impl RunStatus {
    /// The stored label.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Running => "running",
            Self::Succeeded => "succeeded",
            Self::Failed => "failed",
            Self::Skipped => "skipped",
            Self::Abandoned => "abandoned",
        }
    }

    /// Parses a stored label.
    ///
    /// # Errors
    /// Returns [`DomainError::Invalid`] for an unknown status.
    pub fn parse(raw: &str) -> Result<Self, DomainError> {
        match raw.trim() {
            "pending" => Ok(Self::Pending),
            "running" => Ok(Self::Running),
            "succeeded" => Ok(Self::Succeeded),
            "failed" => Ok(Self::Failed),
            "skipped" => Ok(Self::Skipped),
            "abandoned" => Ok(Self::Abandoned),
            _ => Err(DomainError::Invalid {
                field: "status",
                reason: "is not a run status".to_string(),
            }),
        }
    }

    /// Whether no further transition may change the row.
    #[must_use]
    pub fn is_terminal(self) -> bool {
        matches!(self, Self::Succeeded | Self::Skipped | Self::Abandoned)
    }
}

/// Why the run exists (BR7.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Trigger {
    /// The scheduled window.
    Scheduled,
    /// A window recovered inside the lookback.
    Missed,
    /// An operator backfill.
    Backfill,
    /// An operator manual run.
    Manual,
}

impl Trigger {
    /// The stored label.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Scheduled => "scheduled",
            Self::Missed => "missed",
            Self::Backfill => "backfill",
            Self::Manual => "manual",
        }
    }

    /// Parses a stored label.
    ///
    /// # Errors
    /// Returns [`DomainError::Invalid`] for an unknown trigger.
    pub fn parse(raw: &str) -> Result<Self, DomainError> {
        match raw.trim() {
            "scheduled" => Ok(Self::Scheduled),
            "missed" => Ok(Self::Missed),
            "backfill" => Ok(Self::Backfill),
            "manual" => Ok(Self::Manual),
            _ => Err(DomainError::Invalid {
                field: "trigger",
                reason: "is not a run trigger".to_string(),
            }),
        }
    }
}

/// One attempt at one scheduled window (BR1.2, BR5.1).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JobRun {
    /// The attempt identifier.
    pub id: RunId,
    /// The job name.
    pub job: String,
    /// The canonical whole-second UTC window.
    pub scheduled_for: DateTime<Utc>,
    /// 1-based attempt number.
    pub attempt: u32,
    /// The state.
    pub status: RunStatus,
    /// Why the run exists.
    pub trigger: Trigger,
    /// The requesting operator, for backfill/manual.
    pub actor_id: Option<UserId>,
    /// The reason, for backfill/manual.
    pub reason: Option<String>,
    /// The instance holding the run lease.
    pub owner: Option<String>,
    /// The run lease expiry.
    pub lease_until: Option<DateTime<Utc>>,
    /// When a failed attempt may be retried.
    pub next_attempt_at: Option<DateTime<Utc>>,
    /// When the call began.
    pub started_at: Option<DateTime<Utc>>,
    /// When the row reached a terminal state.
    pub finished_at: Option<DateTime<Utc>>,
    /// The provider's HTTP status.
    pub response_status: Option<u16>,
    /// A short non-secret detail.
    pub detail: Option<String>,
}

/// The largest detail length kept.
pub const MAX_DETAIL_CHARS: usize = 500;

impl JobRun {
    /// Queues a new attempt.
    ///
    /// # Errors
    /// Returns [`DomainError`] for an unknown job, attempt 0, or a backfill/manual
    /// run without its reason.
    #[allow(clippy::too_many_arguments)]
    pub fn queued(
        id: RunId,
        job: &str,
        scheduled_for: DateTime<Utc>,
        attempt: u32,
        trigger: Trigger,
        actor_id: Option<UserId>,
        reason: Option<&str>,
    ) -> Result<Self, DomainError> {
        if !is_known_job(job) {
            return Err(DomainError::Invalid {
                field: "job",
                reason: "is not in the catalogue".to_string(),
            });
        }
        if attempt == 0 {
            return Err(DomainError::Invalid {
                field: "attempt",
                reason: "starts at 1".to_string(),
            });
        }
        let reason = reason.map(str::trim).filter(|reason| !reason.is_empty());
        if matches!(trigger, Trigger::Backfill | Trigger::Manual) && reason.is_none() {
            return Err(DomainError::Required { field: "reason" });
        }
        if let Some(reason) = reason {
            if reason.chars().count() > 200 {
                return Err(DomainError::Invalid {
                    field: "reason",
                    reason: "exceeds 200 characters".to_string(),
                });
            }
        }
        Ok(Self {
            id,
            job: job.to_string(),
            scheduled_for,
            attempt,
            status: RunStatus::Pending,
            trigger,
            actor_id,
            reason: reason.map(ToString::to_string),
            owner: None,
            lease_until: None,
            next_attempt_at: None,
            started_at: None,
            finished_at: None,
            response_status: None,
            detail: None,
        })
    }

    /// Moves a pending attempt to running for `owner` (BR1.3).
    ///
    /// # Errors
    /// Returns [`DomainError::InvalidTransition`] when the row is not pending.
    pub fn claim(
        &mut self,
        owner: &str,
        lease_until: DateTime<Utc>,
        started_at: DateTime<Utc>,
    ) -> Result<(), DomainError> {
        if self.status != RunStatus::Pending {
            return Err(DomainError::InvalidTransition {
                entity: "run",
                from: self.status.as_str(),
                to: RunStatus::Running.as_str(),
            });
        }
        if owner.trim().is_empty() {
            return Err(DomainError::Required { field: "owner" });
        }
        self.status = RunStatus::Running;
        self.owner = Some(owner.to_string());
        self.lease_until = Some(lease_until);
        self.started_at = Some(started_at);
        Ok(())
    }

    /// Records the provider's answer.
    ///
    /// # Errors
    /// Returns [`DomainError::InvalidTransition`] when the row is not running, and
    /// [`DomainError::Invalid`] for an over-long detail.
    pub fn finish(
        &mut self,
        status: RunStatus,
        response_status: Option<u16>,
        detail: Option<&str>,
        finished_at: DateTime<Utc>,
        next_attempt_at: Option<DateTime<Utc>>,
    ) -> Result<(), DomainError> {
        if self.status != RunStatus::Running {
            return Err(DomainError::InvalidTransition {
                entity: "run",
                from: self.status.as_str(),
                to: status.as_str(),
            });
        }
        if matches!(status, RunStatus::Pending | RunStatus::Running) {
            return Err(DomainError::InvalidTransition {
                entity: "run",
                from: self.status.as_str(),
                to: status.as_str(),
            });
        }
        let detail = detail.map(str::trim).filter(|detail| !detail.is_empty());
        if let Some(detail) = detail {
            if detail.chars().count() > MAX_DETAIL_CHARS {
                return Err(DomainError::Invalid {
                    field: "detail",
                    reason: "exceeds 500 characters".to_string(),
                });
            }
        }
        self.status = status;
        self.response_status = response_status;
        self.detail = detail.map(ToString::to_string);
        self.finished_at = if status.is_terminal() {
            Some(finished_at)
        } else {
            None
        };
        self.next_attempt_at = next_attempt_at;
        self.lease_until = None;
        self.owner = None;
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Service identities
// ---------------------------------------------------------------------------

/// The internal operations this service exposes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Operation {
    /// `GET /v1/internal/smoke`.
    Smoke,
}

impl Operation {
    /// The stored label.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Smoke => "smoke",
        }
    }
}

/// Whether a registered caller may call one operation.
#[must_use]
pub fn service_allowed(service: &str, operation: Operation) -> bool {
    match operation {
        Operation::Smoke => matches!(service.trim(), "smoke-reader" | "gateway"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(raw: &str) -> DateTime<Utc> {
        DateTime::parse_from_rfc3339(raw)
            .expect("a valid instant")
            .with_timezone(&Utc)
    }

    #[test]
    fn every_schedule_is_anchored_to_the_epoch() {
        let schedule = Schedule::parse("every:5").expect("a valid schedule");
        let window = schedule.window(at("2026-11-08T08:03:17Z"));
        assert_eq!(window, at("2026-11-08T08:00:00Z"));
        assert_eq!(schedule.previous_window(window), at("2026-11-08T07:55:00Z"));
        assert_eq!(schedule.next_window(window), at("2026-11-08T08:05:00Z"));
    }

    #[test]
    fn an_hourly_schedule_uses_its_minute() {
        let schedule = Schedule::parse("hourly:15").expect("a valid schedule");
        let window = schedule.window(at("2026-11-08T08:03:00Z"));
        assert_eq!(window, at("2026-11-08T07:15:00Z"));
        let window = schedule.window(at("2026-11-08T08:20:00Z"));
        assert_eq!(window, at("2026-11-08T08:15:00Z"));
    }

    #[test]
    fn a_daily_schedule_uses_utc_time() {
        let schedule = Schedule::parse("daily:07:00").expect("a valid schedule");
        assert_eq!(
            schedule.window(at("2026-11-08T08:00:00Z")),
            at("2026-11-08T07:00:00Z")
        );
        assert_eq!(
            schedule.window(at("2026-11-08T06:59:59Z")),
            at("2026-11-07T07:00:00Z")
        );
    }

    #[test]
    fn a_bad_schedule_is_refused() {
        assert!(Schedule::parse("weekly:1").is_err());
        assert!(Schedule::parse("every:0").is_err());
        assert!(Schedule::parse("every:61").is_err());
        assert!(Schedule::parse("hourly:60").is_err());
        assert!(Schedule::parse("daily:24:00").is_err());
        assert!(Schedule::parse("daily:7").is_err());
    }

    #[test]
    fn a_short_every_schedule_retries_in_five_minutes() {
        let schedule = Schedule::parse("every:5").expect("a valid schedule");
        let window = at("2026-11-08T08:00:00Z");
        assert_eq!(schedule.retry_at(window), at("2026-11-08T08:05:00Z"));
        let daily = Schedule::parse("daily:07:00").expect("a valid schedule");
        assert_eq!(daily.retry_at(window), at("2026-11-09T08:00:00Z"));
    }

    #[test]
    fn the_catalogue_names_a_provider_for_every_job() {
        for (job, provider) in CATALOGUE {
            assert!(is_known_job(job));
            assert_eq!(provider_for(job), Some(provider));
        }
        assert!(!is_known_job("drop-everything"));
    }

    #[test]
    fn a_manual_run_needs_a_reason() {
        let error = JobRun::queued(
            RunId::generate(),
            "account-sync",
            at("2026-11-08T07:00:00Z"),
            1,
            Trigger::Manual,
            None,
            None,
        )
        .expect_err("a manual run needs a reason");
        assert_eq!(error, DomainError::Required { field: "reason" });
    }

    #[test]
    fn a_backfill_run_needs_a_reason() {
        let error = JobRun::queued(
            RunId::generate(),
            "account-sync",
            at("2026-11-08T07:00:00Z"),
            1,
            Trigger::Backfill,
            Some(UserId::generate()),
            Some("  "),
        )
        .expect_err("a backfill needs a reason");
        assert_eq!(error, DomainError::Required { field: "reason" });
    }

    #[test]
    fn an_unknown_job_is_refused() {
        let error = JobRun::queued(
            RunId::generate(),
            "drop-everything",
            at("2026-11-08T07:00:00Z"),
            1,
            Trigger::Scheduled,
            None,
            None,
        )
        .expect_err("an unknown job is refused");
        assert!(matches!(error, DomainError::Invalid { field: "job", .. }));
    }

    #[test]
    fn a_queued_run_claims_then_succeeds() {
        let mut run = JobRun::queued(
            RunId::generate(),
            "account-sync",
            at("2026-11-08T07:00:00Z"),
            1,
            Trigger::Scheduled,
            None,
            None,
        )
        .expect("a valid run");
        assert_eq!(run.status, RunStatus::Pending);
        run.claim(
            "instance-1",
            at("2026-11-08T08:00:00Z"),
            at("2026-11-08T08:00:00Z"),
        )
        .expect("a pending run can be claimed");
        assert_eq!(run.status, RunStatus::Running);
        assert_eq!(run.owner.as_deref(), Some("instance-1"));
        run.finish(
            RunStatus::Succeeded,
            Some(200),
            None,
            at("2026-11-08T08:00:01Z"),
            None,
        )
        .expect("a running run can succeed");
        assert!(run.status.is_terminal());
        assert!(run.finished_at.is_some());
        assert_eq!(run.owner, None);
    }

    #[test]
    fn a_claim_on_a_running_run_is_refused() {
        let mut run = JobRun::queued(
            RunId::generate(),
            "account-sync",
            at("2026-11-08T07:00:00Z"),
            1,
            Trigger::Scheduled,
            None,
            None,
        )
        .expect("a valid run");
        run.claim("a", at("2026-11-08T08:00:00Z"), at("2026-11-08T08:00:00Z"))
            .expect("the first claim succeeds");
        assert!(run
            .claim("b", at("2026-11-08T08:00:00Z"), at("2026-11-08T08:00:00Z"))
            .is_err());
    }

    #[test]
    fn a_failed_finish_keeps_the_next_attempt_time() {
        let mut run = JobRun::queued(
            RunId::generate(),
            "account-sync",
            at("2026-11-08T07:00:00Z"),
            1,
            Trigger::Scheduled,
            None,
            None,
        )
        .expect("a valid run");
        run.claim("a", at("2026-11-08T08:00:00Z"), at("2026-11-08T08:00:00Z"))
            .expect("the claim succeeds");
        let next = at("2026-11-08T09:00:00Z");
        run.finish(
            RunStatus::Failed,
            Some(503),
            Some("target unavailable"),
            at("2026-11-08T08:00:01Z"),
            Some(next),
        )
        .expect("the failure is recorded");
        assert_eq!(run.next_attempt_at, Some(next));
        assert!(!run.status.is_terminal());
    }

    #[test]
    fn a_terminal_status_cannot_be_finished_again() {
        let mut run = JobRun::queued(
            RunId::generate(),
            "account-sync",
            at("2026-11-08T07:00:00Z"),
            1,
            Trigger::Scheduled,
            None,
            None,
        )
        .expect("a valid run");
        run.claim("a", at("2026-11-08T08:00:00Z"), at("2026-11-08T08:00:00Z"))
            .expect("the claim succeeds");
        run.finish(
            RunStatus::Skipped,
            Some(202),
            Some("run active"),
            at("2026-11-08T08:00:01Z"),
            None,
        )
        .expect("the skip is recorded");
        assert!(run
            .finish(
                RunStatus::Succeeded,
                Some(200),
                None,
                at("2026-11-08T08:00:02Z"),
                None
            )
            .is_err());
    }

    #[test]
    fn a_pending_or_running_finish_is_refused() {
        let mut run = JobRun::queued(
            RunId::generate(),
            "account-sync",
            at("2026-11-08T07:00:00Z"),
            1,
            Trigger::Scheduled,
            None,
            None,
        )
        .expect("a valid run");
        run.claim("a", at("2026-11-08T08:00:00Z"), at("2026-11-08T08:00:00Z"))
            .expect("the claim succeeds");
        assert!(run
            .finish(
                RunStatus::Running,
                None,
                None,
                at("2026-11-08T08:00:01Z"),
                None
            )
            .is_err());
    }

    #[test]
    fn an_overlong_detail_is_refused() {
        let mut run = JobRun::queued(
            RunId::generate(),
            "account-sync",
            at("2026-11-08T07:00:00Z"),
            1,
            Trigger::Scheduled,
            None,
            None,
        )
        .expect("a valid run");
        run.claim("a", at("2026-11-08T08:00:00Z"), at("2026-11-08T08:00:00Z"))
            .expect("the claim succeeds");
        let detail = "x".repeat(MAX_DETAIL_CHARS + 1);
        assert!(run
            .finish(
                RunStatus::Failed,
                Some(503),
                Some(&detail),
                at("2026-11-08T08:00:01Z"),
                None
            )
            .is_err());
    }

    #[test]
    fn statuses_and_triggers_round_trip() {
        for status in [
            RunStatus::Pending,
            RunStatus::Running,
            RunStatus::Succeeded,
            RunStatus::Failed,
            RunStatus::Skipped,
            RunStatus::Abandoned,
        ] {
            assert_eq!(RunStatus::parse(status.as_str()), Ok(status));
        }
        for trigger in [
            Trigger::Scheduled,
            Trigger::Missed,
            Trigger::Backfill,
            Trigger::Manual,
        ] {
            assert_eq!(Trigger::parse(trigger.as_str()), Ok(trigger));
        }
        assert!(RunStatus::parse("done").is_err());
    }

    #[test]
    fn the_terminal_set_is_exactly_success_skip_and_abandon() {
        assert!(RunStatus::Succeeded.is_terminal());
        assert!(RunStatus::Skipped.is_terminal());
        assert!(RunStatus::Abandoned.is_terminal());
        assert!(!RunStatus::Failed.is_terminal());
        assert!(!RunStatus::Pending.is_terminal());
        assert!(!RunStatus::Running.is_terminal());
    }

    #[test]
    fn only_the_smoke_reader_may_call_smoke() {
        assert!(service_allowed("smoke-reader", Operation::Smoke));
        assert!(!service_allowed("reporting", Operation::Smoke));
    }

    #[test]
    fn ids_round_trip_and_refuse_junk() {
        let id = RunId::generate();
        assert_eq!(RunId::parse(&id.to_string()), Ok(id));
        assert!(RunId::parse("not-a-uuid").is_err());
    }
}
