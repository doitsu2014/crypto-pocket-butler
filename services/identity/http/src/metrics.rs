//! Minimal Prometheus text metrics: request classes by route, error rate, the
//! workflow status counts the runbook alerts read, and the `authorisation_*`
//! family the decision emits.
//!
//! The `support_*` family serves the support-access rules (BR3.9–BR3.13,
//! FR16.3–FR16.6) and the monitoring design (NFR1.8, NFR17.1): every series is
//! rendered zero-initialised so an alert query answers before the first event.

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicI64, AtomicU64, Ordering};
use std::sync::Mutex;
use std::time::Duration;

use identity_application::account_lifecycle::AccountObservation;
use identity_application::authorisation::ports::{
    AuditWriteContext, ObservationSink, StatusFailure,
};
use identity_application::consent::ports::{
    DeliveryOutcome, DrainerObservation, ObservationSink as ConsentObservationSink,
    PreferenceWriteOutcome, StateQueryOutcome, WithdrawalRollbackCause,
};
use identity_application::ports::{
    SupportDrainDelivery, SupportDrainObservation, SupportSourceFailureSink,
};
use identity_domain::{ConsentAction, ConsentKind, DenialReason, OperationClass, UserStatus};

/// The upper bounds of the decision-duration histogram.
const DURATION_BUCKETS: [f64; 6] = [0.005, 0.01, 0.03, 0.1, 0.5, 1.0];

#[derive(Debug, Default, Clone)]
struct DurationSamples {
    buckets: [u64; 7],
    sum_seconds: f64,
    count: u64,
}

impl DurationSamples {
    fn observe(&mut self, elapsed: Duration) {
        let seconds = elapsed.as_secs_f64();
        self.sum_seconds += seconds;
        self.count += 1;
        let mut placed = false;
        for (index, bound) in DURATION_BUCKETS.iter().enumerate() {
            if seconds <= *bound {
                self.buckets[index] += 1;
                placed = true;
                break;
            }
        }
        if !placed {
            self.buckets[DURATION_BUCKETS.len()] += 1;
        }
    }
}

/// Request and workflow counters exposed at `/metrics`.
#[derive(Debug, Default)]
pub struct Metrics {
    requests: Mutex<BTreeMap<(String, String), u64>>,
    total_2xx: AtomicU64,
    total_3xx: AtomicU64,
    total_4xx: AtomicU64,
    total_5xx: AtomicU64,
    errors_total: AtomicU64,
    export_pending: AtomicU64,
    export_building: AtomicU64,
    export_ready: AtomicU64,
    export_failed: AtomicU64,
    export_expired: AtomicU64,
    deletion_requested: AtomicU64,
    deletion_in_progress: AtomicU64,
    deletion_completed: AtomicU64,
    deletion_failed: AtomicU64,
    jurisdiction_allowed: AtomicU64,
    jurisdiction_denied: AtomicU64,
    jurisdiction_errors: AtomicU64,
    principal_pending: AtomicU64,
    principal_active: AtomicU64,
    principal_disabled: AtomicU64,
    principal_pending_deletion: AtomicU64,
    principal_deleted: AtomicU64,
    authorisation_decisions: Mutex<BTreeMap<(String, String), u64>>,
    authorisation_durations: Mutex<BTreeMap<String, DurationSamples>>,
    authorisation_status_failures: Mutex<BTreeMap<String, u64>>,
    authorisation_audit_failures: Mutex<BTreeMap<String, u64>>,
    authorisation_role_changes: Mutex<BTreeMap<String, u64>>,
    authorisation_cache_age: AtomicI64,
    authorisation_cache_entries: AtomicI64,
    consent_state_queries: Mutex<BTreeMap<String, u64>>,
    consent_state_with_missing: Mutex<BTreeMap<String, u64>>,
    consent_state_durations: Mutex<DurationSamples>,
    consent_events: Mutex<BTreeMap<(String, String), u64>>,
    preference_writes: Mutex<BTreeMap<String, u64>>,
    consent_withdrawal_rollbacks: Mutex<BTreeMap<String, u64>>,
    consent_outbox_deliveries: Mutex<BTreeMap<String, u64>>,
    consent_outbox_pending: AtomicI64,
    consent_outbox_oldest_age: AtomicI64,
    consent_outbox_last_pass: AtomicI64,
    status_outbox_deliveries: Mutex<BTreeMap<String, u64>>,
    status_outbox_pending: AtomicI64,
    status_outbox_oldest_age: AtomicI64,
    support_grants: Mutex<BTreeMap<(String, String), u64>>,
    support_reads: Mutex<BTreeMap<String, u64>>,
    support_read_durations: Mutex<DurationSamples>,
    support_write_durations: Mutex<DurationSamples>,
    support_metadata_source_errors: Mutex<BTreeMap<(String, String), u64>>,
    support_audit_write_failures: Mutex<BTreeMap<String, u64>>,
    support_grant_decision_errors: Mutex<BTreeMap<String, u64>>,
    support_outbox_deliveries: Mutex<BTreeMap<String, u64>>,
    support_outbox_pending: AtomicI64,
    support_outbox_oldest_age: AtomicI64,
    support_outbox_last_pass: AtomicI64,
    status_transitions: Mutex<BTreeMap<(String, String, String), u64>>,
    status_refusals: Mutex<BTreeMap<String, u64>>,
    deletion_events: Mutex<BTreeMap<String, u64>>,
    erasure_failures: Mutex<BTreeMap<String, u64>>,
}

