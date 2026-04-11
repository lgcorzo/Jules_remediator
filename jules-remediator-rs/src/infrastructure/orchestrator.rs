use crate::domain::models::{ClusterError, FixProposal};
use anyhow::Result;
use async_trait::async_trait;

#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait Orchestrator: Send + Sync {
    async fn orchestrate_remediation(&self, error: &ClusterError) -> Result<FixProposal>;
}

pub struct OrchestratorImpl {
    dispatcher_uri: String,
}

impl OrchestratorImpl {
    pub fn new(dispatcher_uri: &str) -> Self {
        Self {
            dispatcher_uri: dispatcher_uri.to_string(),
        }
    }
}

#[async_trait]
impl Orchestrator for OrchestratorImpl {
    async fn orchestrate_remediation(&self, error: &ClusterError) -> Result<FixProposal> {
        // Implementation of the actual call to Jules via ZeroClaw/MCP
        // For Phase 2, we mocked this, but now we'll provide the real structure
        // In a real scenario, this would use reqwest to call dispatcher_uri

        println!(
            "[Orchestrator] Calling Jules at {} for error: {}",
            self.dispatcher_uri, error.id
        );

        // Mocking the call for now until we have the full MCP client integrated
        Ok(FixProposal {
            error_id: error.id,
            proposal_id: uuid::Uuid::new_v4(),
            code_change: "spec.containers[0].resources.limits.memory = \"512Mi\"".into(),
            explanation: "Increasing memory limits for OOMKilled pod.".into(),
            risk_score: crate::domain::models::RiskScore::Low,
            confidence: 0.95,
            remediation_command: Some("kubectl patch ...".into()),
        })
    }
}
