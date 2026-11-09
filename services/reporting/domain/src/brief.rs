//! The brief entity and its delivery record.
//!
//! BR1.1: one brief per user per period window. BR1.2: a brief carries the
//! period it covers and the instant its figures were read (`as_of`) together with
//! a data status. BR1.3: a missing figure stays absent rather than becoming zero.
//! BR2.1: a brief that cannot be produced is recorded as failed with an explicit
//! reason. BR2.2: every generated brief submits an email alert and Notification
//! decides whether to suppress it.

use chrono::{DateTime, Duration, Utc};

use crate::error::DomainError;
use crate::ids::{BriefDeliveryId, BriefId, UserId};

/// The largest failure reason preserved on a brief.
pub const MAX_FAILURE_REASON_CHARS: usize = 500;

/// The reporting period a brief covers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Period {
    /// One local day.
    Daily,
    /// One ISO week.
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
    /// Returns [`DomainError::Invalid`] for an unknown period.
    pub fn parse(raw: &str) -> Result<Self, DomainError> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "daily" => Ok(Self::Daily),
            "weekly" => Ok(Self::Weekly),
            _ => Err(DomainError::Invalid {
                field: "period",
                reason: "must be daily or weekly".to_string(),
            }),
        }
    }

    /// The longest window the period may cover, allowing for a daylight-saving
    /// shift on either edge.
    #[must_use]
    fn longest_window(self) -> Duration {
        match self {
            Self::Daily => Duration::hours(26),
            Self::Weekly => Duration::days(8),
        }
    }

    /// The shortest window the period may cover.
    #[must_use]
    fn shortest_window(self) -> Duration {
        match self {
            Self::Daily => Duration::hours(22),
            Self::Weekly => Duration::days(6),
        }
    }
}

/// What the figures in a brief could be computed from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum DataStatus {
    /// Every priced holding had a fresh price.
    Complete,
    /// Some figures could not be computed; they are absent, never zero.
    Partial,
    /// No figure could be computed.
    Unavailable,
}

impl DataStatus {
    /// The stored label.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Complete => "complete",
            Self::Partial => "partial",
            Self::Unavailable => "unavailable",
        }
    }

    /// Parses a stored label.
    ///
    /// # Errors
    /// Returns [`DomainError::Invalid`] for an unknown status.
    pub fn parse(raw: &str) -> Result<Self, DomainError> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "complete" => Ok(Self::Complete),
            "partial" => Ok(Self::Partial),
            "unavailable" => Ok(Self::Unavailable),
            _ => Err(DomainError::Invalid {
                field: "dataStatus",
                reason: "must be complete, partial, or unavailable".to_string(),
            }),
        }
    }

    /// The weaker of two statuses. BR1.2: a brief never claims better data than
    /// its weakest section.
    #[must_use]
    pub fn weakest(self, other: Self) -> Self {
        self.max(other)
    }
}

/// The state of a brief.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BriefStatus {
    /// Recorded but not yet produced.
    Pending,
    /// Produced and readable.
    Generated,
    /// Could not be produced; the reason is recorded.
    Failed,
}

impl BriefStatus {
    /// The stored label.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Generated => "generated",
            Self::Failed => "failed",
        }
    }

    /// Parses a stored label.
    ///
    /// # Errors
    /// Returns [`DomainError::Invalid`] for an unknown status.
    pub fn parse(raw: &str) -> Result<Self, DomainError> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "pending" => Ok(Self::Pending),
            "generated" => Ok(Self::Generated),
            "failed" => Ok(Self::Failed),
            _ => Err(DomainError::Invalid {
                field: "status",
                reason: "must be pending, generated, or failed".to_string(),
            }),
        }
    }
}

