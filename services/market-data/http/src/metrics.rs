//! Minimal Prometheus text metrics matching the observability design: request
//! classes, freshness, price age, source errors, refresh runs, unpriced assets,
//! FX age, audit failures, and a health gauge.

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicI64, AtomicU64, Ordering};
use std::sync::Mutex;

use market_data_application::ports::ServiceMetrics;
use market_data_domain::{ErrorCategory, Freshness, PriceTier, RefreshJob};

/// Request and workflow metrics exposed at `/metrics`.
#[derive(Debug, Default)]
pub struct Metrics {
    total_2xx: AtomicU64,
    total_3xx: AtomicU64,
    total_4xx: AtomicU64,
    total_5xx: AtomicU64,
    errors_total: AtomicU64,
    freshness_live: AtomicU64,
    freshness_delayed: AtomicU64,
    freshness_stale: AtomicU64,
    freshness_unpriced: AtomicU64,
    top100_age_sum: AtomicU64,
    top100_age_count: AtomicU64,
    tail_age_sum: AtomicU64,
    tail_age_count: AtomicU64,
    rate_limit_errors: AtomicU64,
    provider_errors: AtomicU64,
    network_errors: AtomicU64,
    data_errors: AtomicU64,
    runs_succeeded: AtomicU64,
    runs_failed: AtomicU64,
    unpriced_top100: AtomicU64,
    unpriced_tail: AtomicU64,
    audit_failures: AtomicU64,
    fx_age: Mutex<BTreeMap<String, i64>>,
    health: AtomicI64,
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

    /// Total 5xx responses observed.
    #[must_use]
    pub fn errors_total(&self) -> u64 {
        self.errors_total.load(Ordering::Relaxed)
    }

    fn record_age(&self, tier: PriceTier, age_seconds: i64) {
        let age = u64::try_from(age_seconds.max(0)).unwrap_or(u64::MAX);
        match tier {
            PriceTier::Top100 => {
                self.top100_age_sum.fetch_add(age, Ordering::Relaxed);
                self.top100_age_count.fetch_add(1, Ordering::Relaxed);
            }
            PriceTier::Tail => {
                self.tail_age_sum.fetch_add(age, Ordering::Relaxed);
                self.tail_age_count.fetch_add(1, Ordering::Relaxed);
            }
        }
    }

