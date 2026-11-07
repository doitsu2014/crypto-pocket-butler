//! Prometheus text metrics for the Analytics API and worker.
//!
//! Labels are low-cardinality: the event name is from the closed catalogue, and
//! no user reference, subject, or property value ever appears.

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;

const BUCKETS: [f64; 15] = [
    0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0, 30.0, 60.0, 120.0, 180.0,
];

#[derive(Debug, Default)]
struct Histogram {
    buckets: [u64; BUCKETS.len() + 1],
    sum: f64,
    count: u64,
}

/// Request, ingest, aggregation, and erasure metrics.
#[derive(Debug, Default)]
pub struct Metrics {
    total_2xx: AtomicU64,
    total_3xx: AtomicU64,
    total_4xx: AtomicU64,
    total_5xx: AtomicU64,
    errors_total: AtomicU64,
    suppressed_cohorts: AtomicU64,
    erase_errors: AtomicU64,
    histogram: Mutex<Histogram>,
    aggregate_histogram: Mutex<Histogram>,
    events: Mutex<BTreeMap<(String, String, String), u64>>,
    aggregate_runs: Mutex<BTreeMap<String, u64>>,
    erase_outcomes: Mutex<BTreeMap<String, u64>>,
}

impl Metrics {
    /// Records one response status.
    pub fn record(&self, status: u16) {
        match status {
            200..=299 => self.total_2xx.fetch_add(1, Ordering::Relaxed),
            300..=399 => self.total_3xx.fetch_add(1, Ordering::Relaxed),
            400..=499 => self.total_4xx.fetch_add(1, Ordering::Relaxed),
            _ => {
                self.total_5xx.fetch_add(1, Ordering::Relaxed);
                self.errors_total.fetch_add(1, Ordering::Relaxed)
            }
        };
    }

    /// Records one request's duration in the histogram.
    pub fn record_duration(&self, seconds: f64) {
        record_in_histogram(&self.histogram, seconds);
    }

    /// Records one ingest outcome.
    pub fn record_event(&self, event_name: &str, consent_result: &str, outcome: &str) {
        let mut events = self
            .events
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        *events
            .entry((
                event_name.to_string(),
                consent_result.to_string(),
                outcome.to_string(),
            ))
            .or_insert(0) += 1;
    }

    /// Records one aggregation run's duration in the worker histogram.
    pub fn record_aggregate_duration(&self, seconds: f64) {
        record_in_histogram(&self.aggregate_histogram, seconds);
    }

    /// Records one aggregation run and its suppressed-cohort count.
    pub fn record_aggregate_run(&self, success: bool, suppressed: u64) {
        let key = if success { "success" } else { "failed" };
        let mut runs = self
            .aggregate_runs
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        *runs.entry(key.to_string()).or_insert(0) += 1;
        self.suppressed_cohorts.store(suppressed, Ordering::Relaxed);
    }

    /// Records one erasure outcome.
    pub fn record_erase(&self, success: bool) {
        if !success {
            self.erase_errors.fetch_add(1, Ordering::Relaxed);
        }
        let key = if success { "success" } else { "error" };
        let mut outcomes = self
            .erase_outcomes
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        *outcomes.entry(key.to_string()).or_insert(0) += 1;
    }

    /// Total 5xx responses observed.
    #[must_use]
    pub fn errors_total(&self) -> u64 {
        self.errors_total.load(Ordering::Relaxed)
    }

