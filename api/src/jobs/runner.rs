use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::time::Instant;
use tracing;

/// Standard result structure for all jobs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobResult {
    /// Whether the job completed successfully
    pub success: bool,
    /// Name of the job
    pub job_name: String,
    /// Duration of job execution in milliseconds
    pub duration_ms: u64,
    /// Timestamp when job started
    pub started_at: chrono::DateTime<Utc>,
    /// Timestamp when job completed
    pub completed_at: chrono::DateTime<Utc>,
    /// Job-specific metrics
    pub metrics: JobMetrics,
    /// Error message if job failed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Job-specific metrics that vary by job type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobMetrics {
    /// Number of items processed
    pub items_processed: usize,
    /// Number of items created
    pub items_created: usize,
    /// Number of items updated
    pub items_updated: usize,
    /// Number of items skipped
    pub items_skipped: usize,
    /// Additional custom metrics
    #[serde(flatten)]
    pub custom: serde_json::Value,
}

impl Default for JobMetrics {
    fn default() -> Self {
        Self {
            items_processed: 0,
            items_created: 0,
            items_updated: 0,
            items_skipped: 0,
            custom: serde_json::json!({}),
        }
    }
}

/// Job runner that wraps job execution with common functionality
pub struct JobRunner {
    job_name: String,
}

impl JobRunner {
    /// Create a new job runner
    pub fn new(job_name: impl Into<String>) -> Self {
        Self {
            job_name: job_name.into(),
        }
    }

    /// Execute a job with timing, logging, and error handling
    ///
    /// # Arguments
    /// * `job_fn` - Async function that performs the job work and returns metrics
    ///
    /// # Returns
    /// JobResult with execution details and metrics
    pub async fn execute<F, Fut>(
        &self,
        job_fn: F,
    ) -> JobResult
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = Result<JobMetrics, String>>,
    {
        let started_at = Utc::now();
        let start_instant = Instant::now();

        tracing::info!("Starting job: {}", self.job_name);

        let (success, metrics, error) = match job_fn().await {
            Ok(metrics) => {
                tracing::info!(
                    "Job '{}' completed successfully: {} processed, {} created, {} updated, {} skipped",
                    self.job_name,
                    metrics.items_processed,
                    metrics.items_created,
                    metrics.items_updated,
                    metrics.items_skipped
                );
                (true, metrics, None)
            }
            Err(e) => {
                tracing::error!("Job '{}' failed: {}", self.job_name, e);
                (false, JobMetrics::default(), Some(e))
            }
        };

        let completed_at = Utc::now();
        let duration_ms = start_instant.elapsed().as_millis() as u64;

        tracing::info!(
            "Job '{}' execution time: {} ms",
            self.job_name,
            duration_ms
        );

        JobResult {
            success,
            job_name: self.job_name.clone(),
            duration_ms,
            started_at,
            completed_at,
            metrics,
            error,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_job_runner_success() {
        let runner = JobRunner::new("test_job");
        
        let result = runner.execute(|| async {
            // Add a small delay to ensure duration_ms > 0
            tokio::time::sleep(tokio::time::Duration::from_millis(1)).await;
            Ok(JobMetrics {
                items_processed: 100,
                items_created: 50,
                items_updated: 30,
                items_skipped: 20,
                custom: serde_json::json!({ "test": "value" }),
            })
        }).await;

        assert!(result.success);
        assert_eq!(result.job_name, "test_job");
        assert_eq!(result.metrics.items_processed, 100);
        assert_eq!(result.metrics.items_created, 50);
        assert_eq!(result.metrics.items_updated, 30);
        assert_eq!(result.metrics.items_skipped, 20);
        assert!(result.error.is_none());
    }

    #[tokio::test]
    async fn test_job_runner_failure() {
        let runner = JobRunner::new("failing_job");
        
        let result = runner.execute(|| async {
            tokio::time::sleep(tokio::time::Duration::from_millis(1)).await;
            Err::<JobMetrics, String>("Something went wrong".to_string())
        }).await;

        assert!(!result.success);
        assert_eq!(result.job_name, "failing_job");
        assert_eq!(result.metrics.items_processed, 0);
        assert_eq!(result.error, Some("Something went wrong".to_string()));
    }
}
