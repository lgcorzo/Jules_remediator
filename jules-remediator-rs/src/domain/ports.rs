use anyhow::Result;
use async_trait::async_trait;

#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait Tracker: Send + Sync {
    /// Logs a specific metric to the tracking system.
    async fn log_metric(&self, key: &str, value: f64) -> Result<()>;

    /// Logs the outcome of a remediation event.
    async fn log_remediation(&self, success: bool, latency_ms: u64) -> Result<()>;
}
