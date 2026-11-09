//! Prometheus text metrics for the Ledger API and worker.
//!
//! Labels are low-cardinality: the route template and the status class. No
//! amount, asset quantity, user reference, or credential ever appears.

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;

const DURATION_BUCKETS: [f64; 15] = [
    0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0, 30.0, 60.0, 120.0, 180.0,
];
const COVERAGE_BUCKETS: [f64; 6] = [0.0, 0.25, 0.5, 0.75, 0.99, 1.0];

#[derive(Debug)]
struct Histogram {
    buckets: Vec<u64>,
    bounds: &'static [f64],
    sum: f64,
    count: u64,
}

impl Histogram {
    fn new(bounds: &'static [f64]) -> Self {
        Self {
            buckets: vec![0; bounds.len() + 1],
            bounds,
            sum: 0.0,
            count: 0,
        }
    }

    fn record(&mut self, value: f64) {
        self.sum += value;
        self.count += 1;
        for (index, bound) in self.bounds.iter().enumerate() {
            if value <= *bound {
                self.buckets[index] += 1;
                return;
            }
        }
        self.buckets[self.bounds.len()] += 1;
    }
}

fn status_class(status: u16) -> &'static str {
    match status {
        200..=299 => "2xx",
        300..=399 => "3xx",
        400..=499 => "4xx",
        _ => "5xx",
    }
}

/// Request, ingest, correction, reconcile, coverage, and audit metrics.
#[derive(Debug)]
pub struct Metrics {
    requests: Mutex<BTreeMap<(String, String), u64>>,
    duration: Mutex<BTreeMap<(String, String), Histogram>>,
    transactions: Mutex<BTreeMap<(String, String), u64>>,
    duplicates: Mutex<BTreeMap<String, u64>>,
    corrections: Mutex<BTreeMap<String, u64>>,
    reconcile_runs: Mutex<BTreeMap<String, u64>>,
    reconcile_duration: Mutex<Histogram>,
    coverage: Mutex<Histogram>,
    audit_failures: AtomicU64,
}

impl Default for Metrics {
    fn default() -> Self {
        Self {
            requests: Mutex::new(BTreeMap::new()),
            duration: Mutex::new(BTreeMap::new()),
            transactions: Mutex::new(BTreeMap::new()),
            duplicates: Mutex::new(BTreeMap::new()),
            corrections: Mutex::new(BTreeMap::new()),
            reconcile_runs: Mutex::new(BTreeMap::new()),
            reconcile_duration: Mutex::new(Histogram::new(&DURATION_BUCKETS)),
            coverage: Mutex::new(Histogram::new(&COVERAGE_BUCKETS)),
            audit_failures: AtomicU64::new(0),
        }
    }
}

impl Metrics {
    /// Records one response.
    pub fn record_request(&self, route: &str, status: u16, seconds: f64) {
        let class = status_class(status).to_string();
        let key = (route.to_string(), class);
        *self
            .requests
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .entry(key.clone())
            .or_insert(0) += 1;
        self.duration
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .entry(key)
            .or_insert_with(|| Histogram::new(&DURATION_BUCKETS))
            .record(seconds);
    }

    /// Records ingested transactions by kind and source.
    pub fn record_transactions(&self, kind: &str, source: &str, count: u64) {
        *self
            .transactions
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .entry((kind.to_string(), source.to_string()))
            .or_insert(0) += count;
    }

    /// Records skipped duplicates by source.
    pub fn record_duplicates(&self, source: &str, count: u64) {
        *self
            .duplicates
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .entry(source.to_string())
            .or_insert(0) += count;
    }

    /// Records one correction by field.
    pub fn record_correction(&self, field: &str) {
        *self
            .corrections
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .entry(field.to_string())
            .or_insert(0) += 1;
    }

    /// Records one reconcile run.
    pub fn record_reconcile(&self, status: &str, seconds: f64) {
        *self
            .reconcile_runs
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .entry(status.to_string())
            .or_insert(0) += 1;
        self.reconcile_duration
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .record(seconds);
    }

    /// Records one P&L coverage value.
    pub fn record_coverage(&self, coverage: f64) {
        self.coverage
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .record(coverage);
    }

    /// Records one exhausted audit retry.
    pub fn record_audit_failure(&self) {
        self.audit_failures.fetch_add(1, Ordering::Relaxed);
    }

    /// Total 5xx responses observed.
    #[must_use]
    pub fn errors_total(&self) -> u64 {
        self.requests
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .iter()
            .filter(|((_, class), _)| class == "5xx")
            .map(|(_, value)| *value)
            .sum()
    }

