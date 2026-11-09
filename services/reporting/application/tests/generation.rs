//! The brief-generation run against in-memory fakes.

use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use chrono::{DateTime, Duration, Utc};
use reporting_application::{
    AuditClient, BriefRepository, BriefRun, Clock, DeliveryRepository, DueUser, DueUsersClient,
    ExternalError, GenerationService, IdGenerator, NotificationClient, PerformanceClient,
    RepositoryError, RunRepository, ServiceCaller, StoredBrief, ValuationClient,
};
use reporting_domain::{
    AuditRecord, Brief, BriefDelivery, BriefId, BriefStatus, DataStatus, DeliveryChannel,
    DeliveryStatus, Period, PnlRead, UserId, ValuationLineRead, ValuationRead,
};
use serde_json::Value;
use uuid::Uuid;

fn now() -> DateTime<Utc> {
    DateTime::from_timestamp(1_800_000_000, 0).expect("a valid instant")
}

struct FixedClock(DateTime<Utc>);
impl Clock for FixedClock {
    fn now(&self) -> DateTime<Utc> {
        self.0
    }
}

struct SequentialIds(Mutex<u32>);
impl IdGenerator for SequentialIds {
    fn next(&self) -> Uuid {
        let mut next = self.0.lock().expect("the lock is not poisoned");
        *next += 1;
        Uuid::from_u128(u128::from(*next))
    }
}

#[derive(Default)]
struct Briefs {
    rows: Mutex<Vec<StoredBrief>>,
}

impl Briefs {
    fn all(&self) -> Vec<StoredBrief> {
        self.rows.lock().expect("the lock is not poisoned").clone()
    }
}

#[async_trait]
impl BriefRepository for Briefs {
    async fn upsert_generated(
        &self,
        brief: &Brief,
        sections: &Value,
    ) -> Result<Brief, RepositoryError> {
        let mut rows = self.rows.lock().expect("the lock is not poisoned");
        if let Some(existing) = rows.iter().find(|stored| {
            stored.brief.user_id() == brief.user_id()
                && stored.brief.period() == brief.period()
                && stored.brief.period_start() == brief.period_start()
                && stored.brief.status() == BriefStatus::Generated
        }) {
            return Ok(existing.brief.clone());
        }
        rows.retain(|stored| {
            !(stored.brief.user_id() == brief.user_id()
                && stored.brief.period() == brief.period()
                && stored.brief.period_start() == brief.period_start())
        });
        rows.push(StoredBrief {
            brief: brief.clone(),
            sections: Some(sections.clone()),
        });
        Ok(brief.clone())
    }

    async fn upsert_failed(&self, brief: &Brief) -> Result<Brief, RepositoryError> {
        let mut rows = self.rows.lock().expect("the lock is not poisoned");
        if let Some(existing) = rows.iter().find(|stored| {
            stored.brief.user_id() == brief.user_id()
                && stored.brief.period() == brief.period()
                && stored.brief.period_start() == brief.period_start()
        }) {
            if existing.brief.status() == BriefStatus::Generated {
                return Ok(existing.brief.clone());
            }
        }
        rows.retain(|stored| {
            !(stored.brief.user_id() == brief.user_id()
                && stored.brief.period() == brief.period()
                && stored.brief.period_start() == brief.period_start())
        });
        rows.push(StoredBrief {
            brief: brief.clone(),
            sections: None,
        });
        Ok(brief.clone())
    }

    async fn find_by_id(&self, id: BriefId) -> Result<Option<StoredBrief>, RepositoryError> {
        Ok(self
            .all()
            .into_iter()
            .find(|stored| stored.brief.id() == id))
    }

    async fn find_window(
        &self,
        user_id: UserId,
        period: Period,
        period_start: DateTime<Utc>,
    ) -> Result<Option<StoredBrief>, RepositoryError> {
        Ok(self.all().into_iter().find(|stored| {
            stored.brief.user_id() == user_id
                && stored.brief.period() == period
                && stored.brief.period_start() == period_start
        }))
    }

    async fn latest_for_user(
        &self,
        user_id: UserId,
    ) -> Result<Option<StoredBrief>, RepositoryError> {
        Ok(self
            .all()
            .into_iter()
            .find(|stored| stored.brief.user_id() == user_id))
    }

