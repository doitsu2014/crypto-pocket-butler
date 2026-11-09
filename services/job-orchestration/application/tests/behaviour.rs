//! Scheduler and operator behaviour against in-memory fakes.

use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use chrono::{DateTime, Duration, Utc};
use job_orchestration_application::common::Caller;
use job_orchestration_application::operator::OperatorService;
use job_orchestration_application::ports::{
    AuditClient, AuditEvent, Clock, HeartbeatRepository, IdGenerator, JobDefinition,
    LeaseRepository, RepositoryError, RunRepository, TargetAnswer, TargetClient,
};
use job_orchestration_application::scheduler::{idempotency_key, Scheduler};
use job_orchestration_domain::{JobRun, RunId, RunStatus, Trigger, UserId};
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

struct Ids(Mutex<u32>);
impl IdGenerator for Ids {
    fn next(&self) -> Uuid {
        let mut next = self.0.lock().expect("the lock is not poisoned");
        *next += 1;
        Uuid::from_u128(u128::from(*next))
    }
}

#[derive(Default)]
struct Runs {
    rows: Mutex<Vec<JobRun>>,
    definitions: Vec<JobDefinition>,
}

impl Runs {
    fn with_definition() -> Self {
        Self {
            rows: Mutex::new(Vec::new()),
            definitions: vec![JobDefinition {
                name: "account-sync".to_string(),
                provider: "account".to_string(),
                schedule: "daily:07:00".to_string(),
                timeout_seconds: 600,
                max_attempts: 3,
                enabled: true,
                last_scheduled_for: None,
            }],
        }
    }

    fn all(&self) -> Vec<JobRun> {
        self.rows.lock().expect("the lock is not poisoned").clone()
    }
}

#[async_trait]
impl RunRepository for Runs {
    async fn definitions(&self) -> Result<Vec<JobDefinition>, RepositoryError> {
        Ok(self.definitions.clone())
    }

    async fn insert(&self, run: &JobRun) -> Result<(), RepositoryError> {
        let mut rows = self.rows.lock().expect("the lock is not poisoned");
        if rows.iter().any(|existing| {
            existing.job == run.job
                && existing.scheduled_for == run.scheduled_for
                && existing.attempt == run.attempt
        }) {
            return Ok(());
        }
        rows.push(run.clone());
        Ok(())
    }

    async fn latest_for_window(
        &self,
        job: &str,
        scheduled_for: DateTime<Utc>,
    ) -> Result<Option<JobRun>, RepositoryError> {
        Ok(self
            .all()
            .into_iter()
            .filter(|run| run.job == job && run.scheduled_for == scheduled_for)
            .max_by_key(|run| run.attempt))
    }

    async fn window_exists(
        &self,
        job: &str,
        scheduled_for: DateTime<Utc>,
    ) -> Result<bool, RepositoryError> {
        Ok(self.all().iter().any(|run| {
            run.job == job
                && run.scheduled_for == scheduled_for
                && matches!(
                    run.status,
                    RunStatus::Pending
                        | RunStatus::Running
                        | RunStatus::Succeeded
                        | RunStatus::Skipped
                        | RunStatus::Abandoned
                )
        }))
    }

    async fn pending(&self, limit: i64) -> Result<Vec<JobRun>, RepositoryError> {
        Ok(self
            .all()
            .into_iter()
            .filter(|run| run.status == RunStatus::Pending)
            .take(usize::try_from(limit).unwrap_or(usize::MAX))
            .collect())
    }

    async fn claim(
        &self,
        id: RunId,
        owner: &str,
        lease_until: DateTime<Utc>,
        started_at: DateTime<Utc>,
    ) -> Result<bool, RepositoryError> {
        let mut rows = self.rows.lock().expect("the lock is not poisoned");
        let Some(run) = rows.iter_mut().find(|run| run.id == id) else {
            return Ok(false);
        };
        if run.status != RunStatus::Pending {
            return Ok(false);
        }
        run.claim(owner, lease_until, started_at)
            .map_err(|error| RepositoryError::InvalidData(error.to_string()))?;
        Ok(true)
    }

