//! BR3.1 and BR3.2: the pure aggregate formulas and the suppression decision.
//!
//! The application fetches three separate bounded windows — the closed day plus
//! the weekly-active and 24-hour activation look-backs, the day-7 signup cohort,
//! and the day-30 signup cohort — and the domain computes the figures. A cohort
//! below ten distinct users is reported as suppressed rather than stored.

use std::collections::{BTreeMap, BTreeSet};

use chrono::{DateTime, Datelike, Duration, Utc};
use rust_decimal::Decimal;

use crate::aggregate::{Metric, MetricCandidate, Period};
use crate::event::{definition, EventClass};

/// The minimum data one metric needs from a stored event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EventFact {
    /// The catalogue event name.
    pub event_name: String,
    /// When the event happened.
    pub occurred_at: DateTime<Utc>,
    /// The salted user hash, when the event carried one.
    pub user_ref: Option<String>,
    /// The allow-listed properties.
    pub properties: BTreeMap<String, String>,
}

/// The three bounded windows a nightly run supplies.
#[derive(Debug, Clone, Copy)]
pub struct AggregateInputs<'a> {
    /// `[day_start - 7 days, day_start + 2 days)`; carries the closed day, the
    /// weekly-active window, and the 24-hour activation look-ahead.
    pub daily: &'a [EventFact],
    /// `[day_start - 7 days, day_start - 6 days)`; the day-7 signup cohort.
    pub cohort_d7: &'a [EventFact],
    /// `[day_start - 30 days, day_start - 29 days)`; the day-30 signup cohort.
    pub cohort_d30: &'a [EventFact],
}

/// Computes the six nightly figures for the closed day beginning at `day_start`.
#[must_use]
pub fn compute_metrics(
    inputs: AggregateInputs<'_>,
    day_start: DateTime<Utc>,
) -> Vec<MetricCandidate> {
    let day_end = day_start + Duration::days(1);
    let daily: Vec<&EventFact> = inputs
        .daily
        .iter()
        .filter(|event| is_analytics(event))
        .collect();
    let cohort_d7: Vec<&EventFact> = inputs
        .cohort_d7
        .iter()
        .filter(|event| is_analytics(event))
        .collect();
    let cohort_d30: Vec<&EventFact> = inputs
        .cohort_d30
        .iter()
        .filter(|event| is_analytics(event))
        .collect();

    let signups = users_with(&daily, "signup_started", day_start, day_end);
    let activated = activated_users(&daily, &signups, day_start, day_end);
    let active = users_any(&daily, day_start, day_end);

    let mut candidates = Vec::with_capacity(6);
    candidates.push(candidate(
        Metric::Activation,
        Period::Daily,
        "all",
        day_start,
        activated.len(),
        signups.len(),
        signups.len(),
    ));

    let weekly_active = users_any(&daily, day_start - Duration::days(7), day_end);
    candidates.push(candidate(
        Metric::Engagement,
        Period::Daily,
        "all",
        day_start,
        active.len(),
        weekly_active.len(),
        weekly_active.len(),
    ));

    let week_start = start_of_week(day_start);
    candidates.push(retention_candidate(
        &cohort_d7,
        &daily,
        day_start,
        7,
        Metric::RetentionD7,
        week_start,
    ));
    candidates.push(retention_candidate(
        &cohort_d30,
        &daily,
        day_start,
        30,
        Metric::RetentionD30,
        week_start,
    ));

    let upgrades = users_with(&daily, "upgrade_started", day_start, day_end);
    candidates.push(candidate(
        Metric::Conversion,
        Period::Daily,
        "all",
        day_start,
        upgrades.len(),
        active.len(),
        active.len(),
    ));

    let reads: Vec<&EventFact> = daily
        .iter()
        .copied()
        .filter(|event| {
            event.event_name == "valuation_read"
                && event.occurred_at >= day_start
                && event.occurred_at < day_end
        })
        .collect();
    let live = reads
        .iter()
        .filter(|event| {
            event
                .properties
                .get("freshness")
                .is_some_and(|value| value == "live")
        })
        .count();
    let read_users = distinct_users(reads.into_iter());
    candidates.push(candidate(
        Metric::Quality,
        Period::Daily,
        "all",
        day_start,
        live,
        reads_len(&daily, day_start, day_end),
        read_users.len(),
    ));

    candidates
}

