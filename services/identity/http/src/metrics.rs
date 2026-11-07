//! Minimal Prometheus text metrics: request classes, error rate, and the
//! export/deletion status counts the runbook alerts read.

use std::sync::atomic::{AtomicU64, Ordering};

/// Request and workflow counters exposed at `/metrics`.
#[derive(Debug, Default)]
pub struct Metrics {
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
            "# HELP service_up 1 while the process is running.\n\
             # TYPE service_up gauge\n",
        );
        output.push_str(&format!("service_up{{service=\"{service}\"}} 1\n"));
        output
    }
}