impl Metrics {
    /// Records one response status under the `other` route.
    pub fn record(&self, status: u16) {
        self.record_routed(status, "other");
    }

    /// Records one response status and the route it was served on.
    pub fn record_routed(&self, status: u16, route: &str) {
        let class = match status {
            200..=299 => {
                self.total_2xx.fetch_add(1, Ordering::Relaxed);
                "2xx"
            }
            300..=399 => {
                self.total_3xx.fetch_add(1, Ordering::Relaxed);
                "3xx"
            }
            400..=499 => {
                self.total_4xx.fetch_add(1, Ordering::Relaxed);
                "4xx"
            }
            _ => {
                self.total_5xx.fetch_add(1, Ordering::Relaxed);
                self.errors_total.fetch_add(1, Ordering::Relaxed);
                "5xx"
            }
        };
        let mut requests = self
            .requests
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        *requests
            .entry((class.to_string(), route.to_string()))
            .or_default() += 1;
    }

    /// Records one observed export status.
    pub fn record_export_status(&self, status: &str) {
        let counter = match status {
            "pending" => &self.export_pending,
            "building" => &self.export_building,
            "ready" => &self.export_ready,
            "failed" => &self.export_failed,
            "expired" => &self.export_expired,
            _ => return,
        };
        counter.fetch_add(1, Ordering::Relaxed);
    }

    /// Records one observed deletion status.
    pub fn record_deletion_status(&self, status: &str) {
        let counter = match status {
            "requested" => &self.deletion_requested,
            "in_progress" => &self.deletion_in_progress,
            "completed" => &self.deletion_completed,
            "failed" => &self.deletion_failed,
            _ => return,
        };
        counter.fetch_add(1, Ordering::Relaxed);
    }

    /// Records one jurisdiction check outcome: `allowed`, `denied`, or `error`.
    pub fn record_jurisdiction_outcome(&self, outcome: &str) {
        let counter = match outcome {
            "allowed" => &self.jurisdiction_allowed,
            "denied" => &self.jurisdiction_denied,
            "error" => &self.jurisdiction_errors,
            _ => return,
        };
        counter.fetch_add(1, Ordering::Relaxed);
    }

    /// Records one observed principal status.
    pub fn record_principal_status(&self, status: &str) {
        let counter = match status {
            "pending" => &self.principal_pending,
            "active" => &self.principal_active,
            "disabled" => &self.principal_disabled,
            "pending_deletion" => &self.principal_pending_deletion,
            "deleted" => &self.principal_deleted,
            _ => return,
        };
        counter.fetch_add(1, Ordering::Relaxed);
    }

    /// Total 5xx responses observed.
    #[must_use]
    pub fn errors_total(&self) -> u64 {
        self.errors_total.load(Ordering::Relaxed)
    }