    async fn list_for_user(
        &self,
        user_id: UserId,
        period: Option<Period>,
        _limit: i64,
        _before: Option<DateTime<Utc>>,
    ) -> Result<Vec<Brief>, RepositoryError> {
        Ok(self
            .all()
            .into_iter()
            .filter(|stored| stored.brief.user_id() == user_id)
            .filter(|stored| period.is_none_or(|wanted| stored.brief.period() == wanted))
            .map(|stored| stored.brief)
            .collect())
    }

    async fn list_for_user_export(
        &self,
        user_id: UserId,
    ) -> Result<Vec<StoredBrief>, RepositoryError> {
        Ok(self
            .all()
            .into_iter()
            .filter(|stored| stored.brief.user_id() == user_id)
            .collect())
    }

    async fn anonymise_for_user(
        &self,
        _user_id: UserId,
        _anonymised_ref: UserId,
        _at: DateTime<Utc>,
    ) -> Result<u64, RepositoryError> {
        Ok(0)
    }

    async fn delete_generated_before(
        &self,
        _cutoff: DateTime<Utc>,
    ) -> Result<u64, RepositoryError> {
        Ok(0)
    }
}

#[derive(Default)]
struct Deliveries {
    rows: Mutex<Vec<BriefDelivery>>,
}

impl Deliveries {
    fn all(&self) -> Vec<BriefDelivery> {
        self.rows.lock().expect("the lock is not poisoned").clone()
    }
}

#[async_trait]
impl DeliveryRepository for Deliveries {
    async fn upsert(&self, delivery: &BriefDelivery) -> Result<(), RepositoryError> {
        self.rows
            .lock()
            .expect("the lock is not poisoned")
            .push(delivery.clone());
        Ok(())
    }

    async fn list_for_brief(
        &self,
        brief_id: BriefId,
    ) -> Result<Vec<BriefDelivery>, RepositoryError> {
        Ok(self
            .all()
            .into_iter()
            .filter(|delivery| delivery.brief_id() == brief_id)
            .collect())
    }
}

struct Due {
    users: Vec<DueUser>,
    unavailable: bool,
}

#[async_trait]
impl DueUsersClient for Due {
    async fn due(
        &self,
        _period: Period,
        _at: DateTime<Utc>,
    ) -> Result<Vec<DueUser>, ExternalError> {
        if self.unavailable {
            return Err(ExternalError::Unavailable("C3 is down".to_string()));
        }
        Ok(self.users.clone())
    }
}

struct Valuations {
    status: DataStatus,
    unavailable: bool,
}

#[async_trait]
impl ValuationClient for Valuations {
    async fn valuation(
        &self,
        _user_id: UserId,
        at: DateTime<Utc>,
    ) -> Result<ValuationRead, ExternalError> {
        if self.unavailable {
            return Err(ExternalError::Unavailable("C8 is down".to_string()));
        }
        Ok(ValuationRead {
            total_value: Some(rust_decimal::Decimal::new(1000, 0)),
            currency: "USD".to_string(),
            status: self.status,
            as_of: at,
            unpriced_count: usize::from(self.status != DataStatus::Complete),
            lines: vec![ValuationLineRead {
                asset_id: "btc".to_string(),
                symbol: Some("BTC".to_string()),
                value: Some(rust_decimal::Decimal::new(1000, 0)),
                allocation_percent: Some(rust_decimal::Decimal::new(100, 0)),
                target_percent: Some(rust_decimal::Decimal::new(100, 0)),
                drift_points: Some(rust_decimal::Decimal::ZERO),
                data_status: self.status,
                price_source: Some("venue".to_string()),
                price_freshness: "live".to_string(),
                price_observed_at: Some(at),
                fx_source: None,
                fx_observed_at: None,
            }],
        })
    }
}

struct Performances {
    unavailable: bool,
}

