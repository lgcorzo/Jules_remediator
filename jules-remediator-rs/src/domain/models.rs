use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StartupPhase {
    Initial,
    InProcess,
    Stabilized,
    Controlled,    // Added for tiered release
    Orchestrating, // Master orchestration loop active
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, PartialOrd)]
pub enum DependencyTier {
    Bootstrap = 0,
    Foundation = 1,
    CoreServices = 2,
    Applications = 3,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StartupEvent {
    pub timestamp: DateTime<Utc>,
    pub resource: ClusterResource,
    pub status: String, // "Started", "Ready", "Failed"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterStartupState {
    pub phase: StartupPhase,
    pub event_count: usize,
    pub start_time: DateTime<Utc>,
    pub current_tier: DependencyTier,
    pub boot_storm_detected: bool,
    pub batch_size: usize, // For Tier 3 release
    pub release_interval_secs: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ErrorType {
    Structural,
    Transient,
    Unknown,
}

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
    pub error_type: ErrorType,
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
    #[serde(rename = "session_id")]
    pub tracking_id: Uuid, // Track the iterative dialogue
    pub code_change: String,
    pub explanation: String,
    pub risk_score: RiskScore,
    pub confidence: f32, // 0.0 to 1.0
    pub remediation_command: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationMessage {
    #[serde(rename = "session_id")]
    pub tracking_id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub role: String, // "agent" or "jules"
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemediationStep {
    #[serde(rename = "session_id")]
    pub tracking_id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub command: String,
    pub success: bool,
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemediationOutcome {
    pub proposal_id: Uuid,
    #[serde(rename = "session_id")]
    pub tracking_id: Uuid,
    pub success: bool,
    pub latency_ms: u64,
    pub logs: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutonomousReview {
    pub error_id: Uuid,
    pub analysis: String,
    pub is_remediable: bool,
    pub suggested_action: Option<String>,
    pub confidence: f32,
}