    /// Renders the Prometheus text exposition.
    #[must_use]
    pub fn render(&self, service: &str) -> String {
        let classes = [
            ("2xx", self.total_2xx.load(Ordering::Relaxed)),
            ("3xx", self.total_3xx.load(Ordering::Relaxed)),
            ("4xx", self.total_4xx.load(Ordering::Relaxed)),
            ("5xx", self.total_5xx.load(Ordering::Relaxed)),
        ];
        let mut output = String::from(
            "# HELP http_requests_total Total requests handled by the service.\n\
             # TYPE http_requests_total counter\n",
        );
        let requests = self
            .requests
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone();
        for (class, value) in classes {
            // The aggregate series keeps dashboards that do not select a route
            // working, and the named routes isolate the C4 and U4 endpoints.
            let labelled: BTreeMap<String, u64> = requests
                .iter()
                .filter(|((recorded_class, route), _)| recorded_class == class && route != "other")
                .map(|((_, route), count)| (route.clone(), *count))
                .collect();
            let labelled_total: u64 = labelled.values().sum();
            output.push_str(&format!(
                "http_requests_total{{service=\"{service}\",status_class=\"{class}\",route=\"other\"}} {}\n",
                value.saturating_sub(labelled_total)
            ));
            for (route, count) in labelled {
                output.push_str(&format!(
                    "http_requests_total{{service=\"{service}\",status_class=\"{class}\",route=\"{route}\"}} {count}\n"
                ));
            }
        }
        output.push_str(
            "# HELP identity_request_errors_total Total 5xx responses.\n\
             # TYPE identity_request_errors_total counter\n",
        );
        output.push_str(&format!(
            "identity_request_errors_total{{service=\"{service}\"}} {}\n",
            self.errors_total.load(Ordering::Relaxed)
        ));
        let exports = [
            ("pending", self.export_pending.load(Ordering::Relaxed)),
            ("building", self.export_building.load(Ordering::Relaxed)),
            ("ready", self.export_ready.load(Ordering::Relaxed)),
            ("failed", self.export_failed.load(Ordering::Relaxed)),
            ("expired", self.export_expired.load(Ordering::Relaxed)),
        ];
        output.push_str(
            "# HELP identity_export_status_total Export requests observed, by status.\n\
             # TYPE identity_export_status_total counter\n",
        );
        for (status, value) in exports {
            output.push_str(&format!(
                "identity_export_status_total{{status=\"{status}\"}} {value}\n"
            ));
        }
        let deletions = [
            ("requested", self.deletion_requested.load(Ordering::Relaxed)),
            (
                "in_progress",
                self.deletion_in_progress.load(Ordering::Relaxed),
            ),
            ("completed", self.deletion_completed.load(Ordering::Relaxed)),
            ("failed", self.deletion_failed.load(Ordering::Relaxed)),
        ];
        output.push_str(
            "# HELP identity_deletion_status_total Deletion requests observed, by status.\n\
             # TYPE identity_deletion_status_total counter\n",
        );
        for (status, value) in deletions {
            output.push_str(&format!(
                "identity_deletion_status_total{{status=\"{status}\"}} {value}\n"
            ));
        }
        output.push_str(
            "# HELP jurisdiction_checks_total Jurisdiction checks, by outcome.\n\
             # TYPE jurisdiction_checks_total counter\n",
        );
        let jurisdiction = [
            ("allowed", self.jurisdiction_allowed.load(Ordering::Relaxed)),
            ("denied", self.jurisdiction_denied.load(Ordering::Relaxed)),
            ("error", self.jurisdiction_errors.load(Ordering::Relaxed)),
        ];
        for (outcome, value) in jurisdiction {
            output.push_str(&format!(
                "jurisdiction_checks_total{{outcome=\"{outcome}\"}} {value}\n"
            ));
        }
        let principal = [
            ("pending", self.principal_pending.load(Ordering::Relaxed)),
            ("active", self.principal_active.load(Ordering::Relaxed)),
            ("disabled", self.principal_disabled.load(Ordering::Relaxed)),
            (
                "pending_deletion",
                self.principal_pending_deletion.load(Ordering::Relaxed),
            ),
            ("deleted", self.principal_deleted.load(Ordering::Relaxed)),
        ];
        output.push_str(
            "# HELP identity_principal_status_total Principals resolved, by status.\n\
             # TYPE identity_principal_status_total counter\n",
        );
        for (status, value) in principal {
            output.push_str(&format!(
                "identity_principal_status_total{{status=\"{status}\"}} {value}\n"
            ));
        }

        output.push_str(
            "# HELP authorisation_decisions_total Decisions, by operation class, outcome, and reason.\n\
             # TYPE authorisation_decisions_total counter\n",
        );
        let decisions = self
            .authorisation_decisions
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone();
        if decisions.is_empty() {
            output.push_str(
                "authorisation_decisions_total{operation_class=\"undeclared\",outcome=\"denied\",reason=\"not_declared\"} 0\n",
            );
        }
        for ((class, reason), value) in decisions {
            let outcome = if reason == "allowed" {
                "allowed"
            } else {
                "denied"
            };
            output.push_str(&format!(
                "authorisation_decisions_total{{operation_class=\"{class}\",outcome=\"{outcome}\",reason=\"{reason}\"}} {value}\n"
            ));
        }

        output.push_str(
            "# HELP authorisation_decision_duration_seconds Decision duration.\n\
             # TYPE authorisation_decision_duration_seconds histogram\n",
        );
        let durations = self
            .authorisation_durations
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone();
        for (path, samples) in &durations {
            let mut cumulative = 0;
            for (index, bound) in DURATION_BUCKETS.iter().enumerate() {
                cumulative += samples.buckets[index];
                output.push_str(&format!(
                    "authorisation_decision_duration_seconds_bucket{{path=\"{path}\",le=\"{bound}\"}} {cumulative}\n"
                ));
            }
            cumulative += samples.buckets[DURATION_BUCKETS.len()];
            output.push_str(&format!(
                "authorisation_decision_duration_seconds_bucket{{path=\"{path}\",le=\"+Inf\"}} {cumulative}\n"
            ));
            output.push_str(&format!(
                "authorisation_decision_duration_seconds_sum{{path=\"{path}\"}} {}\n",
                samples.sum_seconds
            ));
            output.push_str(&format!(
                "authorisation_decision_duration_seconds_count{{path=\"{path}\"}} {}\n",
                samples.count
            ));
        }

        push_counter_family(
            &mut output,
            "authorisation_status_lookup_errors_total",
            "Failed status reads, by cause.",
            &self.authorisation_status_failures,
            "cause",
        );
        push_counter_family(
            &mut output,
            "authorisation_audit_write_failures_total",
            "Failed audit writes, by context.",
            &self.authorisation_audit_failures,
            "context",
        );
        push_counter_family(
            &mut output,
            "authorisation_role_changes_total",
            "Role changes, by outcome.",
            &self.authorisation_role_changes,
            "outcome",
        );

        output.push_str(
            "# HELP authorisation_cache_age_seconds The age of the newest outage-cache entry read.\n\
             # TYPE authorisation_cache_age_seconds gauge\n",
        );
        output.push_str(&format!(
            "authorisation_cache_age_seconds {}\n",
            self.authorisation_cache_age.load(Ordering::Relaxed)
        ));
        output.push_str(
            "# HELP authorisation_cache_entries Entries held by the outage cache.\n\
             # TYPE authorisation_cache_entries gauge\n",
        );
        output.push_str(&format!(
            "authorisation_cache_entries {}\n",
            self.authorisation_cache_entries.load(Ordering::Relaxed)
        ));

        push_counter_family(
            &mut output,
            "consent_state_queries_total",
            "Consent-state queries, by outcome.",
            &self.consent_state_queries,
            "outcome",
        );
        push_counter_family(
            &mut output,
            "consent_state_queries_with_missing_total",
            "Consent-state queries, by whether a required acceptance is missing.",
            &self.consent_state_with_missing,
            "has_missing",
        );
        output.push_str(
            "# HELP consent_state_duration_seconds Consent-state query duration.\n\
             # TYPE consent_state_duration_seconds histogram\n",
        );
        let state_durations = self
            .consent_state_durations
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone();
        let mut cumulative = 0;
        for (index, bound) in DURATION_BUCKETS.iter().enumerate() {
            cumulative += state_durations.buckets[index];
            output.push_str(&format!(
                "consent_state_duration_seconds_bucket{{le=\"{bound}\"}} {cumulative}\n"
            ));
        }
        cumulative += state_durations.buckets[DURATION_BUCKETS.len()];
        output.push_str(&format!(
            "consent_state_duration_seconds_bucket{{le=\"+Inf\"}} {cumulative}\n"
        ));
        output.push_str(&format!(
            "consent_state_duration_seconds_sum {}\n",
            state_durations.sum_seconds
        ));
        output.push_str(&format!(
            "consent_state_duration_seconds_count {}\n",
            state_durations.count
        ));

        output.push_str(
            "# HELP consent_events_total Consent events recorded, by kind and action.\n\
             # TYPE consent_events_total counter\n",
        );
        let events = self
            .consent_events
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone();
        if events.is_empty() {
            output.push_str("consent_events_total{kind=\"none\",action=\"none\"} 0\n");
        }
        for ((kind, action), value) in events {
            output.push_str(&format!(
                "consent_events_total{{kind=\"{kind}\",action=\"{action}\"}} {value}\n"
            ));
        }

        push_counter_family(
            &mut output,
            "preference_writes_total",
            "Preference writes, by outcome.",
            &self.preference_writes,
            "outcome",
        );
        push_counter_family(
            &mut output,
            "consent_withdrawal_rollbacks_total",
            "Withdrawal rollbacks, by cause.",
            &self.consent_withdrawal_rollbacks,
            "cause",
        );
        push_counter_family(
            &mut output,
            "consent_outbox_deliveries_total",
            "Audit-outbox deliveries, by outcome.",
            &self.consent_outbox_deliveries,
            "outcome",
        );

        output.push_str(
            "# HELP consent_outbox_pending Undelivered audit-outbox rows.\n\
             # TYPE consent_outbox_pending gauge\n",
        );
        output.push_str(&format!(
            "consent_outbox_pending {}\n",
            self.consent_outbox_pending.load(Ordering::Relaxed)
        ));
        output.push_str(
            "# HELP consent_outbox_oldest_age_seconds The age of the oldest pending outbox row.\n\
             # TYPE consent_outbox_oldest_age_seconds gauge\n",
        );
        output.push_str(&format!(
            "consent_outbox_oldest_age_seconds {}\n",
            self.consent_outbox_oldest_age.load(Ordering::Relaxed)
        ));
        output.push_str(
            "# HELP consent_outbox_last_pass_timestamp_seconds When the drainer last finished a pass.\n\
             # TYPE consent_outbox_last_pass_timestamp_seconds gauge\n",
        );
        output.push_str(&format!(
            "consent_outbox_last_pass_timestamp_seconds {}\n",
            self.consent_outbox_last_pass.load(Ordering::Relaxed)
        ));

        // U5: the account lifecycle (observability design).
        output.push_str(
            "# HELP account_status_transitions_total Accepted status transitions, by edge.\n\
             # TYPE account_status_transitions_total counter\n",
        );
        let transitions = self
            .status_transitions
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone();
        for ((from, to, actor_kind), count) in transitions {
            output.push_str(&format!(
                "account_status_transitions_total{{from=\"{from}\",to=\"{to}\",actor_kind=\"{actor_kind}\"}} {count}\n"
            ));
        }
        push_counter_family(
            &mut output,
            "account_transition_refusals_total",
            "Refused status transitions, by reason.",
            &self.status_refusals,
            "reason",
        );
        push_counter_family(
            &mut output,
            "deletion_requests_total",
            "Deletion request lifecycle events.",
            &self.deletion_events,
            "event",
        );
        push_counter_family(
            &mut output,
            "deletion_erasure_failures_total",
            "Failed erasure steps, by step.",
            &self.erasure_failures,
            "step",
        );
        push_counter_family(
            &mut output,
            "status_audit_outbox_deliveries_total",
            "Status audit deliveries, by outcome.",
            &self.status_outbox_deliveries,
            "outcome",
        );
        output.push_str(
            "# HELP status_audit_outbox_pending Undelivered status audit rows.\n\
             # TYPE status_audit_outbox_pending gauge\n",
        );
        output.push_str(&format!(
            "status_audit_outbox_pending {}\n",
            self.status_outbox_pending.load(Ordering::Relaxed)
        ));
        output.push_str(
            "# HELP status_audit_outbox_oldest_age_seconds The age of the oldest pending status audit row.\n\
             # TYPE status_audit_outbox_oldest_age_seconds gauge\n",
        );
        output.push_str(&format!(
            "status_audit_outbox_oldest_age_seconds {}\n",
            self.status_outbox_oldest_age.load(Ordering::Relaxed)
        ));

        // U3: support access (monitoring design, NFR1.8, NFR17.1). Every series
        // is rendered zero-initialised so a dashboard query answers before the
        // first event, and so an absent series means a misconfiguration rather
        // than silence.
        push_counter_family_seeded(
            &mut output,
            "support_grants_total",
            "Support-grant writes, by operation and outcome.",
            &self.support_grants,
            ("operation", "outcome"),
            &[
                ("request", "accepted"),
                ("request", "refused"),
                ("request", "failed"),
                ("approve", "accepted"),
                ("approve", "refused"),
                ("approve", "failed"),
                ("revoke", "accepted"),
                ("revoke", "refused"),
                ("revoke", "failed"),
            ],
        );
        push_counter_family_with_zeroes(
            &mut output,
            "support_reads_total",
            "Support-metadata reads, by outcome.",
            &self.support_reads,
            "outcome",
            &["served", "refused", "unavailable", "failed"],
        );
        output.push_str(
            "# HELP support_read_duration_seconds Support-metadata read duration.\n\
             # TYPE support_read_duration_seconds histogram\n",
        );
        let read_durations = self
            .support_read_durations
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone();
        if read_durations.count == 0 {
            for bound in DURATION_BUCKETS {
                output.push_str(&format!(
                    "support_read_duration_seconds_bucket{{le=\"{bound}\"}} 0\n"
                ));
            }
            output.push_str("support_read_duration_seconds_bucket{le=\"+Inf\"} 0\n");
            output.push_str("support_read_duration_seconds_sum 0\n");
            output.push_str("support_read_duration_seconds_count 0\n");
        } else {
            let mut cumulative = 0;
            for (index, bound) in DURATION_BUCKETS.iter().enumerate() {
                cumulative += read_durations.buckets[index];
                output.push_str(&format!(
                    "support_read_duration_seconds_bucket{{le=\"{bound}\"}} {cumulative}\n"
                ));
            }
            cumulative += read_durations.buckets[DURATION_BUCKETS.len()];
            output.push_str(&format!(
                "support_read_duration_seconds_bucket{{le=\"+Inf\"}} {cumulative}\n"
            ));
            output.push_str(&format!(
                "support_read_duration_seconds_sum {}\n",
                read_durations.sum_seconds
            ));
            output.push_str(&format!(
                "support_read_duration_seconds_count {}\n",
                read_durations.count
            ));
        }
        output.push_str(
            "# HELP support_write_duration_seconds Support-grant write duration.\n\
             # TYPE support_write_duration_seconds histogram\n",
        );
        let write_durations = self
            .support_write_durations
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone();
        if write_durations.count == 0 {
            for bound in DURATION_BUCKETS {
                output.push_str(&format!(
                    "support_write_duration_seconds_bucket{{le=\"{bound}\"}} 0\n"
                ));
            }
            output.push_str("support_write_duration_seconds_bucket{le=\"+Inf\"} 0\n");
            output.push_str("support_write_duration_seconds_sum 0\n");
            output.push_str("support_write_duration_seconds_count 0\n");
        } else {
            let mut cumulative = 0;
            for (index, bound) in DURATION_BUCKETS.iter().enumerate() {
                cumulative += write_durations.buckets[index];
                output.push_str(&format!(
                    "support_write_duration_seconds_bucket{{le=\"{bound}\"}} {cumulative}\n"
                ));
            }
            cumulative += write_durations.buckets[DURATION_BUCKETS.len()];
            output.push_str(&format!(
                "support_write_duration_seconds_bucket{{le=\"+Inf\"}} {cumulative}\n"
            ));
            output.push_str(&format!(
                "support_write_duration_seconds_sum {}\n",
                write_durations.sum_seconds
            ));
            output.push_str(&format!(
                "support_write_duration_seconds_count {}\n",
                write_durations.count
            ));
        }
        push_counter_family_seeded(
            &mut output,
            "support_metadata_source_errors_total",
            "Owner metadata source failures, by source and cause.",
            &self.support_metadata_source_errors,
            ("source", "cause"),
            &[
                ("account", "unreachable"),
                ("account", "timeout"),
                ("portfolio", "unreachable"),
                ("portfolio", "timeout"),
            ],
        );
        push_counter_family_with_zeroes(
            &mut output,
            "support_audit_write_failures_total",
            "Fail-closed support audit appends, by cause.",
            &self.support_audit_write_failures,
            "cause",
            &["read"],
        );
        push_counter_family_with_zeroes(
            &mut output,
            "support_grant_decision_errors_total",
            "Failing grant lookups inside the support read, by cause.",
            &self.support_grant_decision_errors,
            "cause",
            &["lookup"],
        );
        push_counter_family_with_zeroes(
            &mut output,
            "support_outbox_deliveries_total",
            "Support audit-outbox deliveries, by outcome.",
            &self.support_outbox_deliveries,
            "outcome",
            &["delivered", "retryable", "dead"],
        );
        output.push_str(
            "# HELP support_outbox_pending Undelivered support audit rows.\n\
             # TYPE support_outbox_pending gauge\n",
        );
        output.push_str(&format!(
            "support_outbox_pending {}\n",
            self.support_outbox_pending.load(Ordering::Relaxed)
        ));
        output.push_str(
            "# HELP support_outbox_oldest_age_seconds The age of the oldest pending support audit row.\n\
             # TYPE support_outbox_oldest_age_seconds gauge\n",
        );
        output.push_str(&format!(
            "support_outbox_oldest_age_seconds {}\n",
            self.support_outbox_oldest_age.load(Ordering::Relaxed)
        ));
        output.push_str(
            "# HELP support_outbox_last_pass_timestamp_seconds When the support drainer last finished a pass.\n\
             # TYPE support_outbox_last_pass_timestamp_seconds gauge\n",
        );
        output.push_str(&format!(
            "support_outbox_last_pass_timestamp_seconds {}\n",
            self.support_outbox_last_pass.load(Ordering::Relaxed)
        ));

        output.push_str(
            "# HELP service_up 1 while the process is running.\n\
             # TYPE service_up gauge\n",
        );
        output.push_str(&format!("service_up{{service=\"{service}\"}} 1\n"));
        output
    }
}