/// One brief for one user and one period window.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Brief {
    id: BriefId,
    user_id: UserId,
    period: Period,
    period_start: DateTime<Utc>,
    period_end: DateTime<Utc>,
    status: BriefStatus,
    failure_reason: Option<String>,
    as_of: Option<DateTime<Utc>>,
    data_status: Option<DataStatus>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl Brief {
    /// Records a pending brief. The window must fit its period, which tolerates
    /// a daylight-saving shift on either edge.
    ///
    /// # Errors
    /// Returns a validation refusal when the window is inverted or does not fit
    /// the period.
    pub fn new(
        id: BriefId,
        user_id: UserId,
        period: Period,
        period_start: DateTime<Utc>,
        period_end: DateTime<Utc>,
        now: DateTime<Utc>,
    ) -> Result<Self, DomainError> {
        let length = period_end.signed_duration_since(period_start);
        if length <= Duration::zero() {
            return Err(DomainError::Invalid {
                field: "periodEnd",
                reason: "must be after periodStart".to_string(),
            });
        }
        if length > period.longest_window() || length < period.shortest_window() {
            return Err(DomainError::Invalid {
                field: "periodEnd",
                reason: format!("does not fit a {} window", period.as_str()),
            });
        }
        Ok(Self {
            id,
            user_id,
            period,
            period_start,
            period_end,
            status: BriefStatus::Pending,
            failure_reason: None,
            as_of: None,
            data_status: None,
            created_at: now,
            updated_at: now,
        })
    }

    /// Rebuilds a brief from storage.
    ///
    /// # Errors
    /// Returns [`DomainError::Invalid`] when a stored row violates the state
    /// shape (a generated brief without an as-of instant, for example).
    #[allow(clippy::too_many_arguments)]
    pub fn restore(
        id: BriefId,
        user_id: UserId,
        period: Period,
        period_start: DateTime<Utc>,
        period_end: DateTime<Utc>,
        status: BriefStatus,
        failure_reason: Option<String>,
        as_of: Option<DateTime<Utc>>,
        data_status: Option<DataStatus>,
        created_at: DateTime<Utc>,
        updated_at: DateTime<Utc>,
    ) -> Result<Self, DomainError> {
        let shaped = match status {
            BriefStatus::Generated => as_of.is_some() && data_status.is_some(),
            BriefStatus::Failed => failure_reason
                .as_deref()
                .is_some_and(|reason| !reason.is_empty()),
            BriefStatus::Pending => true,
        };
        if !shaped {
            return Err(DomainError::Invalid {
                field: "status",
                reason: "does not match the stored brief fields".to_string(),
            });
        }
        Ok(Self {
            id,
            user_id,
            period,
            period_start,
            period_end,
            status,
            failure_reason,
            as_of,
            data_status,
            created_at,
            updated_at,
        })
    }

    /// Marks the brief generated with the instant its figures were read and their
    /// status (BR1.2).
    ///
    /// # Errors
    /// Returns [`DomainError::InvalidTransition`] when the brief is already
    /// generated.
    pub fn mark_generated(
        &mut self,
        as_of: DateTime<Utc>,
        data_status: DataStatus,
        now: DateTime<Utc>,
    ) -> Result<(), DomainError> {
        if self.status == BriefStatus::Generated {
            return Err(DomainError::InvalidTransition {
                entity: "brief",
                from: self.status.as_str(),
                to: BriefStatus::Generated.as_str(),
            });
        }
        self.status = BriefStatus::Generated;
        self.as_of = Some(as_of);
        self.data_status = Some(data_status);
        self.failure_reason = None;
        self.updated_at = now;
        Ok(())
    }

    /// Marks the brief failed with an explicit, bounded reason (BR2.1). The
    /// reason never carries a credential, token, or full address.
    ///
    /// # Errors
    /// Returns [`DomainError::Required`] for a blank reason and
    /// [`DomainError::TooLarge`] past the bound.
    pub fn mark_failed(&mut self, reason: &str, now: DateTime<Utc>) -> Result<(), DomainError> {
        let reason = reason.trim();
        if reason.is_empty() {
            return Err(DomainError::Required {
                field: "failureReason",
            });
        }
        if reason.chars().count() > MAX_FAILURE_REASON_CHARS {
            return Err(DomainError::TooLarge {
                field: "failureReason",
                limit: "500 characters",
            });
        }
        self.status = BriefStatus::Failed;
        self.failure_reason = Some(reason.to_string());
        self.as_of = None;
        self.data_status = None;
        self.updated_at = now;
        Ok(())
    }

    /// BR1.1: a failed or pending brief is retried, a generated one is not.
    #[must_use]
    pub fn is_retryable(&self) -> bool {
        self.status != BriefStatus::Generated
    }

    /// The brief identifier.
    #[must_use]
    pub fn id(&self) -> BriefId {
        self.id
    }

    /// The owning user.
    #[must_use]
    pub fn user_id(&self) -> UserId {
        self.user_id
    }

    /// The period.
    #[must_use]
    pub fn period(&self) -> Period {
        self.period
    }

    /// The first instant of the window.
    #[must_use]
    pub fn period_start(&self) -> DateTime<Utc> {
        self.period_start
    }

    /// The window's exclusive end.
    #[must_use]
    pub fn period_end(&self) -> DateTime<Utc> {
        self.period_end
    }

    /// The state.
    #[must_use]
    pub fn status(&self) -> BriefStatus {
        self.status
    }

    /// The failure reason, when the brief failed.
    #[must_use]
    pub fn failure_reason(&self) -> Option<&str> {
        self.failure_reason.as_deref()
    }

    /// The instant the figures were read.
    #[must_use]
    pub fn as_of(&self) -> Option<DateTime<Utc>> {
        self.as_of
    }

    /// The status of the figures.
    #[must_use]
    pub fn data_status(&self) -> Option<DataStatus> {
        self.data_status
    }

    /// When the record was created.
    #[must_use]
    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }

    /// When the record last changed.
    #[must_use]
    pub fn updated_at(&self) -> DateTime<Utc> {
        self.updated_at
    }
}