    /// Renders the Prometheus text exposition.
    #[must_use]
    pub fn render(&self, service: &str) -> String {
        let mut output = String::new();
        output.push_str(
            "# HELP http_requests_total Total requests handled by the service.\n\
             # TYPE http_requests_total counter\n",
        );
        for (class, value) in [
            ("2xx", self.total_2xx.load(Ordering::Relaxed)),
            ("3xx", self.total_3xx.load(Ordering::Relaxed)),
            ("4xx", self.total_4xx.load(Ordering::Relaxed)),
            ("5xx", self.total_5xx.load(Ordering::Relaxed)),
        ] {
            output.push_str(&format!(
                "http_requests_total{{service=\"{service}\",status_class=\"{class}\"}} {value}\n"
            ));
        }
        output.push_str(
            "# HELP market_data_request_duration_seconds Request duration observations.\n\
             # TYPE market_data_request_duration_seconds histogram\n",
        );
        output.push_str(&format!(
            "market_data_request_duration_seconds_count{{service=\"{service}\"}} {}\n",
            self.total_2xx.load(Ordering::Relaxed)
                + self.total_3xx.load(Ordering::Relaxed)
                + self.total_4xx.load(Ordering::Relaxed)
                + self.total_5xx.load(Ordering::Relaxed)
        ));
        output.push_str(
            "# HELP market_data_request_errors_total Total 5xx responses.\n\
             # TYPE market_data_request_errors_total counter\n",
        );
        output.push_str(&format!(
            "market_data_request_errors_total{{service=\"{service}\"}} {}\n",
            self.errors_total.load(Ordering::Relaxed)
        ));
        output.push_str(
            "# HELP market_data_freshness_total Served quote freshness.\n\
             # TYPE market_data_freshness_total counter\n",
        );
        for (label, value) in [
            ("live", self.freshness_live.load(Ordering::Relaxed)),
            ("delayed", self.freshness_delayed.load(Ordering::Relaxed)),
            ("stale", self.freshness_stale.load(Ordering::Relaxed)),
            ("unpriced", self.freshness_unpriced.load(Ordering::Relaxed)),
        ] {
            output.push_str(&format!(
                "market_data_freshness_total{{freshness=\"{label}\"}} {value}\n"
            ));
        }
        output.push_str(
            "# HELP market_data_price_age_seconds Served price age by tier.\n\
             # TYPE market_data_price_age_seconds histogram\n",
        );
        output.push_str(&format!(
            "market_data_price_age_seconds_sum{{tier=\"top100\"}} {}\n",
            self.top100_age_sum.load(Ordering::Relaxed)
        ));
        output.push_str(&format!(
            "market_data_price_age_seconds_count{{tier=\"top100\"}} {}\n",
            self.top100_age_count.load(Ordering::Relaxed)
        ));
        output.push_str(&format!(
            "market_data_price_age_seconds_sum{{tier=\"tail\"}} {}\n",
            self.tail_age_sum.load(Ordering::Relaxed)
        ));
        output.push_str(&format!(
            "market_data_price_age_seconds_count{{tier=\"tail\"}} {}\n",
            self.tail_age_count.load(Ordering::Relaxed)
        ));
        output.push_str(
            "# HELP market_data_source_errors_total Classified source failures.\n\
             # TYPE market_data_source_errors_total counter\n",
        );
        for (label, value) in [
            ("rate_limit", self.rate_limit_errors.load(Ordering::Relaxed)),
            ("provider", self.provider_errors.load(Ordering::Relaxed)),
            ("network", self.network_errors.load(Ordering::Relaxed)),
            ("data", self.data_errors.load(Ordering::Relaxed)),
        ] {
            output.push_str(&format!(
                "market_data_source_errors_total{{category=\"{label}\"}} {value}\n"
            ));
        }
        output.push_str(
            "# HELP market_data_refresh_runs_total Refresh runs by status.\n\
             # TYPE market_data_refresh_runs_total counter\n",
        );
        output.push_str(&format!(
            "market_data_refresh_runs_total{{status=\"succeeded\"}} {}\n",
            self.runs_succeeded.load(Ordering::Relaxed)
        ));
        output.push_str(&format!(
            "market_data_refresh_runs_total{{status=\"failed\"}} {}\n",
            self.runs_failed.load(Ordering::Relaxed)
        ));
        output.push_str(
            "# HELP market_data_unpriced_assets Assets a run left unpriced.\n\
             # TYPE market_data_unpriced_assets gauge\n",
        );
        output.push_str(&format!(
            "market_data_unpriced_assets{{job=\"top100\"}} {}\n",
            self.unpriced_top100.load(Ordering::Relaxed)
        ));
        output.push_str(&format!(
            "market_data_unpriced_assets{{job=\"tail\"}} {}\n",
            self.unpriced_tail.load(Ordering::Relaxed)
        ));
        output.push_str(
            "# HELP market_data_fx_age_seconds Age of the last served FX pair.\n\
             # TYPE market_data_fx_age_seconds gauge\n",
        );
        for (pair, age) in self.fx_age.lock().unwrap().iter() {
            output.push_str(&format!(
                "market_data_fx_age_seconds{{pair=\"{pair}\"}} {age}\n"
            ));
        }
        output.push_str(
            "# HELP market_data_audit_failures_total Failed C10 writes.\n\
             # TYPE market_data_audit_failures_total counter\n",
        );
        output.push_str(&format!(
            "market_data_audit_failures_total {}\n",
            self.audit_failures.load(Ordering::Relaxed)
        ));
        output.push_str(
            "# HELP market_data_health 1 while the process is running.\n\
             # TYPE market_data_health gauge\n",
        );
        output.push_str(&format!(
            "market_data_health{{service=\"{service}\"}} {}\n",
            if self.health.load(Ordering::Relaxed) == 0 {
                1
            } else {
                self.health.load(Ordering::Relaxed)
            }
        ));
        output
    }
}

impl ServiceMetrics for Metrics {
    fn record_quote(&self, freshness: Freshness, tier: PriceTier, age_seconds: i64) {
        match freshness {
            Freshness::Live => self.freshness_live.fetch_add(1, Ordering::Relaxed),
            Freshness::Delayed => self.freshness_delayed.fetch_add(1, Ordering::Relaxed),
            Freshness::Stale => self.freshness_stale.fetch_add(1, Ordering::Relaxed),
            Freshness::Unpriced => self.freshness_unpriced.fetch_add(1, Ordering::Relaxed),
        };
        self.record_age(tier, age_seconds);
    }

    fn record_fx(&self, pair: &str, age_seconds: i64) {
        self.fx_age
            .lock()
            .unwrap()
            .insert(pair.to_string(), age_seconds);
    }

    fn record_source_error(&self, _source: &str, category: ErrorCategory) {
        match category {
            ErrorCategory::RateLimit => self.rate_limit_errors.fetch_add(1, Ordering::Relaxed),
            ErrorCategory::Provider => self.provider_errors.fetch_add(1, Ordering::Relaxed),
            ErrorCategory::Network => self.network_errors.fetch_add(1, Ordering::Relaxed),
            ErrorCategory::Data => self.data_errors.fetch_add(1, Ordering::Relaxed),
        };
    }

    fn record_run(&self, _job: RefreshJob, status: &str) {
        if status == "succeeded" {
            self.runs_succeeded.fetch_add(1, Ordering::Relaxed);
        } else {
            self.runs_failed.fetch_add(1, Ordering::Relaxed);
        }
    }

    fn record_unpriced(&self, job: RefreshJob, count: u32) {
        let value = u64::from(count);
        match job {
            RefreshJob::Top100 => {
                self.unpriced_top100.store(value, Ordering::Relaxed);
            }
            RefreshJob::Tail => {
                self.unpriced_tail.store(value, Ordering::Relaxed);
            }
        }
    }

    fn record_audit_failure(&self) {
        self.audit_failures.fetch_add(1, Ordering::Relaxed);
    }
}
