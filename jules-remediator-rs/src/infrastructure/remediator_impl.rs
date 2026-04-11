use crate::domain::models::*;
use crate::domain::ports::Tracker;
use crate::domain::services::Remediator;
use crate::infrastructure::mlflow_logger::MlflowLogger;
use crate::infrastructure::orchestrator::Orchestrator;
use crate::infrastructure::persistence::SurrealPersistence;
use crate::infrastructure::zeroclaw::ZeroClaw;
use anyhow::{Context, Result};
use std::process::Command;
use std::sync::Arc;

pub struct RemediatorImpl {
    zeroclaw: Arc<dyn Orchestrator>,
    tracker: Arc<dyn Tracker>,
    persistence: Arc<SurrealPersistence>,
}

impl RemediatorImpl {
    pub async fn new(_dispatcher_uri: &str, mlflow_uri: &str, db_path: &str) -> Result<Self> {
        let zeroclaw: Arc<dyn Orchestrator> = Arc::new(ZeroClaw::new()?);
        let tracker: Arc<dyn Tracker> = Arc::new(MlflowLogger::new(mlflow_uri.into()));
        Self::new_with_dependencies(db_path, zeroclaw, tracker).await
    }

    pub async fn new_with_dependencies(
        db_path: &str,
        orchestrator: Arc<dyn Orchestrator>,
        tracker: Arc<dyn Tracker>,
    ) -> Result<Self> {
        Ok(Self {
            zeroclaw: orchestrator,
            tracker,
            persistence: Arc::new(SurrealPersistence::new(db_path).await?),
        })
    }
}

#[async_trait::async_trait]
impl Remediator for RemediatorImpl {
    fn classify_error(&self, error: &ClusterError) -> (bool, ErrorType) {
        // High-level categorization
        let error_type = if error.error_code == "OOMKilled" {
            ErrorType::Permanent
        } else if error.error_code == "BackOff" || error.error_code == "CrashLoopBackOff" {
            // CrashLoopBackOff can be transient (e.g. database not ready)
            // or permanent (e.g. bad binary). For now we assume permanent for remediation.
            ErrorType::Permanent
        } else if error.message.contains("transient") || error.message.contains("timeout") {
            ErrorType::Transient
        } else {
            ErrorType::Unknown
        };

        let should_remediate = match error_type {
            ErrorType::Permanent => true,
            ErrorType::Transient => false, // Don't remediate transient glitches automatically
            ErrorType::Unknown => error.severity == Severity::Critical,
        };

        (should_remediate, error_type)
    }

    async fn propose_fix(&self, error: &ClusterError) -> Result<FixProposal> {
        self.persistence.save_error(error).await?;
        self.zeroclaw.orchestrate_remediation(error).await
    }

    async fn execute_fix(&self, proposal: &FixProposal) -> Result<RemediationOutcome> {
        println!(
            "[Remediator] Executing fix proposal: {}",
            proposal.proposal_id
        );

        let mut success = false;
        let logs;

        if let Some(cmd) = &proposal.remediation_command {
            println!("[Remediator] Running command: {}", cmd);
            let output = Command::new("sh")
                .arg("-c")
                .arg(cmd)
                .output()
                .context("failed to execute remediation command")?;

            success = output.status.success();
            logs = format!(
                "STDOUT: {}\nSTDERR: {}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
        } else {
            logs = "No remediation command provided in proposal.".into();
        }

        let outcome = RemediationOutcome {
            proposal_id: proposal.proposal_id,
            success,
            latency_ms: 1200, // Placeholder
            logs,
        };

        self.persistence.save_outcome(&outcome).await?;
        self.tracker
            .log_remediation(outcome.success, outcome.latency_ms)
            .await?;

        Ok(outcome)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::ports::MockTracker;
    use crate::infrastructure::orchestrator::MockOrchestrator;
    use uuid::Uuid;

    fn create_test_error() -> ClusterError {
        ClusterError {
            id: Uuid::new_v4(),
            timestamp: chrono::Utc::now(),
            severity: Severity::High,
            error_type: ErrorType::Unknown,
            resource: ClusterResource {
                kind: "Deployment".into(),
                name: "test-app".into(),
                namespace: "default".into(),
                api_version: "apps/v1".into(),
            },
            message: "OOMKilled".into(),
            error_code: "OOMKilled".into(),
            raw_event: serde_json::json!({}),
        }
    }

    #[tokio::test]
    async fn test_remediator_impl_integration() {
        // Mock for Tracker
        let mut mock_tracker = MockTracker::new();
        mock_tracker
            .expect_log_remediation()
            .returning(|_, _| Ok(()));

        // Mock for Orchestrator
        let mut mock_orch = MockOrchestrator::new();
        let error = create_test_error();
        let error_id = error.id;

        mock_orch
            .expect_orchestrate_remediation()
            .returning(move |e| {
                Ok(FixProposal {
                    error_id: e.id,
                    proposal_id: Uuid::new_v4(),
                    code_change: "spec.replicas = 2".into(),
                    explanation: "Scaling fix".into(),
                    risk_score: RiskScore::Low,
                    confidence: 0.9,
                    remediation_command: Some("true".into()),
                })
            });

        let remediator = RemediatorImpl::new_with_dependencies(
            "mem://",
            Arc::new(mock_orch),
            Arc::new(mock_tracker),
        )
        .await
        .unwrap();

        let proposal: FixProposal = remediator.propose_fix(&error).await.unwrap();
        assert_eq!(proposal.error_id, error_id);

        let outcome: RemediationOutcome = remediator.execute_fix(&proposal).await.unwrap();
        assert!(outcome.success);
    }

    #[tokio::test]
    async fn test_classify_error_logic() {
        let mock_tracker = MockTracker::new();
        let mock_orch = MockOrchestrator::new();

        let remediator = RemediatorImpl::new_with_dependencies(
            "mem://",
            Arc::new(mock_orch),
            Arc::new(mock_tracker),
        )
        .await
        .unwrap();

        let mut error = create_test_error();

        // Permanent Error (OOMKilled)
        error.error_code = "OOMKilled".into();
        let (should, etype) = remediator.classify_error(&error);
        assert!(should);
        assert_eq!(etype, ErrorType::Permanent);

        // Permanent Error (CrashLoopBackOff)
        error.error_code = "CrashLoopBackOff".into();
        let (should, etype) = remediator.classify_error(&error);
        assert!(should);
        assert_eq!(etype, ErrorType::Permanent);

        // Transient Error
        error.error_code = "ErrImagePull".into();
        error.message = "transient network timeout".into();
        let (should, etype) = remediator.classify_error(&error);
        assert!(!should);
        assert_eq!(etype, ErrorType::Transient);
    }
}