fn push_counter_family(
    output: &mut String,
    name: &str,
    help: &str,
    counters: &Mutex<BTreeMap<String, u64>>,
    label: &str,
) {
    output.push_str(&format!("# HELP {name} {help}\n# TYPE {name} counter\n"));
    let counters = counters
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .clone();
    if counters.is_empty() {
        output.push_str(&format!("{name}{{{label}=\"none\"}} 0\n"));
    }
    for (key, value) in counters {
        output.push_str(&format!("{name}{{{label}=\"{key}\"}} {value}\n"));
    }
}

/// Renders a one-label counter family with the known label values present at
/// zero, so a rate query answers before the first event.
fn push_counter_family_with_zeroes(
    output: &mut String,
    name: &str,
    help: &str,
    counters: &Mutex<BTreeMap<String, u64>>,
    label: &str,
    defaults: &[&str],
) {
    output.push_str(&format!("# HELP {name} {help}\n# TYPE {name} counter\n"));
    let counters = counters
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .clone();
    for value in defaults {
        let count = counters.get(*value).copied().unwrap_or(0);
        output.push_str(&format!("{name}{{{label}=\"{value}\"}} {count}\n"));
    }
    for (key, count) in counters {
        if defaults.contains(&key.as_str()) {
            continue;
        }
        output.push_str(&format!("{name}{{{label}=\"{key}\"}} {count}\n"));
    }
}