    async fn finish(
        &self,
        run: &JobRun,
        owner: &str,
        _now: DateTime<Utc>,
    ) -> Result<bool, RepositoryError> {
        let mut rows = self.rows.lock().expect("the lock is not poisoned");
        let Some(existing) = rows.iter_mut().find(|existing| existing.id == run.id) else {
            return Ok(false);
        };
        if existing.owner.as_deref() != Some(owner) {
            return Ok(false);
        }
        *existing = run.clone();
        Ok(true)
    }

    async fn reap_expired(&self, _now: DateTime<Utc>) -> Result<Vec<JobRun>, RepositoryError> {
        Ok(Vec::new())
    }

    async fn due_retries(&self, _now: DateTime<Utc>) -> Result<Vec<JobRun>, RepositoryError> {
        Ok(self
            .all()
            .into_iter()
            .filter(|run| run.status == RunStatus::Failed)
            .collect())
    }

    async fn list_for_job(
        &self,
        job: &str,
        _from: Option<DateTime<Utc>>,
        _to: Option<DateTime<Utc>>,
        limit: i64,
    ) -> Result<Vec<JobRun>, RepositoryError> {
        Ok(self
            .all()
            .into_iter()
            .filter(|run| run.job == job)
            .take(usize::try_from(limit).unwrap_or(usize::MAX))
            .collect())
    }

    async fn latest_per_job(&self) -> Result<Vec<JobRun>, RepositoryError> {
        Ok(self.all())
    }

    async fn set_watermark(
        &self,
        _job: &str,
        _window: DateTime<Utc>,
    ) -> Result<(), RepositoryError> {
        Ok(())
    }

    async fn prune_before(&self, _cutoff: DateTime<Utc>) -> Result<u64, RepositoryError> {
        Ok(0)
    }
}

struct Lease {
    owner: Mutex<Option<String>>,
}

#[async_trait]
impl LeaseRepository for Lease {
    async fn claim_or_renew(
        &self,
        owner: &str,
        _lease_until: DateTime<Utc>,
        _now: DateTime<Utc>,
    ) -> Result<bool, RepositoryError> {
        let mut held = self.owner.lock().expect("the lock is not poisoned");
        if held.as_deref().is_none_or(|held| held == owner) {
            *held = Some(owner.to_string());
            Ok(true)
        } else {
            Ok(false)
        }
    }

    async fn owned(&self, owner: &str, _now: DateTime<Utc>) -> Result<bool, RepositoryError> {
        Ok(self
            .owner
            .lock()
            .expect("the lock is not poisoned")
            .as_deref()
            == Some(owner))
    }
}

#[derive(Default)]
struct Heartbeat {
    writes: Mutex<u32>,
}

#[async_trait]
impl HeartbeatRepository for Heartbeat {
    async fn write(&self, _owner: &str, _now: DateTime<Utc>) -> Result<(), RepositoryError> {
        *self.writes.lock().expect("the lock is not poisoned") += 1;
        Ok(())
    }

    async fn age_seconds(&self, _now: DateTime<Utc>) -> Result<Option<i64>, RepositoryError> {
        Ok(Some(0))
    }
}

struct Target {
    answer: TargetAnswer,
    keys: Mutex<Vec<String>>,
}

#[async_trait]
impl TargetClient for Target {
    async fn run(
        &self,
        _provider: &str,
        _job: &str,
        _scheduled_for: DateTime<Utc>,
        _owner: &str,
        idempotency_key: &str,
        _timeout_seconds: u64,
    ) -> Result<TargetAnswer, job_orchestration_application::ports::ExternalError> {
        self.keys
            .lock()
            .expect("the lock is not poisoned")
            .push(idempotency_key.to_string());
        Ok(self.answer)
    }
}

#[derive(Default)]
struct Audits {
    events: Mutex<Vec<AuditEvent>>,
}

#[async_trait]
impl AuditClient for Audits {
    async fn append(&self, event: &AuditEvent) -> Result<(), RepositoryError> {
        self.events
            .lock()
            .expect("the lock is not poisoned")
            .push(event.clone());
        Ok(())
    }
}

struct Harness {
    scheduler: Scheduler,
    runs: Arc<Runs>,
    heartbeat: Arc<Heartbeat>,
    target: Arc<Target>,
    _audits: Arc<Audits>,
    _lease: Arc<Lease>,
}