/// The channel a delivery used.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeliveryChannel {
    /// The in-app feed, read from this service.
    InApp,
    /// An email alert submitted to Notification.
    Email,
}

impl DeliveryChannel {
    /// The stored label.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::InApp => "in_app",
            Self::Email => "email",
        }
    }

    /// Parses a stored label.
    ///
    /// # Errors
    /// Returns [`DomainError::Invalid`] for an unknown channel.
    pub fn parse(raw: &str) -> Result<Self, DomainError> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "in_app" => Ok(Self::InApp),
            "email" => Ok(Self::Email),
            _ => Err(DomainError::Invalid {
                field: "channel",
                reason: "must be in_app or email".to_string(),
            }),
        }
    }
}

/// The outcome of a delivery.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeliveryStatus {
    /// Accepted by Notification.
    Submitted,
    /// Notification suppressed the alert; the user's preference was honoured.
    Suppressed,
    /// Notification could not accept the alert.
    Failed,
}

impl DeliveryStatus {
    /// The stored label.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Submitted => "submitted",
            Self::Suppressed => "suppressed",
            Self::Failed => "failed",
        }
    }

    /// Parses a stored label.
    ///
    /// # Errors
    /// Returns [`DomainError::Invalid`] for an unknown status.
    pub fn parse(raw: &str) -> Result<Self, DomainError> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "submitted" => Ok(Self::Submitted),
            "suppressed" => Ok(Self::Suppressed),
            "failed" => Ok(Self::Failed),
            _ => Err(DomainError::Invalid {
                field: "deliveryStatus",
                reason: "must be submitted, suppressed, or failed".to_string(),
            }),
        }
    }
}

/// One delivery attempt for one brief on one channel.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BriefDelivery {
    id: BriefDeliveryId,
    brief_id: BriefId,
    channel: DeliveryChannel,
    alert_id: Option<String>,
    status: DeliveryStatus,
    submitted_at: DateTime<Utc>,
}