/// Renders a two-label counter family with the known label pairs present at
/// zero.
fn push_counter_family_seeded(
    output: &mut String,
    name: &str,
    help: &str,
    counters: &Mutex<BTreeMap<(String, String), u64>>,
    labels: (&str, &str),
    defaults: &[(&str, &str)],
) {
    output.push_str(&format!("# HELP {name} {help}\n# TYPE {name} counter\n"));
    let counters = counters
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .clone();
    let (first, second) = labels;
    for (left, right) in defaults {
        let count = counters
            .get(&((*left).to_string(), (*right).to_string()))
            .copied()
            .unwrap_or(0);
        output.push_str(&format!(
            "{name}{{{first}=\"{left}\",{second}=\"{right}\"}} {count}\n"
        ));
    }
    for ((left, right), count) in counters {
        if defaults
            .iter()
            .any(|(known_left, known_right)| *known_left == left && *known_right == right)
        {
            continue;
        }
        output.push_str(&format!(
            "{name}{{{first}=\"{left}\",{second}=\"{right}\"}} {count}\n"
        ));
    }
}

impl ObservationSink for Metrics {
    fn record_decision(&self, class: OperationClass, reason: DenialReason, elapsed: Duration) {
        let mut decisions = self
            .authorisation_decisions
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        *decisions
            .entry((class.as_str().to_string(), reason.as_str().to_string()))
            .or_default() += 1;
        let mut durations = self
            .authorisation_durations
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        durations
            .entry("decision".to_string())
            .or_default()
            .observe(elapsed);
    }