fn harness(answer: TargetAnswer, lease_owner: Option<&str>) -> Harness {
    let runs = Arc::new(Runs::with_definition());
    let heartbeat = Arc::new(Heartbeat::default());
    let target = Arc::new(Target {
        answer,
        keys: Mutex::new(Vec::new()),
    });
    let audits = Arc::new(Audits::default());
    let lease = Arc::new(Lease {
        owner: Mutex::new(lease_owner.map(ToString::to_string)),
    });
    let scheduler = Scheduler::new(
        runs.clone(),
        lease.clone(),
        heartbeat.clone(),
        target.clone(),
        audits.clone(),
        Arc::new(FixedClock(now())),
        Arc::new(Ids(Mutex::new(0))),
    );
    Harness {
        scheduler,
        runs,
        heartbeat,
        target,
        _audits: audits,
        _lease: lease,
    }
}

#[tokio::test]
async fn a_tick_queues_the_window_and_runs_it_once() {
    let harness = harness(TargetAnswer::Succeeded, None);
    let report = harness
        .scheduler
        .tick("instance-1")
        .await
        .expect("the tick runs");
    assert!(report.leased);
    assert_eq!(report.succeeded, 1);
    assert_eq!(harness.runs.all().len(), 1);
    assert_eq!(harness.runs.all()[0].status, RunStatus::Succeeded);
    assert!(harness
        .target
        .keys
        .lock()
        .expect("the lock is not poisoned")[0]
        .starts_with("job:account-sync:"));
    assert!(
        *harness
            .heartbeat
            .writes
            .lock()
            .expect("the lock is not poisoned")
            >= 2
    );
}

#[tokio::test]
async fn a_second_tick_does_not_requeue_a_covered_window() {
    let harness = harness(TargetAnswer::Succeeded, None);
    harness
        .scheduler
        .tick("instance-1")
        .await
        .expect("the first tick runs");
    harness
        .scheduler
        .tick("instance-1")
        .await
        .expect("the second tick runs");
    assert_eq!(harness.runs.all().len(), 1);
}

#[tokio::test]
async fn a_held_lease_keeps_the_instance_idle() {
    let harness = harness(TargetAnswer::Succeeded, Some("other"));
    let report = harness
        .scheduler
        .tick("instance-1")
        .await
        .expect("the tick answers");
    assert!(!report.leased);
    assert!(harness.runs.all().is_empty());
}

#[tokio::test]
async fn a_lease_held_run_is_failed_and_retried_later() {
    let harness = harness(TargetAnswer::LeaseHeld, None);
    harness
        .scheduler
        .tick("instance-1")
        .await
        .expect("the tick runs");
    let run = &harness.runs.all()[0];
    assert_eq!(run.status, RunStatus::Failed);
    assert!(run.next_attempt_at.is_some());
}

#[tokio::test]
async fn an_already_running_answer_is_recorded_skipped() {
    let harness = harness(TargetAnswer::AlreadyRunning, None);
    harness
        .scheduler
        .tick("instance-1")
        .await
        .expect("the tick runs");
    assert_eq!(harness.runs.all()[0].status, RunStatus::Skipped);
}

#[tokio::test]
async fn a_refused_call_is_abandoned() {
    let harness = harness(TargetAnswer::Refused(422), None);
    harness
        .scheduler
        .tick("instance-1")
        .await
        .expect("the tick runs");
    assert_eq!(harness.runs.all()[0].status, RunStatus::Abandoned);
}

#[tokio::test]
async fn an_unavailable_target_is_failed() {
    let harness = harness(TargetAnswer::Unavailable, None);
    harness
        .scheduler
        .tick("instance-1")
        .await
        .expect("the tick runs");
    assert_eq!(harness.runs.all()[0].status, RunStatus::Failed);
}

#[tokio::test]
async fn a_due_retry_creates_the_next_attempt() {
    let harness = harness(TargetAnswer::Failed, None);
    harness
        .scheduler
        .tick("instance-1")
        .await
        .expect("the first tick runs");
    // Make the failure due now.
    {
        let mut rows = harness.runs.rows.lock().expect("the lock is not poisoned");
        rows[0].next_attempt_at = Some(now() - Duration::minutes(1));
    }
    harness
        .scheduler
        .tick("instance-1")
        .await
        .expect("the second tick runs");
    let attempts: Vec<u32> = harness
        .runs
        .all()
        .iter()
        .filter(|run| run.job == "account-sync")
        .map(|run| run.attempt)
        .collect();
    assert!(attempts.contains(&2), "the retry inserted attempt 2");
}