impl BriefDelivery {
    /// Records a delivery attempt.
    ///
    /// # Errors
    /// Returns [`DomainError::Invalid`] when a submitted delivery has no alert
    /// identifier or a non-submitted delivery carries one.
    pub fn new(
        id: BriefDeliveryId,
        brief_id: BriefId,
        channel: DeliveryChannel,
        alert_id: Option<String>,
        status: DeliveryStatus,
        submitted_at: DateTime<Utc>,
    ) -> Result<Self, DomainError> {
        let shaped = match status {
            DeliveryStatus::Submitted => {
                alert_id.as_deref().is_some_and(|id| !id.trim().is_empty())
            }
            _ => alert_id.is_none(),
        };
        if !shaped {
            return Err(DomainError::Invalid {
                field: "alertId",
                reason: "does not match the delivery status".to_string(),
            });
        }
        Ok(Self {
            id,
            brief_id,
            channel,
            alert_id,
            status,
            submitted_at,
        })
    }

    /// The delivery identifier.
    #[must_use]
    pub fn id(&self) -> BriefDeliveryId {
        self.id
    }

    /// The brief the delivery belongs to.
    #[must_use]
    pub fn brief_id(&self) -> BriefId {
        self.brief_id
    }

    /// The channel.
    #[must_use]
    pub fn channel(&self) -> DeliveryChannel {
        self.channel
    }

    /// The C9 alert identifier, when Notification accepted it.
    #[must_use]
    pub fn alert_id(&self) -> Option<&str> {
        self.alert_id.as_deref()
    }

    /// The outcome.
    #[must_use]
    pub fn status(&self) -> DeliveryStatus {
        self.status
    }

