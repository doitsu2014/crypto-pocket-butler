//! Minimal Prometheus text metrics: request classes and the error rate, the
//! valuation status, unpriced assets, snapshots, digest mismatches, audit
//! failures, guardrail alert failures, run duration, and a health gauge
//! (observability design).

use std::sync::atomic::{AtomicU64, Ordering};

/// Request and workflow counters exposed at `/metrics`.
#[derive(Debug, Default)]
pub struct Metrics {
    total_2xx: AtomicU64,
    total_3xx: AtomicU64,
    total_4xx: AtomicU64,
    total_5xx: AtomicU64,
    errors_total: AtomicU64,
    valuations_complete: AtomicU64,
    valuations_partial: AtomicU64,
    valuations_unavailable: AtomicU64,
    unpriced_assets: AtomicU64,
    snapshots_total: AtomicU64,
    digest_mismatch: AtomicU64,
    audit_failures: AtomicU64,
    guardrail_alert_failures: AtomicU64,
    overlays_expired: AtomicU64,
    run_duration_seconds: AtomicU64,
    runs_succeeded: AtomicU64,
    runs_failed: AtomicU64,
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

    /// Records one valuation by status and its excluded asset count.
    pub fn record_valuation(&self, status: &str, unpriced_assets: u32) {
        match status {
            "complete" => self.valuations_complete.fetch_add(1, Ordering::Relaxed),
            "partial" => self.valuations_partial.fetch_add(1, Ordering::Relaxed),
            _ => self.valuations_unavailable.fetch_add(1, Ordering::Relaxed),
        };
        self.unpriced_assets
            .store(u64::from(unpriced_assets), Ordering::Relaxed);
    }

    /// Records one written snapshot.
    pub fn record_snapshot(&self) {
        self.snapshots_total.fetch_add(1, Ordering::Relaxed);
    }

    /// Records one snapshot digest mismatch.
    pub fn record_digest_mismatch(&self) {
        self.digest_mismatch.fetch_add(1, Ordering::Relaxed);
    }

    /// Records one abandoned audit write.
    pub fn record_audit_failure(&self) {
        self.audit_failures.fetch_add(1, Ordering::Relaxed);
    }

    /// Records one failed guardrail alert.
    pub fn record_guardrail_alert_failure(&self, count: u32) {
        self.guardrail_alert_failures
            .fetch_add(u64::from(count), Ordering::Relaxed);
    }

    /// Records overlays expired because no venue sync reported them.
    pub fn record_overlays_expired(&self, count: u64) {
        self.overlays_expired.fetch_add(count, Ordering::Relaxed);
    }

    /// Records one finished run.
    pub fn record_run(&self, failed: bool, duration_seconds: u64) {
        if failed {
            self.runs_failed.fetch_add(1, Ordering::Relaxed);
        } else {
            self.runs_succeeded.fetch_add(1, Ordering::Relaxed);
        }
        self.run_duration_seconds
            .store(duration_seconds, Ordering::Relaxed);
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
            "# HELP portfolio_request_errors_total Total 5xx responses.\n\
             # TYPE portfolio_request_errors_total counter\n",
        );
        output.push_str(&format!(
            "portfolio_request_errors_total{{service=\"{service}\"}} {}\n",
            self.errors_total.load(Ordering::Relaxed)
        ));
        output.push_str(
            "# HELP portfolio_valuations_total Valuations observed, by status.\n\
             # TYPE portfolio_valuations_total counter\n",
        );
        for (status, value) in [
            ("complete", self.valuations_complete.load(Ordering::Relaxed)),
            ("partial", self.valuations_partial.load(Ordering::Relaxed)),
            (
                "unavailable",
                self.valuations_unavailable.load(Ordering::Relaxed),
            ),
        ] {
            output.push_str(&format!(
                "portfolio_valuations_total{{status=\"{status}\"}} {value}\n"
            ));
        }
        output.push_str(
            "# HELP portfolio_unpriced_assets The last observed excluded asset count.\n\
             # TYPE portfolio_unpriced_assets gauge\n",
        );
        output.push_str(&format!(
            "portfolio_unpriced_assets {}\n",
            self.unpriced_assets.load(Ordering::Relaxed)
        ));
        output.push_str(
            "# HELP portfolio_snapshots_total Snapshots written.\n\
             # TYPE portfolio_snapshots_total counter\n",
        );
        output.push_str(&format!(
            "portfolio_snapshots_total {}\n",
            self.snapshots_total.load(Ordering::Relaxed)
        ));
        output.push_str(
            "# HELP portfolio_snapshot_digest_mismatch_total Snapshots whose digest did not verify.\n\
             # TYPE portfolio_snapshot_digest_mismatch_total counter\n",
        );
        output.push_str(&format!(
            "portfolio_snapshot_digest_mismatch_total {}\n",
            self.digest_mismatch.load(Ordering::Relaxed)
        ));
        output.push_str(
            "# HELP portfolio_audit_failures_total Audit writes abandoned after retries.\n\
             # TYPE portfolio_audit_failures_total counter\n",
        );
        output.push_str(&format!(
            "portfolio_audit_failures_total {}\n",
            self.audit_failures.load(Ordering::Relaxed)
        ));
        output.push_str(
            "# HELP portfolio_guardrail_alerts_failed_total Guardrail alerts that will be retried.\n\
             # TYPE portfolio_guardrail_alerts_failed_total counter\n",
        );
        output.push_str(&format!(
            "portfolio_guardrail_alerts_failed_total {}\n",
            self.guardrail_alert_failures.load(Ordering::Relaxed)
        ));
        output.push_str(
            "# HELP portfolio_trade_overlays_expired_total Unmatched overlays expired after seven days.\n\
             # TYPE portfolio_trade_overlays_expired_total counter\n",
        );
        output.push_str(&format!(
            "portfolio_trade_overlays_expired_total {}\n",
            self.overlays_expired.load(Ordering::Relaxed)
        ));
        output.push_str(
            "# HELP portfolio_valuation_run_duration_seconds The last run's duration.\n\
             # TYPE portfolio_valuation_run_duration_seconds gauge\n",
        );
        output.push_str(&format!(
            "portfolio_valuation_run_duration_seconds {}\n",
            self.run_duration_seconds.load(Ordering::Relaxed)
        ));
        output.push_str(
            "# HELP portfolio_valuation_runs_total Runs observed, by status.\n\
             # TYPE portfolio_valuation_runs_total counter\n",
        );
        output.push_str(&format!(
            "portfolio_valuation_runs_total{{status=\"succeeded\"}} {}\n",
            self.runs_succeeded.load(Ordering::Relaxed)
        ));
        output.push_str(&format!(
            "portfolio_valuation_runs_total{{status=\"failed\"}} {}\n",
            self.runs_failed.load(Ordering::Relaxed)
        ));
        output.push_str(
            "# HELP portfolio_health 1 while the process is running.\n\
             # TYPE portfolio_health gauge\n",
        );
        output.push_str(&format!("portfolio_health{{service=\"{service}\"}} 1\n"));
        output
    }
}