#[test]
fn the_idempotency_key_is_canonical() {
    assert_eq!(
        idempotency_key("account-sync", now()),
        "job:account-sync:2027-01-15T08:00:00Z"
    );
}

// ---------------------------------------------------------------------------
// Operator
// ---------------------------------------------------------------------------

fn operator(runs: Arc<Runs>, audits: Arc<Audits>) -> OperatorService {
    OperatorService::new(
        runs,
        audits,
        Arc::new(FixedClock(now())),
        Arc::new(Ids(Mutex::new(100))),
    )
}

fn admin() -> Caller {
    Caller {
        user_id: UserId::generate(),
        role: "admin".to_string(),
        correlation_id: "c-1".to_string(),
    }
}

fn user() -> Caller {
    Caller {
        user_id: UserId::generate(),
        role: "user".to_string(),
        correlation_id: "c-1".to_string(),
    }
}

#[tokio::test]
async fn a_user_role_cannot_read_jobs() {
    let service = operator(
        Arc::new(Runs::with_definition()),
        Arc::new(Audits::default()),
    );
    assert!(service.jobs(&user()).await.is_err());
}

#[tokio::test]
async fn an_admin_reads_the_catalogue() {
    let service = operator(
        Arc::new(Runs::with_definition()),
        Arc::new(Audits::default()),
    );
    let jobs = service.jobs(&admin()).await.expect("the list answers");
    assert!(jobs.iter().any(|view| view.job == "account-sync"));
}

#[tokio::test]
async fn a_manual_run_queues_the_current_window() {
    let runs = Arc::new(Runs::with_definition());
    let service = operator(runs.clone(), Arc::new(Audits::default()));
    let queued = service
        .run_now(&admin(), "account-sync", "operator asked", "daily:07:00")
        .await
        .expect("the manual run queues");
    assert_eq!(queued.run.trigger, Trigger::Manual);
    assert_eq!(runs.all().len(), 1);
}

#[tokio::test]
async fn a_manual_run_of_a_covered_window_conflicts() {
    let runs = Arc::new(Runs::with_definition());
    let service = operator(runs.clone(), Arc::new(Audits::default()));
    service
        .run_now(&admin(), "account-sync", "first", "daily:07:00")
        .await
        .expect("the first run queues");
    assert!(service
        .run_now(&admin(), "account-sync", "second", "daily:07:00")
        .await
        .is_err());
}

#[tokio::test]
async fn a_backfill_accepts_the_missing_windows_and_reports_covered() {
    let runs = Arc::new(Runs::with_definition());
    let service = operator(runs.clone(), Arc::new(Audits::default()));
    let from = now() - Duration::days(2);
    let to = now() - Duration::days(1);
    let result = service
        .backfill(&admin(), "account-sync", from, to, "gap", "daily:07:00")
        .await
        .expect("the backfill queues");
    assert!(result.accepted >= 1);
    assert_eq!(result.covered, 0);
}

#[tokio::test]
async fn a_backfill_wider_than_seven_days_is_refused() {
    let service = operator(
        Arc::new(Runs::with_definition()),
        Arc::new(Audits::default()),
    );
    let error = service
        .backfill(
            &admin(),
            "account-sync",
            now() - Duration::days(9),
            now(),
            "too wide",
            "daily:07:00",
        )
        .await
        .expect_err("a nine-day backfill is refused");
    assert!(matches!(
        error,
        job_orchestration_application::common::AppError::Validation(_)
    ));
}

#[tokio::test]
async fn an_unknown_job_is_not_found() {
    let service = operator(
        Arc::new(Runs::with_definition()),
        Arc::new(Audits::default()),
    );
    assert!(service
        .runs(&admin(), "drop-everything", None, None, None)
        .await
        .is_err());
}

#[tokio::test]
async fn operator_actions_are_audited() {
    let audits = Arc::new(Audits::default());
    let service = operator(Arc::new(Runs::with_definition()), audits.clone());
    service
        .run_now(&admin(), "account-sync", "operator asked", "daily:07:00")
        .await
        .expect("the run queues");
    assert!(audits
        .events
        .lock()
        .expect("the lock is not poisoned")
        .iter()
        .any(|event| event.action == "job.run.manual"));
}