fn candidate(
    metric: Metric,
    period: Period,
    cohort: &str,
    period_start: DateTime<Utc>,
    numerator: usize,
    denominator: usize,
    distinct_users: usize,
) -> MetricCandidate {
    MetricCandidate {
        metric,
        period,
        cohort: cohort.to_string(),
        period_start,
        value: ratio(numerator, denominator),
        distinct_users: u64::try_from(distinct_users).unwrap_or(u64::MAX),
    }
}

fn retention_candidate(
    cohort_events: &[&EventFact],
    daily: &[&EventFact],
    day_start: DateTime<Utc>,
    age_days: i64,
    metric: Metric,
    week_start: DateTime<Utc>,
) -> MetricCandidate {
    let cohort_start = day_start - Duration::days(age_days);
    let cohort_end = cohort_start + Duration::days(1);
    let cohort_users = users_with(cohort_events, "signup_started", cohort_start, cohort_end);
    let retained = users_with(
        daily,
        "valuation_read",
        day_start,
        day_start + Duration::days(1),
    );
    let retained_in_cohort = cohort_users.intersection(&retained).count();
    // Both retention ages use the non-identifying `all` cohort label; the age is
    // carried by the metric name, `retention_d7` or `retention_d30`.
    candidate(
        metric,
        Period::Weekly,
        "all",
        week_start,
        retained_in_cohort,
        cohort_users.len(),
        cohort_users.len(),
    )
}

fn activated_users(
    events: &[&EventFact],
    signups: &BTreeSet<String>,
    day_start: DateTime<Utc>,
    day_end: DateTime<Utc>,
) -> BTreeSet<String> {
    let mut signup_at: BTreeMap<String, DateTime<Utc>> = BTreeMap::new();
    let mut completions: BTreeMap<String, Vec<DateTime<Utc>>> = BTreeMap::new();
    for event in events {
        let Some(user) = event.user_ref.as_ref() else {
            continue;
        };
        match event.event_name.as_str() {
            "signup_started" if event.occurred_at >= day_start && event.occurred_at < day_end => {
                signup_at
                    .entry(user.clone())
                    .and_modify(|existing| *existing = (*existing).min(event.occurred_at))
                    .or_insert(event.occurred_at);
            }
            "onboarding_completed" => {
                completions
                    .entry(user.clone())
                    .or_default()
                    .push(event.occurred_at);
            }
            _ => {}
        }
    }
    signups
        .iter()
        .filter(|user| {
            let Some(signed_up) = signup_at.get(*user) else {
                return false;
            };
            let Some(times) = completions.get(*user) else {
                return false;
            };
            times.iter().any(|completed| {
                *completed >= *signed_up && *completed <= *signed_up + Duration::hours(24)
            })
        })
        .cloned()
        .collect()
}

fn users_with(
    events: &[&EventFact],
    name: &str,
    from: DateTime<Utc>,
    to: DateTime<Utc>,
) -> BTreeSet<String> {
    events
        .iter()
        .filter(|event| {
            event.event_name == name && event.occurred_at >= from && event.occurred_at < to
        })
        .filter_map(|event| event.user_ref.clone())
        .collect()
}

fn users_any(events: &[&EventFact], from: DateTime<Utc>, to: DateTime<Utc>) -> BTreeSet<String> {
    events
        .iter()
        .filter(|event| event.occurred_at >= from && event.occurred_at < to)
        .filter_map(|event| event.user_ref.clone())
        .collect()
}

fn distinct_users<'a>(events: impl Iterator<Item = &'a EventFact>) -> BTreeSet<String> {
    events.filter_map(|event| event.user_ref.clone()).collect()
}

fn reads_len(events: &[&EventFact], from: DateTime<Utc>, to: DateTime<Utc>) -> usize {
    events
        .iter()
        .filter(|event| {
            event.event_name == "valuation_read"
                && event.occurred_at >= from
                && event.occurred_at < to
        })
        .count()
}

fn is_analytics(event: &EventFact) -> bool {
    definition(&event.event_name).is_some_and(|entry| entry.class == EventClass::Analytics)
}

fn ratio(numerator: usize, denominator: usize) -> Decimal {
    if denominator == 0 {
        return Decimal::ZERO;
    }
    (Decimal::from(u64::try_from(numerator).unwrap_or(u64::MAX))
        / Decimal::from(u64::try_from(denominator).unwrap_or(u64::MAX)))
    .round_dp(4)
}

fn start_of_week(day_start: DateTime<Utc>) -> DateTime<Utc> {
    let days_from_monday = i64::from(day_start.weekday().num_days_from_monday());
    day_start - Duration::days(days_from_monday)
}
