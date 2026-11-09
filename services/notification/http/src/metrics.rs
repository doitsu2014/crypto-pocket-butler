//! Prometheus text metrics for the Notification API and worker.
//!
//! Labels are bounded: the route is the matched Axum pattern (never a raw path,
//! so no user id leaks), the type comes from the closed set, and the channel from
//! the two-value set. No address, user id, or payload value ever appears.

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;

use notification_application::DeliveryReport;
use notification_domain::NotificationType;

const BUCKETS: [f64; 15] = [
    0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0, 30.0, 60.0, 120.0, 180.0,
];

#[derive(Debug, Default)]
struct Histogram {
    buckets: [u64; BUCKETS.len() + 1],
    sum: f64,
    count: u64,
}

/// Request, intake, delivery, and audit metrics.
#[derive(Debug, Default)]
pub struct Metrics {
    audit_failures: AtomicU64,
    /// `(route, status) -> count`.
    requests: Mutex<BTreeMap<(String, u16), u64>>,
    /// `(route, status) -> count`, 5xx only.
    errors: Mutex<BTreeMap<(String, u16), u64>>,
    /// `(route, status) -> histogram`.
    durations: Mutex<BTreeMap<(String, u16), Histogram>>,
    alerts: Mutex<BTreeMap<(String, String), u64>>,
    attempts: Mutex<BTreeMap<(String, String), u64>>,
    duplicates: Mutex<BTreeMap<String, u64>>,
    mandatory: Mutex<BTreeMap<(String, String), u64>>,
}

impl Metrics {
    /// Records one response against the route that matched and its status.
    pub fn record_route(&self, route: &str, status: u16, seconds: f64) {
        increment(&self.requests, (route.to_string(), status));
        if status >= 500 {
            increment(&self.errors, (route.to_string(), status));
        }
        let mut durations = self
            .durations
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        record_in_histogram(
            durations.entry((route.to_string(), status)).or_default(),
            seconds,
        );
    }

    /// Records one alert outcome.
    pub fn record_alert(&self, kind: &str, status: &str) {
        increment(&self.alerts, (kind.to_string(), status.to_string()));
    }

    /// Records one delivery attempt.
    pub fn record_attempt(&self, channel: &str, status: &str) {
        increment(&self.attempts, (channel.to_string(), status.to_string()));
    }

    /// Records one deduplicated submit (BR1.1).
    pub fn record_duplicate(&self, kind: &str) {
        increment(&self.duplicates, kind.to_string());
    }

    /// Records one mandatory email outcome.
    pub fn record_mandatory(&self, kind: &str, status: &str) {
        increment(&self.mandatory, (kind.to_string(), status.to_string()));
    }

    /// Adds to the abandoned-audit-write counter.
    pub fn record_audit_failures(&self, count: u64) {
        if count > 0 {
            self.audit_failures.fetch_add(count, Ordering::Relaxed);
        }
    }