#[tokio::test]
async fn a_support_role_can_queue_a_manual_run() {
    let runs = Arc::new(Runs::with_definition());
    let service = operator(runs.clone(), Arc::new(Audits::default()));
    let support = Caller {
        user_id: UserId::generate(),
        role: "support".to_string(),
        correlation_id: "c-2".to_string(),
    };
    assert!(service
        .run_now(&support, "account-sync", "support asked", "daily:07:00")
        .await
        .is_ok());
}

#[tokio::test]
async fn a_run_page_is_bounded() {
    let service = operator(
        Arc::new(Runs::with_definition()),
        Arc::new(Audits::default()),
    );
    assert!(service
        .runs(&admin(), "account-sync", None, None, Some(0))
        .await
        .is_err());
    assert!(service
        .runs(&admin(), "account-sync", None, None, Some(501))
        .await
        .is_err());
    assert!(service
        .runs(&admin(), "account-sync", None, None, Some(500))
        .await
        .is_ok());
}

#[tokio::test]
async fn an_inverted_backfill_is_refused() {
    let service = operator(
        Arc::new(Runs::with_definition()),
        Arc::new(Audits::default()),
    );
    assert!(service
        .backfill(
            &admin(),
            "account-sync",
            now(),
            now() - Duration::days(1),
            "inverted",
            "daily:07:00"
        )
        .await
        .is_err());
}

#[tokio::test]
async fn a_backfill_skips_a_window_already_covered() {
    let runs = Arc::new(Runs::with_definition());
    let service = operator(runs.clone(), Arc::new(Audits::default()));
    let from = now() - Duration::days(1);
    let to = now() - Duration::days(1);
    let first = service
        .backfill(&admin(), "account-sync", from, to, "gap", "daily:07:00")
        .await
        .expect("the first backfill queues");
    assert!(first.accepted >= 1);
    let second = service
        .backfill(&admin(), "account-sync", from, to, "again", "daily:07:00")
        .await
        .expect("the second backfill answers");
    assert_eq!(second.accepted, 0);
    assert!(second.covered >= 1);
}

#[tokio::test]
async fn a_manual_run_of_an_unknown_job_is_refused_by_the_domain() {
    let service = operator(
        Arc::new(Runs::with_definition()),
        Arc::new(Audits::default()),
    );
    assert!(service
        .run_now(&admin(), "drop-everything", "nope", "daily:07:00")
        .await
        .is_err());
}

#[tokio::test]
async fn a_failed_retry_is_not_requeued_beyond_the_attempt_bound() {
    let harness = harness(TargetAnswer::Failed, None);
    {
        let mut rows = harness.runs.rows.lock().expect("the lock is not poisoned");
        let mut failed = JobRun::queued(
            RunId::generate(),
            "account-sync",
            now() - Duration::days(1),
            5,
            Trigger::Scheduled,
            None,
            None,
        )
        .expect("a valid run");
        failed.claim("x", now(), now()).expect("claim");
        failed
            .finish(
                RunStatus::Failed,
                Some(503),
                Some("down"),
                now(),
                Some(now() - Duration::minutes(1)),
            )
            .expect("finish");
        rows.push(failed);
    }
    harness
        .scheduler
        .tick("instance-1")
        .await
        .expect("the tick runs");
    assert!(harness
        .runs
        .all()
        .iter()
        .all(|run| !(run.job == "account-sync" && run.attempt == 6)));
}

#[tokio::test]
async fn a_skipped_window_is_not_picked_up_by_a_later_tick() {
    let harness = harness(TargetAnswer::Skipped, None);
    harness
        .scheduler
        .tick("instance-1")
        .await
        .expect("the first tick runs");
    harness
        .scheduler
        .tick("instance-1")
        .await
        .expect("the second tick runs");
    assert_eq!(harness.runs.all().len(), 1);
}

#[tokio::test]
async fn the_operator_list_reports_the_latest_attempt() {
    let runs = Arc::new(Runs::with_definition());
    let service = operator(runs.clone(), Arc::new(Audits::default()));
    service
        .run_now(&admin(), "account-sync", "asked", "daily:07:00")
        .await
        .expect("the run queues");
    let jobs = service.jobs(&admin()).await.expect("the list answers");
    let account = jobs
        .iter()
        .find(|view| view.job == "account-sync")
        .expect("the job");
    assert!(account.latest.is_some());
}