    /// Renders the Prometheus text exposition.
    #[must_use]
    pub fn render(&self, service: &str) -> String {
        let mut output = String::new();
        output.push_str(
            "# HELP analytics_requests_total Total requests handled by the service.\n\
             # TYPE analytics_requests_total counter\n",
        );
        for (class, value) in [
            ("2xx", self.total_2xx.load(Ordering::Relaxed)),
            ("3xx", self.total_3xx.load(Ordering::Relaxed)),
            ("4xx", self.total_4xx.load(Ordering::Relaxed)),
            ("5xx", self.total_5xx.load(Ordering::Relaxed)),
        ] {
            output.push_str(&format!(
                "analytics_requests_total{{service=\"{service}\",status_class=\"{class}\"}} {value}\n"
            ));
        }

        output.push_str(
            "# HELP analytics_request_errors_total Total 5xx responses.\n\
             # TYPE analytics_request_errors_total counter\n",
        );
        output.push_str(&format!(
            "analytics_request_errors_total{{service=\"{service}\"}} {}\n",
            self.errors_total.load(Ordering::Relaxed)
        ));

        {
            let histogram = self
                .histogram
                .lock()
                .unwrap_or_else(|error| error.into_inner());
            write_histogram(
                &mut output,
                "analytics_request_duration_seconds",
                "Request duration.",
                service,
                &histogram,
            );
        }
        {
            let histogram = self
                .aggregate_histogram
                .lock()
                .unwrap_or_else(|error| error.into_inner());
            write_histogram(
                &mut output,
                "analytics_aggregate_duration_seconds",
                "Nightly aggregation run duration.",
                service,
                &histogram,
            );
        }

        output.push_str(
            "# HELP analytics_events_total Ingest outcomes by event and consent result.\n\
             # TYPE analytics_events_total counter\n",
        );
        let events = self
            .events
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        for ((event_name, consent_result, outcome), value) in events.iter() {
            output.push_str(&format!(
                "analytics_events_total{{service=\"{service}\",event_name=\"{event_name}\",consent_result=\"{consent_result}\",outcome=\"{outcome}\"}} {value}\n"
            ));
        }

        output.push_str(
            "# HELP analytics_aggregate_runs_total Aggregation runs.\n\
             # TYPE analytics_aggregate_runs_total counter\n",
        );
        let runs = self
            .aggregate_runs
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        for (status, value) in runs.iter() {
            output.push_str(&format!(
                "analytics_aggregate_runs_total{{service=\"{service}\",status=\"{status}\"}} {value}\n"
            ));
        }

        output.push_str(
            "# HELP analytics_suppressed_cohorts Cohorts suppressed by the last run.\n\
             # TYPE analytics_suppressed_cohorts gauge\n",
        );
        output.push_str(&format!(
            "analytics_suppressed_cohorts{{service=\"{service}\"}} {}\n",
            self.suppressed_cohorts.load(Ordering::Relaxed)
        ));

        output.push_str(
            "# HELP analytics_erased_events_total Erasure outcomes.\n\
             # TYPE analytics_erased_events_total counter\n",
        );
        let outcomes = self
            .erase_outcomes
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        for (outcome, value) in outcomes.iter() {
            output.push_str(&format!(
                "analytics_erased_events_total{{service=\"{service}\",outcome=\"{outcome}\"}} {value}\n"
            ));
        }

        output.push_str(
            "# HELP analytics_health 1 while the process is running.\n\
             # TYPE analytics_health gauge\n",
        );
        output.push_str(&format!("analytics_health{{service=\"{service}\"}} 1\n"));
        output
    }
}

fn record_in_histogram(target: &Mutex<Histogram>, seconds: f64) {
    let mut histogram = target.lock().unwrap_or_else(|error| error.into_inner());
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

fn write_histogram(
    output: &mut String,
    name: &str,
    help: &str,
    service: &str,
    histogram: &Histogram,
) {
    output.push_str(&format!("# HELP {name} {help}\n# TYPE {name} histogram\n"));
    let mut cumulative = 0u64;
    for (index, bound) in BUCKETS.iter().enumerate() {
        cumulative += histogram.buckets[index];
        output.push_str(&format!(
            "{name}_bucket{{service=\"{service}\",le=\"{bound}\"}} {cumulative}\n"
        ));
    }
    cumulative += histogram.buckets[BUCKETS.len()];
    output.push_str(&format!(
        "{name}_bucket{{service=\"{service}\",le=\"+Inf\"}} {cumulative}\n"
    ));
    output.push_str(&format!(
        "{name}_sum{{service=\"{service}\"}} {}\n",
        histogram.sum
    ));
    output.push_str(&format!(
        "{name}_count{{service=\"{service}\"}} {}\n",
        histogram.count
    ));
}