    /// Renders the Prometheus text exposition. `trail_failures` is the trail's
    /// own count, so a refusal path that never reaches a handler still shows.
    #[must_use]
    pub fn render(&self, service: &str, trail_failures: u64) -> String {
        let mut output = String::new();
        {
            let durations = self
                .durations
                .lock()
                .unwrap_or_else(|error| error.into_inner());
            output.push_str(
                "# HELP notification_request_duration_seconds Request duration by route and status.\n\
                 # TYPE notification_request_duration_seconds histogram\n",
            );
            for ((route, status), histogram) in durations.iter() {
                let mut cumulative = 0u64;
                for (index, bound) in BUCKETS.iter().enumerate() {
                    cumulative += histogram.buckets[index];
                    output.push_str(&format!(
                        "notification_request_duration_seconds_bucket{{service=\"{service}\",route=\"{route}\",status=\"{status}\",le=\"{bound}\"}} {cumulative}\n"
                    ));
                }
                cumulative += histogram.buckets[BUCKETS.len()];
                output.push_str(&format!(
                    "notification_request_duration_seconds_bucket{{service=\"{service}\",route=\"{route}\",status=\"{status}\",le=\"+Inf\"}} {cumulative}\n"
                ));
                output.push_str(&format!(
                    "notification_request_duration_seconds_sum{{service=\"{service}\",route=\"{route}\",status=\"{status}\"}} {}\n",
                    histogram.sum
                ));
                output.push_str(&format!(
                    "notification_request_duration_seconds_count{{service=\"{service}\",route=\"{route}\",status=\"{status}\"}} {}\n",
                    histogram.count
                ));
            }
        }

        output.push_str(
            "# HELP notification_request_errors_total 5xx responses by route and status.\n\
             # TYPE notification_request_errors_total counter\n",
        );
        let errors = self
            .errors
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        for ((route, status), value) in errors.iter() {
            output.push_str(&format!(
                "notification_request_errors_total{{service=\"{service}\",route=\"{route}\",status=\"{status}\"}} {value}\n"
            ));
        }

        output.push_str(
            "# HELP notification_requests_total Requests by route and status.\n\
             # TYPE notification_requests_total counter\n",
        );
        let requests = self
            .requests
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        for ((route, status), value) in requests.iter() {
            output.push_str(&format!(
                "notification_requests_total{{service=\"{service}\",route=\"{route}\",status=\"{status}\"}} {value}\n"
            ));
        }

        write_pairs(
            &mut output,
            "notification_alerts_total",
            "Alert outcomes by type and status.",
            service,
            "type",
            "status",
            &self.alerts,
        );
        write_pairs(
            &mut output,
            "notification_attempts_total",
            "Delivery attempts by channel and status.",
            service,
            "channel",
            "status",
            &self.attempts,
        );
        write_pairs(
            &mut output,
            "notification_mandatory_delivery_total",
            "Mandatory email outcomes by type and status.",
            service,
            "type",
            "status",
            &self.mandatory,
        );

        output.push_str(
            "# HELP notification_duplicates_total Deduplicated submits by type.\n\
             # TYPE notification_duplicates_total counter\n",
        );
        let duplicates = self
            .duplicates
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        for (kind, value) in duplicates.iter() {
            output.push_str(&format!(
                "notification_duplicates_total{{service=\"{service}\",type=\"{kind}\"}} {value}\n"
            ));
        }

        output.push_str(
            "# HELP notification_audit_failures_total Audit writes abandoned after every retry.\n\
             # TYPE notification_audit_failures_total counter\n",
        );
        let total = self.audit_failures.load(Ordering::Relaxed) + trail_failures;
        output.push_str(&format!(
            "notification_audit_failures_total{{service=\"{service}\"}} {total}\n"
        ));

        output.push_str(
            "# HELP notification_health 1 while the process is running.\n\
             # TYPE notification_health gauge\n",
        );
        output.push_str(&format!("notification_health{{service=\"{service}\"}} 1\n"));
        output
    }
}

/// Records one run's alerts, attempts, and audit failures. Called by both the C11
/// handler and the worker's five-second poll, so normal operation is metered too.
pub fn record_delivery_report(metrics: &Metrics, report: &DeliveryReport) {
    for outcome in &report.alerts {
        metrics.record_alert(&outcome.kind, &outcome.status);
        if NotificationType::parse(&outcome.kind).is_ok_and(NotificationType::is_mandatory_email) {
            metrics.record_mandatory(&outcome.kind, &outcome.status);
        }
    }
    for attempt in &report.attempts {
        metrics.record_attempt(&attempt.channel, &attempt.status);
    }
    metrics.record_audit_failures(report.audit_failures);
}

fn increment<K: Ord + Clone>(target: &Mutex<BTreeMap<K, u64>>, key: K) {
    let mut map = target.lock().unwrap_or_else(|error| error.into_inner());
    *map.entry(key).or_insert(0) += 1;
}

fn record_in_histogram(histogram: &mut Histogram, seconds: f64) {
    histogram.sum += seconds;
    histogram.count += 1;
    for (index, bound) in BUCKETS.iter().enumerate() {
        if seconds <= *bound {
            histogram.buckets[index] += 1;
            return;
        }
    }
    histogram.buckets[BUCKETS.len()] += 1;
}

fn write_pairs(
    output: &mut String,
    name: &str,
    help: &str,
    service: &str,
    first_label: &str,
    second_label: &str,
    map: &Mutex<BTreeMap<(String, String), u64>>,
) {
    output.push_str(&format!("# HELP {name} {help}\n# TYPE {name} counter\n"));
    let map = map.lock().unwrap_or_else(|error| error.into_inner());
    for ((first, second), value) in map.iter() {
        output.push_str(&format!(
            "{name}{{service=\"{service}\",{first_label}=\"{first}\",{second_label}=\"{second}\"}} {value}\n"
        ));
    }
}
