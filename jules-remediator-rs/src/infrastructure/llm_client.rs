use crate::domain::models::*;
use anyhow::{Result, anyhow};
use reqwest::Client;
use serde::Deserialize;
use serde_json::json;

#[derive(Debug, Deserialize)]
struct OpenAIResponse {
    choices: Vec<OpenAIChoice>,
}

#[derive(Debug, Deserialize)]
struct OpenAIChoice {
    message: OpenAIMessage,
}

#[derive(Debug, Deserialize)]
struct OpenAIMessage {
    content: String,
}

pub struct LlmClient {
    client: Client,
    endpoint: String,
    model: String,
    api_key: String,
}

impl LlmClient {
    pub fn new(endpoint: &str, model: &str, api_key: &str) -> Self {
        Self {
            client: Client::new(),
            endpoint: endpoint.to_string(),
            model: model.to_string(),
            api_key: api_key.to_string(),
        }
    }

    pub async fn review_error(&self, error: &ClusterError) -> Result<AutonomousReview> {
        println!(
            "[LLM] Sending error {} to model {} for review",
            error.id, self.model
        );

        let prompt = format!(
            "Analyze the following Kubernetes error and provide a JSON response with fields: \
            'analysis', 'is_remediable' (bool), 'suggested_action' (string), and 'confidence' (float 0-1).\n\n\
            Resource: {}/{} ({})\nError: {}\nMessage: {}",
            error.resource.namespace,
            error.resource.name,
            error.resource.kind,
            error.error_code,
            error.message
        );

        let response = self.client.post(&self.endpoint)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&json!({
                "model": self.model,
                "messages": [
                    {"role": "system", "content": "You are a Kubernetes SRE Assistant. Respond only in valid JSON."},
                    {"role": "user", "content": prompt}
                ],
                "response_format": {"type": "json_object"}
            }))
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(anyhow!("LiteLLM gateway error: {}", response.status()));
        }

        let body: OpenAIResponse = response.json().await?;
        let content = body
            .choices
            .first()
            .ok_or_else(|| anyhow!("No choices in LLM response"))?
            .message
            .content
            .clone();

        let review_data: serde_json::Value = serde_json::from_str(&content)?;

        Ok(AutonomousReview {
            error_id: error.id,
            analysis: review_data["analysis"]
                .as_str()
                .unwrap_or("No analysis provided")
                .into(),
            is_remediable: review_data["is_remediable"].as_bool().unwrap_or(false),
            suggested_action: review_data["suggested_action"]
                .as_str()
                .map(|s| s.to_string()),
            confidence: review_data["confidence"].as_f64().unwrap_or(0.0) as f32,
        })
    }
}
