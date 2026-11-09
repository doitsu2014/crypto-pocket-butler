//! Minimal Prometheus text metrics: request counts, error counts, and liveness.

use std::sync::atomic::{AtomicU64, Ordering};

/// Request and error counters exposed at `/metrics`.
#[derive(Debug, Default)]
pub struct Metrics {
    total_2xx: AtomicU64,
    total_3xx: AtomicU64,
    total_4xx: AtomicU64,
    total_5xx: AtomicU64,
    errors_total: AtomicU64,
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

    /// Total requests observed.
    #[must_use]
    pub fn requests_total(&self) -> u64 {
        self.total_2xx.load(Ordering::Relaxed)
            + self.total_3xx.load(Ordering::Relaxed)
            + self.total_4xx.load(Ordering::Relaxed)
            + self.total_5xx.load(Ordering::Relaxed)
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
        output.push_str(
            "# HELP service_up 1 while the process is running.\n\
             # TYPE service_up gauge\n",
        );
        output.push_str(&format!("service_up{{service=\"{service}\"}} 1\n"));
        output
    }
}