    fn record_status_lookup_failure(&self, cause: StatusFailure) {
        self.bump(&self.authorisation_status_failures, cause.as_str());
    }

    fn record_audit_write_failure(&self, context: AuditWriteContext) {
        self.bump(&self.authorisation_audit_failures, context.as_str());
    }

    fn record_cache(&self, age_seconds: i64, entries: usize) {
        self.authorisation_cache_age
            .store(age_seconds, Ordering::Relaxed);
        self.authorisation_cache_entries
            .store(entries as i64, Ordering::Relaxed);
    }

    fn record_role_change(&self, success: bool) {
        let outcome = if success { "success" } else { "failed" };
        self.bump(&self.authorisation_role_changes, outcome);
    }
}

impl Metrics {
    fn bump(&self, counters: &Mutex<BTreeMap<String, u64>>, key: &str) {
        let mut counters = counters
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        *counters.entry(key.to_string()).or_default() += 1;
    }

    /// U3: one support-grant write, by operation and outcome (FR16.3–16.6).
    pub fn record_support_grant(&self, operation: &str, outcome: &str) {
        let mut grants = self
            .support_grants
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        *grants
            .entry((operation.to_string(), outcome.to_string()))
            .or_default() += 1;
    }

    /// U3: one support-grant write, its outcome and the time it took
    /// (FR16.3–FR16.6, NFR17.1). The histogram is separate from the read one
    /// because the write SLO and the read SLO are different budgets.
    pub fn record_support_write(&self, operation: &str, outcome: &str, elapsed: Duration) {
        self.record_support_grant(operation, outcome);
        self.support_write_durations
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .observe(elapsed);
    }

