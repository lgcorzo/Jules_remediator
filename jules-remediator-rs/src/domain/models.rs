use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterResource {
    pub kind: String,
    pub name: String,
    pub namespace: String,
    pub api_version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterError {
    pub id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub severity: Severity,
    pub resource: ClusterResource,
    pub message: String,
    pub error_code: String,
    #[serde(default)]
    pub raw_event: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum RiskScore {
    Low,
    Medium,
    High,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FixProposal {
    pub error_id: Uuid,
    pub proposal_id: Uuid,
    pub code_change: String,
    pub explanation: String,
    pub risk_score: RiskScore,
    pub confidence: f32, // 0.0 to 1.0
    pub remediation_command: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemediationOutcome {
    pub proposal_id: Uuid,
    pub success: bool,
    pub latency_ms: u64,
    pub logs: String,
}