    /// Renders the Prometheus text exposition.
    #[must_use]
    pub fn render(&self, service: &str) -> String {
        let mut output = String::new();
        output.push_str(
            "# HELP ledger_requests_total Total requests handled.\n\
             # TYPE ledger_requests_total counter\n",
        );
        for ((route, class), value) in self
            .requests
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .iter()
        {
            output.push_str(&format!(
                "ledger_requests_total{{service=\"{service}\",route=\"{route}\",status=\"{class}\"}} {value}\n"
            ));
        }

        output.push_str(
            "# HELP ledger_request_errors_total Total 5xx responses.\n\
             # TYPE ledger_request_errors_total counter\n",
        );
        for ((route, class), value) in self
            .requests
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .iter()
        {
            if class == "5xx" {
                output.push_str(&format!(
                    "ledger_request_errors_total{{service=\"{service}\",route=\"{route}\",status=\"{class}\"}} {value}\n"
                ));
            }
        }

        output.push_str(
            "# HELP ledger_request_duration_seconds Request duration.\n\
             # TYPE ledger_request_duration_seconds histogram\n",
        );
        for ((route, class), histogram) in self
            .duration
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .iter()
        {
            write_histogram(
                &mut output,
                "ledger_request_duration_seconds",
                service,
                &[("route", route), ("status", class)],
                histogram,
            );
        }

        output.push_str(
            "# HELP ledger_transactions_total Ingested transactions by kind and source.\n\
             # TYPE ledger_transactions_total counter\n",
        );
        for ((kind, source), value) in self
            .transactions
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .iter()
        {
            output.push_str(&format!(
                "ledger_transactions_total{{service=\"{service}\",kind=\"{kind}\",source=\"{source}\"}} {value}\n"
            ));
        }

        output.push_str(
            "# HELP ledger_duplicates_total Rows skipped as duplicates.\n\
             # TYPE ledger_duplicates_total counter\n",
        );
        for (source, value) in self
            .duplicates
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .iter()
        {
            output.push_str(&format!(
                "ledger_duplicates_total{{service=\"{service}\",source=\"{source}\"}} {value}\n"
            ));
        }

        output.push_str(
            "# HELP ledger_corrections_total Corrections by field.\n\
             # TYPE ledger_corrections_total counter\n",
        );
        for (field, value) in self
            .corrections
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .iter()
        {
            output.push_str(&format!(
                "ledger_corrections_total{{service=\"{service}\",field=\"{field}\"}} {value}\n"
            ));
        }

        output.push_str(
            "# HELP ledger_reconcile_runs_total Reconcile runs.\n\
             # TYPE ledger_reconcile_runs_total counter\n",
        );
        for (status, value) in self
            .reconcile_runs
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .iter()
        {
            output.push_str(&format!(
                "ledger_reconcile_runs_total{{service=\"{service}\",status=\"{status}\"}} {value}\n"
            ));
        }

        {
            let histogram = self
                .reconcile_duration
                .lock()
                .unwrap_or_else(|error| error.into_inner());
            write_histogram(
                &mut output,
                "ledger_reconcile_duration_seconds",
                service,
                &[],
                &histogram,
            );
        }
        {
            let histogram = self
                .coverage
                .lock()
                .unwrap_or_else(|error| error.into_inner());
            write_histogram(&mut output, "ledger_pnl_coverage", service, &[], &histogram);
        }

        output.push_str(
            "# HELP ledger_audit_failures_total C10 audit writes whose retries were exhausted.\n\
             # TYPE ledger_audit_failures_total counter\n",
        );
        output.push_str(&format!(
            "ledger_audit_failures_total{{service=\"{service}\"}} {}\n",
            self.audit_failures.load(Ordering::Relaxed)
        ));

        output.push_str(
            "# HELP ledger_health 1 while the process is running.\n\
             # TYPE ledger_health gauge\n",
        );
        output.push_str(&format!("ledger_health{{service=\"{service}\"}} 1\n"));
        output
    }
}

fn write_histogram(
    output: &mut String,
    name: &str,
    service: &str,
    extra_labels: &[(&str, &str)],
    histogram: &Histogram,
) {
    output.push_str(&format!(
        "# HELP {name} {name} observations.\n# TYPE {name} histogram\n"
    ));
    let mut cumulative = 0u64;
    for (index, bound) in histogram.bounds.iter().enumerate() {
        cumulative += histogram.buckets[index];
        output.push_str(&format!(
            "{name}_bucket{} {cumulative}\n",
            labels(service, extra_labels, &format!("le=\"{bound}\""))
        ));
    }
    cumulative += histogram.buckets[histogram.bounds.len()];
    output.push_str(&format!(
        "{name}_bucket{} {cumulative}\n",
        labels(service, extra_labels, "le=\"+Inf\"")
    ));
    output.push_str(&format!(
        "{name}_sum{} {}\n",
        labels(service, extra_labels, ""),
        histogram.sum
    ));
    output.push_str(&format!(
        "{name}_count{} {}\n",
        labels(service, extra_labels, ""),
        histogram.count
    ));
}

fn labels(service: &str, extra: &[(&str, &str)], last: &str) -> String {
    let mut rendered = format!("{{service=\"{service}\"");
    for (key, value) in extra {
        rendered.push_str(&format!(",{key}=\"{value}\""));
    }
    if !last.is_empty() {
        rendered.push(',');
        rendered.push_str(last);
    }
    rendered.push('}');
    rendered
}
