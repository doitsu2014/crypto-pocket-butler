//! Minimal Prometheus text metrics: request counts, error counts, append and
//! check counts, and liveness. Labels are low-cardinality only; no user or
//! service identifier ever appears here.

use std::sync::atomic::{AtomicU64, Ordering};

/// Request, error, and audit-path counters exposed at `/metrics`.
#[derive(Debug, Default)]
pub struct Metrics {
    total_2xx: AtomicU64,
    total_3xx: AtomicU64,
    total_4xx: AtomicU64,
    total_5xx: AtomicU64,
    errors_total: AtomicU64,
    appends_total: AtomicU64,
    append_errors_total: AtomicU64,
    queries_total: AtomicU64,
    exports_total: AtomicU64,
    support_checks_total: AtomicU64,
    retention_runs_total: AtomicU64,
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

    /// Records one append attempt and whether it failed at storage.
    pub fn record_append(&self, failed: bool) {
        self.appends_total.fetch_add(1, Ordering::Relaxed);
        if failed {
            self.append_errors_total.fetch_add(1, Ordering::Relaxed);
        }
    }

    /// Records one admin query.
    pub fn record_query(&self) {
        self.queries_total.fetch_add(1, Ordering::Relaxed);
    }

    /// Records one export request.
    pub fn record_export(&self) {
        self.exports_total.fetch_add(1, Ordering::Relaxed);
    }

    /// Records one support-session check.
    pub fn record_support_check(&self) {
        self.support_checks_total.fetch_add(1, Ordering::Relaxed);
    }

    /// Records one retention run recorded or executed.
    pub fn record_retention_run(&self) {
        self.retention_runs_total.fetch_add(1, Ordering::Relaxed);
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
        for (class, value) in classes {
            output.push_str(&format!(
                "http_requests_total{{service=\"{service}\",status_class=\"{class}\"}} {value}\n"
            ));
        }
        output.push_str(
            "# HELP http_errors_total Total 5xx responses.\n\
             # TYPE http_errors_total counter\n",
        );
        output.push_str(&format!(
            "http_errors_total{{service=\"{service}\"}} {}\n",
            self.errors_total.load(Ordering::Relaxed)
        ));
        let counters = [
            (
                "audit_appends_total",
                "Audit appends attempted.",
                self.appends_total.load(Ordering::Relaxed),
            ),
            (
                "audit_append_errors_total",
                "Audit appends that could not be stored.",
                self.append_errors_total.load(Ordering::Relaxed),
            ),
            (
                "audit_queries_total",
                "Admin trail queries served.",
                self.queries_total.load(Ordering::Relaxed),
            ),
            (
                "audit_exports_total",
                "Audit export requests recorded.",
                self.exports_total.load(Ordering::Relaxed),
            ),
            (
                "audit_support_checks_total",
                "Support-session checks answered.",
                self.support_checks_total.load(Ordering::Relaxed),
            ),
            (
                "audit_retention_runs_total",
                "Retention runs recorded.",
                self.retention_runs_total.load(Ordering::Relaxed),
            ),
        ];
        for (name, help, value) in counters {
            output.push_str(&format!("# HELP {name} {help}\n# TYPE {name} counter\n"));
            output.push_str(&format!("{name}{{service=\"{service}\"}} {value}\n"));
        }
        output.push_str(
            "# HELP service_up 1 while the process is running.\n\
             # TYPE service_up gauge\n",
        );
        output.push_str(&format!("service_up{{service=\"{service}\"}} 1\n"));
        output
    }
}