    /// When the attempt was made.
    #[must_use]
    pub fn submitted_at(&self) -> DateTime<Utc> {
        self.submitted_at
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn instant(offset_hours: i64) -> DateTime<Utc> {
        DateTime::from_timestamp(1_800_000_000, 0).expect("a valid instant")
            + Duration::hours(offset_hours)
    }

    fn daily_brief(now: DateTime<Utc>) -> Brief {
        Brief::new(
            BriefId::generate(),
            UserId::generate(),
            Period::Daily,
            now,
            now + Duration::hours(24),
            now,
        )
        .expect("a valid daily window")
    }

    #[test]
    fn a_daily_window_is_accepted() {
        let now = instant(0);
        let brief = daily_brief(now);
        assert_eq!(brief.status(), BriefStatus::Pending);
        assert!(brief.is_retryable());
        assert_eq!(brief.period(), Period::Daily);
    }

    #[test]
    fn a_weekly_window_is_accepted() {
        let now = instant(0);
        let brief = Brief::new(
            BriefId::generate(),
            UserId::generate(),
            Period::Weekly,
            now,
            now + Duration::days(7),
            now,
        )
        .expect("a valid weekly window");
        assert_eq!(brief.period(), Period::Weekly);
    }

    #[test]
    fn an_inverted_window_is_refused() {
        let now = instant(0);
        let error = Brief::new(
            BriefId::generate(),
            UserId::generate(),
            Period::Daily,
            now,
            now - Duration::hours(1),
            now,
        )
        .expect_err("an inverted window is refused");
        assert_eq!(error.field(), "periodEnd");
    }

    #[test]
    fn a_daily_window_of_a_week_is_refused() {
        let now = instant(0);
        let error = Brief::new(
            BriefId::generate(),
            UserId::generate(),
            Period::Daily,
            now,
            now + Duration::days(7),
            now,
        )
        .expect_err("a week does not fit a daily brief");
        assert_eq!(error.field(), "periodEnd");
    }

    #[test]
    fn a_zero_length_weekly_window_is_refused() {
        let now = instant(0);
        let error = Brief::new(
            BriefId::generate(),
            UserId::generate(),
            Period::Weekly,
            now,
            now + Duration::hours(1),
            now,
        )
        .expect_err("an hour is not a week");
        assert_eq!(error.field(), "periodEnd");
    }

    #[test]
    fn marking_generated_records_the_as_of_and_status() {
        let now = instant(0);
        let mut brief = daily_brief(now);
        brief
            .mark_generated(now + Duration::hours(1), DataStatus::Partial, now)
            .expect("a pending brief can be generated");
        assert_eq!(brief.status(), BriefStatus::Generated);
        assert_eq!(brief.as_of(), Some(now + Duration::hours(1)));
        assert_eq!(brief.data_status(), Some(DataStatus::Partial));
        assert!(!brief.is_retryable());
    }

    #[test]
    fn generating_twice_is_refused() {
        let now = instant(0);
        let mut brief = daily_brief(now);
        brief
            .mark_generated(now, DataStatus::Complete, now)
            .expect("the first generation succeeds");
        let error = brief
            .mark_generated(now, DataStatus::Complete, now)
            .expect_err("the second generation is refused");
        assert_eq!(error.field(), "state");
    }

    #[test]
    fn a_failure_keeps_an_explicit_reason() {
        let now = instant(0);
        let mut brief = daily_brief(now);
        brief
            .mark_failed("portfolio valuation was unavailable", now)
            .expect("a pending brief can fail");
        assert_eq!(brief.status(), BriefStatus::Failed);
        assert_eq!(
            brief.failure_reason(),
            Some("portfolio valuation was unavailable")
        );
        assert!(brief.is_retryable());
    }

    #[test]
    fn a_blank_failure_reason_is_refused() {
        let now = instant(0);
        let mut brief = daily_brief(now);
        let error = brief
            .mark_failed("   ", now)
            .expect_err("a blank reason is refused");
        assert_eq!(error.field(), "failureReason");
    }

    #[test]
    fn an_overlong_failure_reason_is_refused() {
        let now = instant(0);
        let mut brief = daily_brief(now);
        let reason = "x".repeat(MAX_FAILURE_REASON_CHARS + 1);
        let error = brief
            .mark_failed(&reason, now)
            .expect_err("an overlong reason is refused");
        assert_eq!(error.field(), "failureReason");
    }

    #[test]
    fn a_failed_brief_can_be_retried_and_then_generated() {
        let now = instant(0);
        let mut brief = daily_brief(now);
        brief
            .mark_failed("upstream timeout", now)
            .expect("the failure is recorded");
        brief
            .mark_generated(now, DataStatus::Complete, now)
            .expect("a retry can generate it");
        assert_eq!(brief.failure_reason(), None);
        assert_eq!(brief.status(), BriefStatus::Generated);
    }

    #[test]
    fn restore_refuses_a_generated_row_without_its_figures() {
        let now = instant(0);
        let error = Brief::restore(
            BriefId::generate(),
            UserId::generate(),
            Period::Daily,
            now,
            now + Duration::hours(24),
            BriefStatus::Generated,
            None,
            None,
            None,
            now,
            now,
        )
        .expect_err("a generated row needs an as-of and a status");
        assert_eq!(error.field(), "status");
    }

    #[test]
    fn restore_refuses_a_failed_row_without_a_reason() {
        let now = instant(0);
        let error = Brief::restore(
            BriefId::generate(),
            UserId::generate(),
            Period::Daily,
            now,
            now + Duration::hours(24),
            BriefStatus::Failed,
            None,
            None,
            None,
            now,
            now,
        )
        .expect_err("a failed row needs a reason");
        assert_eq!(error.field(), "status");
    }

    #[test]
    fn restore_accepts_a_well_formed_generated_row() {
        let now = instant(0);
        let brief = Brief::restore(
            BriefId::generate(),
            UserId::generate(),
            Period::Weekly,
            now,
            now + Duration::days(7),
            BriefStatus::Generated,
            None,
            Some(now),
            Some(DataStatus::Complete),
            now,
            now,
        )
        .expect("the row is well formed");
        assert_eq!(brief.data_status(), Some(DataStatus::Complete));
    }

    #[test]
    fn the_weaker_data_status_wins() {
        assert_eq!(
            DataStatus::Complete.weakest(DataStatus::Unavailable),
            DataStatus::Unavailable
        );
        assert_eq!(
            DataStatus::Partial.weakest(DataStatus::Complete),
            DataStatus::Partial
        );
    }

    #[test]
    fn periods_and_statuses_round_trip_through_their_labels() {
        assert_eq!(Period::parse("weekly"), Ok(Period::Weekly));
        assert!(Period::parse("yearly").is_err());
        assert_eq!(DataStatus::parse("partial"), Ok(DataStatus::Partial));
        assert!(DataStatus::parse("unknown").is_err());
        assert_eq!(BriefStatus::parse("failed"), Ok(BriefStatus::Failed));
        assert!(BriefStatus::parse("done").is_err());
        assert_eq!(DeliveryChannel::parse("email"), Ok(DeliveryChannel::Email));
        assert!(DeliveryChannel::parse("sms").is_err());
        assert_eq!(
            DeliveryStatus::parse("suppressed"),
            Ok(DeliveryStatus::Suppressed)
        );
        assert!(DeliveryStatus::parse("queued").is_err());
    }

    #[test]
    fn a_submitted_delivery_needs_an_alert_id() {
        let now = instant(0);
        let error = BriefDelivery::new(
            BriefDeliveryId::generate(),
            BriefId::generate(),
            DeliveryChannel::Email,
            None,
            DeliveryStatus::Submitted,
            now,
        )
        .expect_err("a submitted delivery needs its alert id");
        assert_eq!(error.field(), "alertId");
    }

    #[test]
    fn a_suppressed_delivery_carries_no_alert_id() {
        let now = instant(0);
        let delivery = BriefDelivery::new(
            BriefDeliveryId::generate(),
            BriefId::generate(),
            DeliveryChannel::Email,
            None,
            DeliveryStatus::Suppressed,
            now,
        )
        .expect("a suppressed delivery has no alert id");
        assert_eq!(delivery.status(), DeliveryStatus::Suppressed);
        assert_eq!(delivery.channel(), DeliveryChannel::Email);
    }

    #[test]
    fn a_suppressed_delivery_with_an_alert_id_is_refused() {
        let now = instant(0);
        let error = BriefDelivery::new(
            BriefDeliveryId::generate(),
            BriefId::generate(),
            DeliveryChannel::Email,
            Some("alert-1".to_string()),
            DeliveryStatus::Suppressed,
            now,
        )
        .expect_err("a suppressed delivery cannot carry an alert id");
        assert_eq!(error.field(), "alertId");
    }

    #[test]
    fn a_submitted_delivery_records_its_alert_id() {
        let now = instant(0);
        let delivery = BriefDelivery::new(
            BriefDeliveryId::generate(),
            BriefId::generate(),
            DeliveryChannel::Email,
            Some("alert-9".to_string()),
            DeliveryStatus::Submitted,
            now,
        )
        .expect("a submitted delivery carries its alert id");
        assert_eq!(delivery.alert_id(), Some("alert-9"));
    }

    #[test]
    fn a_blank_alert_id_is_refused_on_submission() {
        let now = instant(0);
        let error = BriefDelivery::new(
            BriefDeliveryId::generate(),
            BriefId::generate(),
            DeliveryChannel::Email,
            Some("   ".to_string()),
            DeliveryStatus::Submitted,
            now,
        )
        .expect_err("a blank alert id is not an identifier");
        assert_eq!(error.field(), "alertId");
    }
}
