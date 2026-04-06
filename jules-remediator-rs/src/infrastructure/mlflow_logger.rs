use anyhow::Result;
use reqwest::Client;
use serde_json::json;

pub struct MlflowLogger {
    client: Client,
    tracking_uri: String,
}

impl MlflowLogger {
    pub fn new(tracking_uri: String) -> Self {
        Self {
            client: Client::new(),
            tracking_uri,
        }
    }

    pub async fn log_remediation(&self, success: bool, latency: u64) -> Result<()> {
        let endpoint = format!("{}/api/2.0/mlflow/runs/log-metric", self.tracking_uri);
        // This is a simplified MLflow metric logging call
        let payload = json!({
            "run_id": "current_remediation_run", // This would be dynamic in a real setup
            "key": "remediation_success",
            "value": if success { 1.0 } else { 0.0 },
            "timestamp": chrono::Utc::now().timestamp_millis(),
        });

        let _ = self.client.post(&endpoint).json(&payload).send().await;

        let payload_latency = json!({
             "run_id": "current_remediation_run",
             "key": "latency_ms",
             "value": latency as f64,
             "timestamp": chrono::Utc::now().timestamp_millis(),
        });
        let _ = self.client.post(&endpoint).json(&payload_latency).send().await;

        Ok(())
    }
}
