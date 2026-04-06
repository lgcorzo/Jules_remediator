use anyhow::{anyhow, Result};
use reqwest::Client;
use serde_json::json;

pub struct MlflowLogger {
    client: Client,
    tracking_uri: String,
    current_run_id: tokio::sync::Mutex<Option<String>>,
}

impl MlflowLogger {
    pub fn new(tracking_uri: String) -> Self {
        Self {
            client: Client::new(),
            tracking_uri,
            current_run_id: tokio::sync::Mutex::new(None),
        }
    }

    async fn get_or_create_run(&self) -> Result<String> {
        let mut run_id_opt = self.current_run_id.lock().await;
        if let Some(ref id) = *run_id_opt {
            return Ok(id.clone());
        }

        // Create a new run
        let endpoint = format!("{}/api/2.0/mlflow/runs/create", self.tracking_uri);
        let payload = json!({
            "experiment_id": "0", // Default experiment
            "start_time": chrono::Utc::now().timestamp_millis(),
            "tags": [
                {"key": "project", "value": "Aethelgard"},
                {"key": "type", "value": "Remediation"}
            ]
        });

        let response = self.client.post(&endpoint).json(&payload).send().await?;
        if !response.status().is_success() {
            return Err(anyhow!("Failed to create MLflow run: {}", response.status()));
        }

        let body: serde_json::Value = response.json().await?;
        let id = body["run"]["info"]["run_id"]
            .as_str()
            .ok_or_else(|| anyhow!("No run_id in MLflow response"))?
            .to_string();

        *run_id_opt = Some(id.clone());
        Ok(id)
    }

    pub async fn log_remediation(&self, success: bool, latency: u64) -> Result<()> {
        let run_id = self.get_or_create_run().await?;
        let endpoint = format!("{}/api/2.0/mlflow/runs/log-metric", self.tracking_uri);
        
        // Log Success Metric
        let _ = self.client.post(&endpoint).json(&json!({
            "run_id": run_id,
            "key": "remediation_success",
            "value": if success { 1.0 } else { 0.0 },
            "timestamp": chrono::Utc::now().timestamp_millis(),
        })).send().await;

        // Log Latency Metric
        let _ = self.client.post(&endpoint).json(&json!({
             "run_id": run_id,
             "key": "latency_ms",
             "value": latency as f64,
             "timestamp": chrono::Utc::now().timestamp_millis(),
        })).send().await;

        Ok(())
    }
}