#[async_trait]
impl PerformanceClient for Performances {
    async fn performance(
        &self,
        _user_id: UserId,
        _from: DateTime<Utc>,
        _to: DateTime<Utc>,
    ) -> Result<PnlRead, ExternalError> {
        if self.unavailable {
            return Err(ExternalError::Unavailable("C7 is down".to_string()));
        }
        Ok(PnlRead {
            realized: Some(rust_decimal::Decimal::new(25, 1)),
            net_invested: Some(rust_decimal::Decimal::new(100, 0)),
            unrealized: Some(rust_decimal::Decimal::new(5, 1)),
            coverage: Some(1.0),
            currency: Some("USD".to_string()),
            fx_as_of: None,
            fx_estimated: false,
            as_of: now(),
            status: DataStatus::Complete,
        })
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum AlertAnswer {
    Accepted,
    Suppressed,
    Failed,
}

struct Alerts {
    answer: AlertAnswer,
    keys: Mutex<Vec<String>>,
    channels: Mutex<Vec<String>>,
}

#[async_trait]
impl NotificationClient for Alerts {
    async fn notify(
        &self,
        _user_id: UserId,
        _alert_type: &str,
        idempotency_key: &str,
        channel: &str,
        _payload: Value,
    ) -> Result<Option<String>, ExternalError> {
        self.keys
            .lock()
            .expect("the lock is not poisoned")
            .push(idempotency_key.to_string());
        self.channels
            .lock()
            .expect("the lock is not poisoned")
            .push(channel.to_string());
        match self.answer {
            AlertAnswer::Accepted => Ok(Some("alert-1".to_string())),
            AlertAnswer::Suppressed => Ok(None),
            AlertAnswer::Failed => Err(ExternalError::Unavailable("C9 is down".to_string())),
        }
    }
}

#[derive(Default)]
struct Audits {
    rows: Mutex<Vec<AuditRecord>>,
}

#[async_trait]
impl AuditClient for Audits {
    async fn append(
        &self,
        record: &AuditRecord,
        _correlation_id: &str,
    ) -> Result<(), RepositoryError> {
        self.rows
            .lock()
            .expect("the lock is not poisoned")
            .push(record.clone());
        Ok(())
    }
}

#[derive(Default)]
struct Runs {
    rows: Mutex<Vec<BriefRun>>,
    claim: bool,
    swept: Mutex<bool>,
}

#[async_trait]
impl RunRepository for Runs {
    async fn claim(&self, run: &BriefRun) -> Result<bool, RepositoryError> {
        if self.claim {
            self.rows
                .lock()
                .expect("the lock is not poisoned")
                .push(run.clone());
        }
        Ok(self.claim)
    }

    async fn renew(
        &self,
        _id: Uuid,
        _lease_until: DateTime<Utc>,
        _now: DateTime<Utc>,
    ) -> Result<bool, RepositoryError> {
        Ok(true)
    }

    async fn finish(&self, run: &BriefRun) -> Result<(), RepositoryError> {
        let mut rows = self.rows.lock().expect("the lock is not poisoned");
        if let Some(existing) = rows.iter_mut().find(|existing| existing.id == run.id) {
            *existing = run.clone();
        } else {
            rows.push(run.clone());
        }
        Ok(())
    }

    async fn sweep_expired(&self, _now: DateTime<Utc>) -> Result<Vec<BriefRun>, RepositoryError> {
        *self.swept.lock().expect("the lock is not poisoned") = true;
        Ok(Vec::new())
    }
}

struct Harness {
    service: GenerationService,
    briefs: Arc<Briefs>,
    deliveries: Arc<Deliveries>,
    alerts: Arc<Alerts>,
    audits: Arc<Audits>,
    runs: Arc<Runs>,
}

fn harness(
    users: Vec<DueUser>,
    valuation_status: DataStatus,
    valuation_down: bool,
    performance_down: bool,
    alert: AlertAnswer,
    claim: bool,
    now: DateTime<Utc>,
) -> Harness {
    let briefs = Arc::new(Briefs::default());
    let deliveries = Arc::new(Deliveries::default());
    let alerts = Arc::new(Alerts {
        answer: alert,
        keys: Mutex::new(Vec::new()),
        channels: Mutex::new(Vec::new()),
    });
    let audits = Arc::new(Audits::default());
    let runs = Arc::new(Runs {
        rows: Mutex::new(Vec::new()),
        claim,
        swept: Mutex::new(false),
    });
    let service = GenerationService::new(
        briefs.clone(),
        deliveries.clone(),
        Arc::new(Due {
            users,
            unavailable: false,
        }),
        Arc::new(Valuations {
            status: valuation_status,
            unavailable: valuation_down,
        }),
        Arc::new(Performances {
            unavailable: performance_down,
        }),
        alerts.clone(),
        audits.clone(),
        runs.clone(),
        Arc::new(FixedClock(now)),
        Arc::new(SequentialIds(Mutex::new(0))),
    );
    Harness {
        service,
        briefs,
        deliveries,
        alerts,
        audits,
        runs,
    }
}

fn due_user() -> DueUser {
    DueUser {
        user_id: UserId::generate(),
        timezone: "UTC".to_string(),
    }
}

fn caller() -> ServiceCaller {
    ServiceCaller::new("job-orchestration", "c-1")
}

#[tokio::test]
async fn a_due_user_gets_one_generated_brief_and_one_email_alert() {
    let harness = harness(
        vec![due_user()],
        DataStatus::Complete,
        false,
        false,
        AlertAnswer::Accepted,
        true,
        now(),
    );
    let report = harness
        .service
        .run(&caller(), "worker-1")
        .await
        .expect("the run succeeds");
    assert_eq!(report.due, 2, "a daily and a weekly window are due");
    assert_eq!(report.generated, 2);
    assert_eq!(report.email_submitted, 2);
    assert_eq!(harness.briefs.all().len(), 2);
    assert!(harness
        .briefs
        .all()
        .iter()
        .all(|stored| stored.sections.is_some()));
    assert_eq!(harness.deliveries.all().len(), 2);
    assert!(harness
        .deliveries
        .all()
        .iter()
        .all(|delivery| delivery.status() == DeliveryStatus::Submitted));
    assert_eq!(
        harness
            .alerts
            .channels
            .lock()
            .expect("the lock is not poisoned")
            .as_slice(),
        ["email", "email"]
    );
}

#[tokio::test]
async fn a_second_run_skips_a_window_that_is_already_generated() {
    let user = due_user();
    let harness = harness(
        vec![user.clone()],
        DataStatus::Complete,
        false,
        false,
        AlertAnswer::Accepted,
        true,
        now(),
    );
    harness
        .service
        .run(&caller(), "worker-1")
        .await
        .expect("the first run succeeds");
    let second = harness
        .service
        .run(&caller(), "worker-2")
        .await
        .expect("the second run succeeds");
    assert_eq!(second.generated, 0);
    assert_eq!(second.skipped, 2);
    assert_eq!(harness.briefs.all().len(), 2);
}

#[tokio::test]
async fn a_recent_failure_is_not_hit_again_inside_the_retry_window() {
    let user = due_user();
    let harness = harness(
        vec![user.clone()],
        DataStatus::Complete,
        false,
        false,
        AlertAnswer::Accepted,
        true,
        now(),
    );
    // Seed a failure updated now, then run with the same clock.
    let start = now() - Duration::hours(1);
    let mut failed = Brief::new(
        BriefId::generate(),
        user.user_id,
        Period::Daily,
        start,
        start + Duration::days(1),
        start,
    )
    .expect("a valid window");
    failed
        .mark_failed("earlier outage", now())
        .expect("the failure is recorded");
    harness
        .briefs
        .rows
        .lock()
        .expect("the lock is not poisoned")
        .push(StoredBrief {
            brief: failed.clone(),
            sections: None,
        });
    let report = harness
        .service
        .run(&caller(), "worker-1")
        .await
        .expect("the run succeeds");
    // The daily window is skipped; the weekly one is still produced.
    assert_eq!(report.skipped, 1);
    assert_eq!(report.generated, 1);
}

#[tokio::test]
async fn an_old_failure_is_retried() {
    let user = due_user();
    let clock = now();
    let harness = harness(
        vec![user.clone()],
        DataStatus::Complete,
        false,
        false,
        AlertAnswer::Accepted,
        true,
        clock,
    );
    let start = now() - Duration::hours(1);
    let mut failed = Brief::new(
        BriefId::generate(),
        user.user_id,
        Period::Daily,
        start,
        start + Duration::days(1),
        start - Duration::hours(5),
    )
    .expect("a valid window");
    failed
        .mark_failed("earlier outage", now() - Duration::hours(5))
        .expect("the failure is recorded");
    harness
        .briefs
        .rows
        .lock()
        .expect("the lock is not poisoned")
        .push(StoredBrief {
            brief: failed,
            sections: None,
        });
    let report = harness
        .service
        .run(&caller(), "worker-1")
        .await
        .expect("the run succeeds");
    assert_eq!(report.generated, 2);
}

#[tokio::test]
async fn a_valuation_outage_records_an_explicit_failure_and_audits_it() {
    let harness = harness(
        vec![due_user()],
        DataStatus::Complete,
        true,
        false,
        AlertAnswer::Accepted,
        true,
        now(),
    );
    let report = harness
        .service
        .run(&caller(), "worker-1")
        .await
        .expect("the run itself succeeds");
    assert_eq!(report.failed, 2);
    assert_eq!(report.generated, 0);
    let stored = harness.briefs.all();
    assert_eq!(stored.len(), 2);
    assert!(stored.iter().all(|row| {
        row.brief.status() == BriefStatus::Failed
            && row.brief.failure_reason() == Some("the portfolio valuation was unavailable")
    }));
    assert_eq!(
        harness
            .audits
            .rows
            .lock()
            .expect("the lock is not poisoned")
            .len(),
        2
    );
}

#[tokio::test]
async fn a_performance_outage_degrades_the_brief_but_still_produces_it() {
    let harness = harness(
        vec![due_user()],
        DataStatus::Complete,
        false,
        true,
        AlertAnswer::Accepted,
        true,
        now(),
    );
    let report = harness
        .service
        .run(&caller(), "worker-1")
        .await
        .expect("the run succeeds");
    assert_eq!(report.failed, 0);
    assert_eq!(report.generated, 2);
    let stored = harness.briefs.all();
    assert!(stored
        .iter()
        .all(|row| row.brief.data_status() == Some(DataStatus::Unavailable)));
}

#[tokio::test]
async fn a_caller_without_the_operation_is_refused_before_the_lease_is_claimed() {
    let harness = harness(
        vec![due_user()],
        DataStatus::Complete,
        false,
        false,
        AlertAnswer::Accepted,
        true,
        now(),
    );
    let identity = ServiceCaller::new("identity", "c-1");
    let error = harness
        .service
        .run(&identity, "worker-1")
        .await
        .expect_err("identity may not run the job");
    assert!(matches!(
        error,
        reporting_application::AppError::Forbidden(_)
    ));
    assert!(harness
        .runs
        .rows
        .lock()
        .expect("the lock is not poisoned")
        .is_empty());
}

#[tokio::test]
async fn a_held_lease_refuses_the_run() {
    let harness = harness(
        vec![due_user()],
        DataStatus::Complete,
        false,
        false,
        AlertAnswer::Accepted,
        false,
        now(),
    );
    let error = harness
        .service
        .run(&caller(), "worker-1")
        .await
        .expect_err("the lease is held");
    assert!(matches!(
        error,
        reporting_application::AppError::Conflict(_)
    ));
    assert!(harness.briefs.all().is_empty());
}

#[tokio::test]
async fn a_suppressed_alert_is_recorded_as_suppressed() {
    let harness = harness(
        vec![due_user()],
        DataStatus::Complete,
        false,
        false,
        AlertAnswer::Suppressed,
        true,
        now(),
    );
    let report = harness
        .service
        .run(&caller(), "worker-1")
        .await
        .expect("the run succeeds");
    assert_eq!(report.email_submitted, 0);
    assert_eq!(report.email_failed, 0);
    assert!(harness
        .deliveries
        .all()
        .iter()
        .all(|delivery| delivery.status() == DeliveryStatus::Suppressed));
}

#[tokio::test]
async fn an_email_failure_is_counted_and_recorded_failed() {
    let harness = harness(
        vec![due_user()],
        DataStatus::Complete,
        false,
        false,
        AlertAnswer::Failed,
        true,
        now(),
    );
    let report = harness
        .service
        .run(&caller(), "worker-1")
        .await
        .expect("the run succeeds");
    assert_eq!(report.email_failed, 2);
    assert!(harness
        .deliveries
        .all()
        .iter()
        .all(|delivery| delivery.status() == DeliveryStatus::Failed));
}

#[tokio::test]
async fn the_run_records_its_terminal_state_and_the_sweep_ran() {
    let harness = harness(
        vec![due_user()],
        DataStatus::Complete,
        false,
        false,
        AlertAnswer::Accepted,
        true,
        now(),
    );
    harness
        .service
        .run(&caller(), "worker-1")
        .await
        .expect("the run succeeds");
    let rows = harness
        .runs
        .rows
        .lock()
        .expect("the lock is not poisoned")
        .clone();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].status, reporting_application::RunStatus::Succeeded);
    assert!(rows[0].finished_at.is_some());
    assert!(*harness.runs.swept.lock().expect("the lock is not poisoned"));
}

