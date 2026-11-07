//! WF2: nightly aggregation from consented events (BR3.1–BR3.2).
//!
//! The run reads three separate bounded windows. A single wide window capped at
//! the monthly volume would truncate the closed day under load; separate windows
//! keep the closed day complete, and exceeding a window's cap fails the run
//! rather than silently dropping events.

use std::sync::Arc;

use analytics_domain::{
    compute_metrics, AggregateInputs, EventFact, Metric, MetricAggregate, MetricAggregateId,
    MetricCandidate, Period, MAX_COHORT_WINDOW_EVENTS, MAX_DAILY_WINDOW_EVENTS,
};
use chrono::{DateTime, Duration, TimeZone, Utc};

use crate::ports::{AggregateRepository, Clock, EventRepository, RepositoryError};

/// How many events each bounded window read.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ScannedCounts {
    /// `[day-7, day+2)` — the closed day, weekly-active, and activation windows.
    pub daily: usize,
    /// `[day-7, day-6)` — the day-7 signup cohort.
    pub cohort_d7: usize,
    /// `[day-30, day-29)` — the day-30 signup cohort.
    pub cohort_d30: usize,
}

/// What one nightly run produced.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AggregateReport {
    /// The figures that were stored.
    pub stored: Vec<MetricAggregate>,
    /// The cohorts suppressed under BR3.2.
    pub suppressed: Vec<MetricCandidate>,
    /// The closed day the figures cover.
    pub period_start: DateTime<Utc>,
    /// The event counts read from each bounded window.
    pub scanned: ScannedCounts,
}

/// Why an aggregation run failed.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum AggregateError {
    /// A storage failure.
    #[error("aggregation storage failure: {0}")]
    Repository(#[from] RepositoryError),
    /// A computed candidate could not form a valid aggregate.
    #[error("computed aggregate is invalid: {0}")]
    InvalidData(String),
    /// A bounded window held more events than its cap. The run fails rather than
    /// truncating the closed day.
    #[error("the {window} window exceeds its {cap}-event cap")]
    WindowTooLarge {
        /// The window that overflowed.
        window: &'static str,
        /// Its cap.
        cap: u32,
    },
}

/// Computes and stores the nightly figures.
pub struct AggregateService {
    events: Arc<dyn EventRepository>,
    aggregates: Arc<dyn AggregateRepository>,
    clock: Arc<dyn Clock>,
}

impl AggregateService {
    /// Creates the use case.
    #[must_use]
    pub fn new(
        events: Arc<dyn EventRepository>,
        aggregates: Arc<dyn AggregateRepository>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self {
            events,
            aggregates,
            clock,
        }
    }

    /// Runs the aggregation for the closed day before `now`.
    ///
    /// # Errors
    /// Returns [`AggregateError`] when a window cannot be read, a window exceeds
    /// its cap, a figure cannot be formed, or an upsert fails.
    pub async fn run_nightly(&self) -> Result<AggregateReport, AggregateError> {
        let today = Utc.from_utc_datetime(
            &self
                .clock
                .now()
                .date_naive()
                .and_hms_opt(0, 0, 0)
                .expect("midnight is a valid time"),
        );
        let yesterday = today - Duration::days(1);
        self.run(yesterday).await
    }

    /// Runs the aggregation for an explicit closed day.
    ///
    /// # Errors
    /// Returns [`AggregateError`] when a window cannot be read, a window exceeds
    /// its cap, a figure cannot be formed, or an upsert fails.
    pub async fn run(&self, day_start: DateTime<Utc>) -> Result<AggregateReport, AggregateError> {
        // Daily metrics need the closed day, the seven-day weekly-active window,
        // and one day of activation look-ahead.
        let (daily, daily_count) = self
            .window(
                day_start - Duration::days(7),
                day_start + Duration::days(2),
                MAX_DAILY_WINDOW_EVENTS,
                "daily",
            )
            .await?;
        // Each retention cohort is exactly one signup day, a bounded slice.
        let (cohort_d7, cohort_d7_count) = self
            .window(
                day_start - Duration::days(7),
                day_start - Duration::days(6),
                MAX_COHORT_WINDOW_EVENTS,
                "day-7 cohort",
            )
            .await?;
        let (cohort_d30, cohort_d30_count) = self
            .window(
                day_start - Duration::days(30),
                day_start - Duration::days(29),
                MAX_COHORT_WINDOW_EVENTS,
                "day-30 cohort",
            )
            .await?;

        let candidates = compute_metrics(
            AggregateInputs {
                daily: &daily,
                cohort_d7: &cohort_d7,
                cohort_d30: &cohort_d30,
            },
            day_start,
        );
        let computed_at = self.clock.now();
        let mut stored = Vec::new();
        let mut suppressed = Vec::new();
        for candidate in candidates {
            if candidate.is_suppressed() {
                suppressed.push(candidate);
                continue;
            }
            let aggregate = MetricAggregate::from_candidate(
                MetricAggregateId::generate(),
                &candidate,
                computed_at,
            )
            .map_err(|error| AggregateError::InvalidData(error.to_string()))?;
            self.aggregates.upsert(&aggregate).await?;
            stored.push(aggregate);
        }
        Ok(AggregateReport {
            stored,
            suppressed,
            period_start: day_start,
            scanned: ScannedCounts {
                daily: daily_count,
                cohort_d7: cohort_d7_count,
                cohort_d30: cohort_d30_count,
            },
        })
    }

    async fn window(
        &self,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
        cap: u32,
        name: &'static str,
    ) -> Result<(Vec<EventFact>, usize), AggregateError> {
        // Ask for one more than the cap so an exactly-cap-sized result is not
        // mistaken for a truncated one.
        let events = self
            .events
            .events_between(from, to, cap.saturating_add(1))
            .await?;
        if events.len() > cap as usize {
            return Err(AggregateError::WindowTooLarge { window: name, cap });
        }
        let count = events.len();
        let facts = events
            .into_iter()
            .map(|event| EventFact {
                event_name: event.event_name().to_string(),
                occurred_at: event.occurred_at(),
                user_ref: event.user_ref().map(ToString::to_string),
                properties: event.properties().clone(),
            })
            .collect();
        Ok((facts, count))
    }

    /// Reads stored figures for the admin path.
    ///
    /// # Errors
    /// Returns [`AggregateError::Repository`] on storage failure.
    pub async fn list(
        &self,
        metric: Option<Metric>,
        period: Option<Period>,
        from: Option<DateTime<Utc>>,
        to: Option<DateTime<Utc>>,
        limit: u32,
    ) -> Result<Vec<MetricAggregate>, AggregateError> {
        Ok(self
            .aggregates
            .list(metric, period, from, to, limit)
            .await?)
    }
}