    /// U3: one metadata read, its outcome, and the composed latency.
    pub fn record_support_read(&self, outcome: &str, elapsed: Duration) {
        self.bump(&self.support_reads, outcome);
        self.support_read_durations
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .observe(elapsed);
    }

    /// U3: one owner source failure, naming the owner that could not answer.
    pub fn record_support_metadata_source_error(&self, source: &str, cause: &str) {
        let mut errors = self
            .support_metadata_source_errors
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        *errors
            .entry((source.to_string(), cause.to_string()))
            .or_default() += 1;
    }

    /// U3: one fail-closed support audit append.
    pub fn record_support_audit_write_failure(&self, cause: &str) {
        self.bump(&self.support_audit_write_failures, cause);
    }

    /// U3: one failing grant lookup inside the support read.
    pub fn record_support_grant_decision_error(&self, cause: &str) {
        self.bump(&self.support_grant_decision_errors, cause);
    }

    /// U3: the drainer finished a pass; the gauges are read at scrape time.
    pub fn record_support_outbox_pass(&self, pending: u64, oldest_age_seconds: i64) {
        self.support_outbox_pending.store(
            i64::try_from(pending).unwrap_or(i64::MAX),
            Ordering::Relaxed,
        );
        self.support_outbox_oldest_age
            .store(oldest_age_seconds, Ordering::Relaxed);
        self.support_outbox_last_pass
            .store(chrono::Utc::now().timestamp(), Ordering::Relaxed);
    }

