//! Aggregate figures over a period and cohort (BR3.1–BR3.2).

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;

use crate::constants::SUPPRESSION_MIN_USERS;
use crate::error::DomainError;
use crate::ids::MetricAggregateId;

/// The metric an aggregate reports.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Metric {
    /// Onboarding completion within 24 hours of signup.
    Activation,
    /// Distinct daily active divided by distinct weekly active.
    Engagement,
    /// Cohort users with a valuation read on day 7.
    RetentionD7,
    /// Cohort users with a valuation read on day 30.
    RetentionD30,
    /// Users who started an upgrade divided by active users.
    Conversion,
    /// Live valuation reads divided by all valuation reads.
    Quality,
}

impl Metric {
    /// The stored label.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Activation => "activation",
            Self::Engagement => "engagement",
            Self::RetentionD7 => "retention_d7",
            Self::RetentionD30 => "retention_d30",
            Self::Conversion => "conversion",
            Self::Quality => "quality",
        }
    }

    /// Parses a stored label.
    ///
    /// # Errors
    /// Returns [`DomainError::Invalid`] for any other value.
    pub fn parse(raw: &str) -> Result<Self, DomainError> {
        match raw.trim() {
            "activation" => Ok(Self::Activation),
            "engagement" => Ok(Self::Engagement),
            "retention_d7" => Ok(Self::RetentionD7),
            "retention_d30" => Ok(Self::RetentionD30),
            "conversion" => Ok(Self::Conversion),
            "quality" => Ok(Self::Quality),
            _ => Err(DomainError::Invalid {
                field: "metric",
                reason: "must be activation, engagement, retention_d7, retention_d30, conversion, or quality"
                    .to_string(),
            }),
        }
    }
}

/// The aggregation period.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Period {
    /// One closed UTC day.
    Daily,
    /// One week.
    Weekly,
}

impl Period {
    /// The stored label.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Daily => "daily",
            Self::Weekly => "weekly",
        }
    }

    /// Parses a stored label.
    ///
    /// # Errors
    /// Returns [`DomainError::Invalid`] for any other value.
    pub fn parse(raw: &str) -> Result<Self, DomainError> {
        match raw.trim() {
            "daily" => Ok(Self::Daily),
            "weekly" => Ok(Self::Weekly),
            _ => Err(DomainError::Invalid {
                field: "period",
                reason: "must be daily or weekly".to_string(),
            }),
        }
    }
}

/// One computed figure before suppression and storage.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MetricCandidate {
    /// The metric.
    pub metric: Metric,
    /// The period.
    pub period: Period,
    /// The non-identifying cohort label.
    pub cohort: String,
    /// The first instant of the period.
    pub period_start: DateTime<Utc>,
    /// The ratio, between zero and one.
    pub value: Decimal,
    /// The distinct user references the figure covers.
    pub distinct_users: u64,
}

impl MetricCandidate {
    /// BR3.2: a cohort below ten distinct users is suppressed.
    #[must_use]
    pub fn is_suppressed(&self) -> bool {
        self.distinct_users < SUPPRESSION_MIN_USERS
    }
}

/// A stored aggregate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MetricAggregate {
    id: MetricAggregateId,
    metric: Metric,
    period: Period,
    period_start: DateTime<Utc>,
    cohort: String,
    value: Decimal,
    computed_at: DateTime<Utc>,
}

impl MetricAggregate {
    /// Builds a storable aggregate from a candidate that is not suppressed.
    ///
    /// # Errors
    /// Returns [`DomainError::Invalid`] when a cohort label is blank or the value
    /// is outside zero to one.
    pub fn from_candidate(
        id: MetricAggregateId,
        candidate: &MetricCandidate,
        computed_at: DateTime<Utc>,
    ) -> Result<Self, DomainError> {
        let cohort = candidate.cohort.trim();
        if cohort.is_empty() {
            return Err(DomainError::Required { field: "cohort" });
        }
        if candidate.value < Decimal::ZERO || candidate.value > Decimal::ONE {
            return Err(DomainError::Invalid {
                field: "value",
                reason: "must be a ratio between 0 and 1".to_string(),
            });
        }
        Ok(Self {
            id,
            metric: candidate.metric,
            period: candidate.period,
            period_start: candidate.period_start,
            cohort: cohort.to_string(),
            value: candidate.value,
            computed_at,
        })
    }

    /// Rebuilds a stored row.
    ///
    /// # Errors
    /// Returns a domain error when a stored label or value is invalid.
    pub fn from_stored(
        id: MetricAggregateId,
        metric: &str,
        period: &str,
        period_start: DateTime<Utc>,
        cohort: &str,
        value: Decimal,
        computed_at: DateTime<Utc>,
    ) -> Result<Self, DomainError> {
        let candidate = MetricCandidate {
            metric: Metric::parse(metric)?,
            period: Period::parse(period)?,
            cohort: cohort.to_string(),
            period_start,
            value,
            distinct_users: SUPPRESSION_MIN_USERS,
        };
        Self::from_candidate(id, &candidate, computed_at)
    }

    /// The identifier.
    #[must_use]
    pub fn id(&self) -> MetricAggregateId {
        self.id
    }

    /// The metric.
    #[must_use]
    pub fn metric(&self) -> Metric {
        self.metric
    }

    /// The period.
    #[must_use]
    pub fn period(&self) -> Period {
        self.period
    }

    /// The first instant of the period.
    #[must_use]
    pub fn period_start(&self) -> DateTime<Utc> {
        self.period_start
    }

    /// The non-identifying cohort label.
    #[must_use]
    pub fn cohort(&self) -> &str {
        &self.cohort
    }

    /// The ratio.
    #[must_use]
    pub fn value(&self) -> Decimal {
        self.value
    }

    /// When the figure was computed.
    #[must_use]
    pub fn computed_at(&self) -> DateTime<Utc> {
        self.computed_at
    }
}
