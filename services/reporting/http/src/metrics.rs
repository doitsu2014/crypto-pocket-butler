//! Minimal Prometheus text metrics: request classes and the error rate, brief
//! generation counts, email delivery counts, run duration, and a health gauge
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
    briefs_generated: AtomicU64,
    briefs_failed: AtomicU64,
    briefs_skipped: AtomicU64,
    email_submitted: AtomicU64,
    email_failed: AtomicU64,
    run_duration_millis: AtomicU64,
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

    /// Records one finished run and its outcome counts.
    pub fn record_run(&self, failed: bool, duration_seconds: f64) {
        if failed {
            self.runs_failed.fetch_add(1, Ordering::Relaxed);
        } else {
            self.runs_succeeded.fetch_add(1, Ordering::Relaxed);
        }
        let millis = (duration_seconds.max(0.0) * 1000.0).round() as u64;
        self.run_duration_millis.store(millis, Ordering::Relaxed);
    }

    /// Records the brief counts of one run.
    pub fn record_briefs(&self, generated: usize, failed: usize, skipped: usize) {
        self.briefs_generated
            .fetch_add(generated as u64, Ordering::Relaxed);
        self.briefs_failed
            .fetch_add(failed as u64, Ordering::Relaxed);
        self.briefs_skipped
            .fetch_add(skipped as u64, Ordering::Relaxed);
    }

    /// Records the email counts of one run.
    pub fn record_email(&self, submitted: usize, failed: usize) {
        self.email_submitted
            .fetch_add(submitted as u64, Ordering::Relaxed);
        self.email_failed
            .fetch_add(failed as u64, Ordering::Relaxed);
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
            "# HELP reporting_request_errors_total Total 5xx responses.\n\
             # TYPE reporting_request_errors_total counter\n",
        );
        output.push_str(&format!(
            "reporting_request_errors_total{{service=\"{service}\"}} {}\n",
            self.errors_total.load(Ordering::Relaxed)
        ));
        output.push_str(
            "# HELP reporting_briefs_total Briefs produced, failed, or skipped by the run.\n\
             # TYPE reporting_briefs_total counter\n",
        );
        for (outcome, value) in [
            ("generated", self.briefs_generated.load(Ordering::Relaxed)),
            ("failed", self.briefs_failed.load(Ordering::Relaxed)),
            ("skipped", self.briefs_skipped.load(Ordering::Relaxed)),
        ] {
            output.push_str(&format!(
                "reporting_briefs_total{{service=\"{service}\",outcome=\"{outcome}\"}} {value}\n"
            ));
        }
        output.push_str(
            "# HELP reporting_brief_email_total Email alerts submitted or failed.\n\
             # TYPE reporting_brief_email_total counter\n",
        );
        for (outcome, value) in [
            ("submitted", self.email_submitted.load(Ordering::Relaxed)),
            ("failed", self.email_failed.load(Ordering::Relaxed)),
        ] {
            output.push_str(&format!(
                "reporting_brief_email_total{{service=\"{service}\",outcome=\"{outcome}\"}} {value}\n"
            ));
        }
        output.push_str(
            "# HELP reporting_brief_runs_total Generation runs by outcome.\n\
             # TYPE reporting_brief_runs_total counter\n",
        );
        for (outcome, value) in [
            ("succeeded", self.runs_succeeded.load(Ordering::Relaxed)),
            ("failed", self.runs_failed.load(Ordering::Relaxed)),
        ] {
            output.push_str(&format!(
                "reporting_brief_runs_total{{service=\"{service}\",outcome=\"{outcome}\"}} {value}\n"
            ));
        }
        output.push_str(
            "# HELP reporting_brief_run_duration_millis The last run's duration.\n\
             # TYPE reporting_brief_run_duration_millis gauge\n",
        );
        output.push_str(&format!(
            "reporting_brief_run_duration_millis{{service=\"{service}\"}} {}\n",
            self.run_duration_millis.load(Ordering::Relaxed)
        ));
        output.push_str(
            "# HELP reporting_health Whether the service is serving.\n\
             # TYPE reporting_health gauge\n",
        );
        output.push_str(&format!("reporting_health{{service=\"{service}\"}} 1\n"));
        output
    }
}

#[cfg(test)]
mod tests {
    use super::Metrics;

    #[test]
    fn every_counter_reaches_the_exposition() {
        let metrics = Metrics::default();
        metrics.record(200);
        metrics.record(503);
        metrics.record_briefs(2, 1, 3);
        metrics.record_email(2, 0);
        metrics.record_run(false, 1.25);
        let text = metrics.render("reporting-api");
        assert!(text
            .contains("reporting_briefs_total{service=\"reporting-api\",outcome=\"generated\"} 2"));
        assert!(text.contains(
            "reporting_brief_email_total{service=\"reporting-api\",outcome=\"submitted\"} 2"
        ));
        assert!(text.contains(
            "reporting_brief_runs_total{service=\"reporting-api\",outcome=\"succeeded\"} 1"
        ));
        assert!(
            text.contains("reporting_brief_run_duration_millis{service=\"reporting-api\"} 1250")
        );
        assert!(text.contains("reporting_request_errors_total{service=\"reporting-api\"} 1"));
    }

    #[test]
    fn the_error_total_counts_only_5xx() {
        let metrics = Metrics::default();
        metrics.record(404);
        assert_eq!(metrics.errors_total(), 0);
        metrics.record(500);
        assert_eq!(metrics.errors_total(), 1);
    }

    #[test]
    fn a_failed_run_is_counted_separately() {
        let metrics = Metrics::default();
        metrics.record_run(true, 0.5);
        let text = metrics.render("reporting-worker");
        assert!(text.contains("outcome=\"failed\"} 1"));
        assert!(text.contains("outcome=\"succeeded\"} 0"));
    }
}