    /// U3: one support audit-outbox delivery outcome.
    pub fn record_support_outbox_delivery(&self, outcome: &str) {
        self.bump(&self.support_outbox_deliveries, outcome);
    }
}

impl ConsentObservationSink for Metrics {
    fn record_state_query(&self, outcome: StateQueryOutcome, has_missing: bool, elapsed: Duration) {
        let outcome = match outcome {
            StateQueryOutcome::Answered => "answered",
            StateQueryOutcome::Error => "error",
        };
        self.bump(&self.consent_state_queries, outcome);
        self.bump(
            &self.consent_state_with_missing,
            if has_missing { "true" } else { "false" },
        );
        self.consent_state_durations
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .observe(elapsed);
    }

    fn record_consent_event(&self, kind: ConsentKind, action: ConsentAction) {
        let mut events = self
            .consent_events
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        *events
            .entry((kind.as_str().to_string(), action.as_str().to_string()))
            .or_default() += 1;
    }

    fn record_preference_write(&self, outcome: PreferenceWriteOutcome) {
        let outcome = match outcome {
            PreferenceWriteOutcome::Applied => "applied",
            PreferenceWriteOutcome::Refused => "refused",
        };
        self.bump(&self.preference_writes, outcome);
    }

    fn record_withdrawal_rollback(&self, cause: WithdrawalRollbackCause) {
        let cause = match cause {
            WithdrawalRollbackCause::OutboxFailed => "outbox_failed",
            WithdrawalRollbackCause::InsertFailed => "insert_failed",
        };
        self.bump(&self.consent_withdrawal_rollbacks, cause);
    }
}

impl DrainerObservation for Metrics {
    fn record_pass(&self, pending: u64, oldest_age_seconds: i64) {
        self.consent_outbox_pending.store(
            i64::try_from(pending).unwrap_or(i64::MAX),
            Ordering::Relaxed,
        );
        self.consent_outbox_oldest_age
            .store(oldest_age_seconds, Ordering::Relaxed);
        self.consent_outbox_last_pass
            .store(chrono::Utc::now().timestamp(), Ordering::Relaxed);
    }

    fn record_delivery(&self, outcome: DeliveryOutcome) {
        let outcome = match outcome {
            DeliveryOutcome::Delivered => "delivered",
            DeliveryOutcome::Retryable => "retryable",
            DeliveryOutcome::Dead => "dead",
        };
        self.bump(&self.consent_outbox_deliveries, outcome);
    }

    fn record_status_pass(&self, pending: u64, oldest_age_seconds: i64) {
        self.status_outbox_pending.store(
            i64::try_from(pending).unwrap_or(i64::MAX),
            Ordering::Relaxed,
        );
        self.status_outbox_oldest_age
            .store(oldest_age_seconds, Ordering::Relaxed);
    }

    fn record_status_delivery(&self, outcome: DeliveryOutcome) {
        let outcome = match outcome {
            DeliveryOutcome::Delivered => "delivered",
            DeliveryOutcome::Retryable => "retryable",
            DeliveryOutcome::Dead => "dead",
        };
        self.bump(&self.status_outbox_deliveries, outcome);
    }
}

impl AccountObservation for Metrics {
    fn record_transition(&self, from: UserStatus, to: UserStatus, actor_kind: &str) {
        let mut transitions = self
            .status_transitions
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        *transitions
            .entry((
                from.as_str().to_string(),
                to.as_str().to_string(),
                actor_kind.to_string(),
            ))
            .or_default() += 1;
    }

    fn record_refusal(&self, reason: &str) {
        self.bump(&self.status_refusals, reason);
    }

    fn record_deletion_event(&self, event: &str) {
        self.bump(&self.deletion_events, event);
    }

    fn record_erasure_failure(&self, step: &str) {
        self.bump(&self.erasure_failures, step);
    }
}

impl SupportSourceFailureSink for Metrics {
    fn source_failure(&self, source: &str, cause: &str) {
        self.record_support_metadata_source_error(source, cause);
    }
}

impl SupportDrainObservation for Metrics {
    fn record_delivery(&self, outcome: SupportDrainDelivery) {
        let outcome = match outcome {
            SupportDrainDelivery::Delivered => "delivered",
            SupportDrainDelivery::Retryable => "retryable",
            SupportDrainDelivery::Dead => "dead",
        };
        self.record_support_outbox_delivery(outcome);
    }

    fn record_pass(&self, pending: u64, oldest_age_seconds: i64) {
        self.record_support_outbox_pass(pending, oldest_age_seconds);
    }
}
