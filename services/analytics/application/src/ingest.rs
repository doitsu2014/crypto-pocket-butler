//! WF1: consent-gated, PII-free ingest (BR1.1–BR1.3, BR2.1–BR2.3).

use std::collections::BTreeMap;
use std::sync::Arc;

use analytics_domain::audit::RefusalKind;
use analytics_domain::{
    sanitize_event, AnalyticsEvent, AnalyticsEventId, ConsentCheck, ConsentCheckId, ConsentPurpose,
    EventClass,
};
use chrono::{DateTime, Utc};

use crate::common::FieldError;
use crate::hashing::{hmac_sha256_hex, sha256_hex};
use crate::ports::{
    Clock, ConsentChecker, EventRepository, ExternalError, RepositoryError, SaltProvider,
};
use crate::trail::Trail;

/// One raw ingest request, as the boundary received it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IngestCommand {
    /// The catalogue event name.
    pub event_name: String,
    /// When the event happened.
    pub occurred_at: DateTime<Utc>,
    /// The caller-supplied properties.
    pub properties: BTreeMap<String, String>,
    /// The validated JWT subject; held only for consent and hashing.
    pub subject: String,
    /// The request correlation id, for the audit record.
    pub correlation_id: Option<String>,
}

/// A stored event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IngestResult {
    /// The event identifier.
    pub event_id: AnalyticsEventId,
    /// The salted user hash.
    pub user_ref: String,
}

/// Why an ingest was refused.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum IngestError {
    /// The event failed the catalogue or PII rules; nothing was stored.
    #[error("the event is invalid")]
    Rejected(Vec<FieldError>),
    /// Consent was absent or withdrawn; nothing was stored.
    #[error("analytics consent was not granted")]
    ConsentNotGranted,
    /// The consent service could not answer; the request fails closed.
    #[error("the consent service is unavailable: {0}")]
    ConsentUnavailable(String),
    /// The salt could not be read; the request fails closed.
    #[error("the pseudonymisation salt is unavailable: {0}")]
    SaltUnavailable(String),
    /// A storage failure.
    #[error("the event could not be stored: {0}")]
    Repository(#[from] RepositoryError),
}

/// Validates, consent-gates, pseudonymises, and stores one event.
pub struct IngestEvent {
    events: Arc<dyn EventRepository>,
    consent_checker: Arc<dyn ConsentChecker>,
    salts: Arc<dyn SaltProvider>,
    trail: Arc<Trail>,
    clock: Arc<dyn Clock>,
}

impl IngestEvent {
    /// Creates the use case.
    #[must_use]
    pub fn new(
        events: Arc<dyn EventRepository>,
        consent_checker: Arc<dyn ConsentChecker>,
        salts: Arc<dyn SaltProvider>,
        trail: Arc<Trail>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self {
            events,
            consent_checker,
            salts,
            trail,
            clock,
        }
    }

    /// Executes one ingest.
    ///
    /// # Errors
    /// Returns [`IngestError`] when the event is invalid, consent is not
    /// granted, a dependency is unavailable, or storage fails.
    pub async fn execute(&self, command: IngestCommand) -> Result<IngestResult, IngestError> {
        // A high-entropy subject hash lets a refusal be audited before the salt
        // is available; it never leaves this function except as a pseudonymous
        // actor id inside the trail writer.
        let subject_digest = sha256_hex(&command.subject);
        let sanitized =
            match sanitize_event(&command.event_name, command.occurred_at, command.properties) {
                Ok(sanitized) => sanitized,
                Err(error) => {
                    self.trail
                        .refused_ingest(
                            RefusalKind::EventRejected,
                            &subject_digest,
                            command.correlation_id.as_deref(),
                        )
                        .await;
                    return Err(IngestError::Rejected(vec![FieldError::from(error)]));
                }
            };

        let salt = match self.salts.current().await {
            Ok(salt) => salt,
            Err(error) => {
                self.trail
                    .refused_ingest(
                        RefusalKind::SaltUnavailable,
                        &subject_digest,
                        command.correlation_id.as_deref(),
                    )
                    .await;
                return Err(IngestError::SaltUnavailable(error.to_string()));
            }
        };
        let user_ref = hmac_sha256_hex(&salt.salt, &command.subject);
        let now = self.clock.now();

        let consent = match sanitized.class() {
            EventClass::Essential => None,
            EventClass::Analytics => {
                match self
                    .consent_checker
                    .analytics_granted(&command.subject)
                    .await
                {
                    Ok(true) => Some(ConsentCheck::record(
                        ConsentCheckId::generate(),
                        ConsentPurpose::Analytics,
                        true,
                        now,
                        Some(user_ref.clone()),
                    )),
                    Ok(false) => {
                        // BR1.1: a refusal stores nothing, not even a negative
                        // ConsentCheck row. The refusal metric and the audit
                        // record carry the evidence.
                        self.trail
                            .refused_ingest(
                                RefusalKind::ConsentNotGranted,
                                &user_ref,
                                command.correlation_id.as_deref(),
                            )
                            .await;
                        return Err(IngestError::ConsentNotGranted);
                    }
                    Err(error) => {
                        self.trail
                            .refused_ingest(
                                refusal_kind(&error),
                                &user_ref,
                                command.correlation_id.as_deref(),
                            )
                            .await;
                        return Err(IngestError::ConsentUnavailable(error.to_string()));
                    }
                }
            }
        };

        let event = AnalyticsEvent::store(
            AnalyticsEventId::generate(),
            sanitized,
            Some(user_ref.clone()),
            now,
        );
        self.events.insert(&event, consent.as_ref()).await?;
        Ok(IngestResult {
            event_id: event.id(),
            user_ref,
        })
    }
}

fn refusal_kind(error: &ExternalError) -> RefusalKind {
    match error {
        ExternalError::Unavailable(_) => RefusalKind::ConsentUnavailable,
        ExternalError::Refused(_) => RefusalKind::ConsentNotGranted,
    }
}