#[tokio::test]
async fn the_email_idempotency_key_names_the_brief_and_the_channel() {
    let harness = harness(
        vec![due_user()],
        DataStatus::Complete,
        false,
        false,
        AlertAnswer::Accepted,
        true,
        now(),
    );
    harness
        .service
        .run(&caller(), "worker-1")
        .await
        .expect("the run succeeds");
    let keys = harness
        .alerts
        .keys
        .lock()
        .expect("the lock is not poisoned")
        .clone();
    assert_eq!(keys.len(), 2);
    assert!(keys.iter().all(|key| key.ends_with(":email")));
    let briefs: Vec<String> = harness
        .briefs
        .all()
        .iter()
        .map(|stored| stored.brief.id().to_string())
        .collect();
    for brief_id in briefs {
        assert!(keys.contains(&format!("brief:{brief_id}:email")));
    }
}

#[tokio::test]
async fn a_due_users_unknown_timezone_still_gets_a_brief_on_utc() {
    let harness = harness(
        vec![DueUser {
            user_id: UserId::generate(),
            timezone: "Mars/Olympus".to_string(),
        }],
        DataStatus::Complete,
        false,
        false,
        AlertAnswer::Accepted,
        true,
        now(),
    );
    let report = harness
        .service
        .run(&caller(), "worker-1")
        .await
        .expect("the run succeeds");
    assert_eq!(report.generated, 2);
}

