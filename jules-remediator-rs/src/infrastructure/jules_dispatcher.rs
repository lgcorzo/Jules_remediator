use crate::domain::models::*;
use anyhow::{anyhow, Result};
use reqwest::Client;
use serde_json::json;

pub struct JulesDispatcher {
    client: Client,
    endpoint: String,
}

impl JulesDispatcher {
    pub async fn new(endpoint: &str) -> Result<Self> {
        println!("[Dispatcher] Linking to Jules endpoint: {}", endpoint);
        Ok(Self {
            client: Client::new(),
            endpoint: endpoint.into(),
        })
    }

    pub async fn get_fix(&self, error: &ClusterError) -> Result<FixProposal> {
        println!(
            "[Dispatcher] Dispatching mission to Jules Cloud endpoint: {} for error: {}",
            self.endpoint, error.id
        );

        let response = self
            .client
            .post(&self.endpoint)
            .json(&json!({
                "jsonrpc": "2.0",
                "method": "call_tool",
                "params": {
                    "name": "remediate_error",
                    "arguments": {
                        "error_id": error.id,
                        "message": error.message,
                        "resource": error.resource.name,
                        "namespace": error.resource.namespace
                    }
                },
                "id": 1
            }))
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(anyhow!("Jules MCP returned error: {}", response.status()));
        }

        let body: serde_json::Value = response.json().await?;
        if let Some(error) = body.get("error") {
             return Err(anyhow!("Jules MCP Tool Error: {}", error["message"]));
        }

        let result = &body["result"];
        
        // Map MCP result to FixProposal
        Ok(FixProposal {
            error_id: error.id,
            proposal_id: uuid::Uuid::new_v4(),
            code_change: result["code_change"].as_str().unwrap_or("").into(),
            explanation: result["explanation"].as_str().unwrap_or("No explanation provided").into(),
            risk_score: match result["risk_score"].as_str().unwrap_or("Low") {
                "High" => RiskScore::High,
                "Medium" => RiskScore::Medium,
                _ => RiskScore::Low,
            },
            confidence: result["confidence"].as_f64().unwrap_or(0.0) as f32,
            remediation_command: result["remediation_command"].as_str().map(|s| s.to_string()),
        })
    }
}
