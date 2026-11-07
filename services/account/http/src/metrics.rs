//! Minimal Prometheus text metrics: request classes, the error rate, sync runs,
//! import rows, lease sweeps, and a health gauge (observability design).

use std::sync::atomic::{AtomicU64, Ordering};

/// Request and workflow counters exposed at `/metrics`.
#[derive(Debug, Default)]
pub struct Metrics {
    total_2xx: AtomicU64,
    total_3xx: AtomicU64,
    total_4xx: AtomicU64,
    total_5xx: AtomicU64,
    errors_total: AtomicU64,
    sync_succeeded: AtomicU64,
    sync_failed: AtomicU64,
    import_imported: AtomicU64,
    import_skipped: AtomicU64,
    import_failed: AtomicU64,
    lease_expired: AtomicU64,
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

    /// Records one finished sync run.
    pub fn record_sync(&self, succeeded: bool) {
        if succeeded {
            self.sync_succeeded.fetch_add(1, Ordering::Relaxed);
        } else {
            self.sync_failed.fetch_add(1, Ordering::Relaxed);
        }
    }

    /// Records the reconciling counts of one import.
    pub fn record_import(&self, imported: u32, skipped: u32, failed: u32) {
        self.import_imported
            .fetch_add(u64::from(imported), Ordering::Relaxed);
        self.import_skipped
            .fetch_add(u64::from(skipped), Ordering::Relaxed);
        self.import_failed
            .fetch_add(u64::from(failed), Ordering::Relaxed);
    }

    /// Records one lease-expiry sweep.
    pub fn record_lease_expiry(&self, count: usize) {
        self.lease_expired
            .fetch_add(count as u64, Ordering::Relaxed);
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
            "# HELP account_request_errors_total Total 5xx responses.\n\
             # TYPE account_request_errors_total counter\n",
        );
        output.push_str(&format!(
            "account_request_errors_total{{service=\"{service}\"}} {}\n",
            self.errors_total.load(Ordering::Relaxed)
        ));
        output.push_str(
            "# HELP account_sync_runs_total Sync runs observed, by status.\n\
             # TYPE account_sync_runs_total counter\n",
        );
        output.push_str(&format!(
            "account_sync_runs_total{{status=\"succeeded\"}} {}\n",
            self.sync_succeeded.load(Ordering::Relaxed)
        ));
        output.push_str(&format!(
            "account_sync_runs_total{{status=\"failed\"}} {}\n",
            self.sync_failed.load(Ordering::Relaxed)
        ));
        output.push_str(
            "# HELP account_import_rows_total Import rows observed, by outcome.\n\
             # TYPE account_import_rows_total counter\n",
        );
        for (outcome, value) in [
            ("imported", self.import_imported.load(Ordering::Relaxed)),
            ("skipped", self.import_skipped.load(Ordering::Relaxed)),
            ("failed", self.import_failed.load(Ordering::Relaxed)),
        ] {
            output.push_str(&format!(
                "account_import_rows_total{{outcome=\"{outcome}\"}} {value}\n"
            ));
        }
        output.push_str(
            "# HELP account_lease_expired_total Sync runs failed by the lease sweep.\n\
             # TYPE account_lease_expired_total counter\n",
        );
        output.push_str(&format!(
            "account_lease_expired_total{{kind=\"sync\"}} {}\n",
            self.lease_expired.load(Ordering::Relaxed)
        ));
        output.push_str(
            "# HELP account_health 1 while the process is running.\n\
             # TYPE account_health gauge\n",
        );
        output.push_str(&format!("account_health{{service=\"{service}\"}} 1\n"));
        output
    }
}