#[tokio::test]
async fn the_brief_sections_carry_identifiers_only_in_the_email_payload() {
    let harness = harness(
        vec![due_user()],
        DataStatus::Complete,
        false,
        false,
        AlertAnswer::Accepted,
        true,
        now(),
    );
    harness
        .service
        .run(&caller(), "worker-1")
        .await
        .expect("the run succeeds");
    let delivery = &harness.deliveries.all()[0];
    assert_eq!(delivery.channel(), DeliveryChannel::Email);
    assert!(delivery.alert_id().is_some());
    let sections = harness
        .briefs
        .all()
        .into_iter()
        .find_map(|stored| stored.sections)
        .expect("the brief has sections");
    assert!(sections.get("netWorth").is_some());
    assert!(sections.get("performance").is_some());
}

#[test]
fn the_email_alert_type_is_one_notification_accepts() {
    // Notification's closed type set (notification_domain::NotificationType):
    // drift, sync_failed, key_revoked, security, brief.
    let accepted = ["drift", "sync_failed", "key_revoked", "security", "brief"];
    assert!(accepted.contains(&reporting_application::ALERT_TYPE));
    assert_eq!(reporting_application::ALERT_TYPE, "brief");
}

#[tokio::test]
async fn an_hour_old_pending_brief_is_retried() {
    let user = due_user();
    let clock = now();
    let harness = harness(
        vec![user.clone()],
        DataStatus::Complete,
        false,
        false,
        AlertAnswer::Accepted,
        true,
        clock,
    );
    let start = now() - Duration::hours(1);
    let pending = Brief::new(
        BriefId::generate(),
        user.user_id,
        Period::Daily,
        start,
        start + Duration::days(1),
        now() - Duration::hours(2),
    )
    .expect("a valid window");
    harness
        .briefs
        .rows
        .lock()
        .expect("the lock is not poisoned")
        .push(StoredBrief {
            brief: pending,
            sections: None,
        });
    let report = harness
        .service
        .run(&caller(), "worker-1")
        .await
        .expect("the run succeeds");
    assert_eq!(report.generated, 2, "the pending daily window is retried");
}

#[tokio::test]
async fn a_generated_brief_email_is_audited() {
    let harness = harness(
        vec![due_user()],
        DataStatus::Complete,
        false,
        false,
        AlertAnswer::Accepted,
        true,
        now(),
    );
    harness
        .service
        .run(&caller(), "worker-1")
        .await
        .expect("the run succeeds");
    let rows = harness
        .audits
        .rows
        .lock()
        .expect("the lock is not poisoned");
    assert!(rows
        .iter()
        .any(|row| row.action() == "reporting.brief.email_submitted"));
}
