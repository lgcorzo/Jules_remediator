use crate::domain::models::*;
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
    logger: Arc<MlflowLogger>,
    persistence: Arc<SurrealPersistence>,
}

impl RemediatorImpl {
    pub async fn new(_dispatcher_uri: &str, mlflow_uri: &str, db_path: &str) -> Result<Self> {
        let zeroclaw = Arc::new(ZeroClaw::new()?);
        Self::new_with_orchestrator(mlflow_uri, db_path, zeroclaw).await
    }

    pub async fn new_with_orchestrator(
        mlflow_uri: &str,
        db_path: &str,
        orchestrator: Arc<dyn Orchestrator>,
    ) -> Result<Self> {
        Ok(Self {
            zeroclaw: orchestrator,
            logger: Arc::new(MlflowLogger::new(mlflow_uri.into())),
            persistence: Arc::new(SurrealPersistence::new(db_path).await?),
        })
    }
}

#[async_trait::async_trait]
impl Remediator for RemediatorImpl {
    fn classify_error(&self, error: &ClusterError) -> bool {
        // Phase 2: Prioritize OOMKilled for self-healing loops.
        // We also handle BackOff as potentially remediable.
        let is_remediable = error.error_code == "OOMKilled" || error.error_code == "BackOff";

        let is_structural = !error.message.contains("transient");

        is_remediable && is_structural
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
            latency_ms: 1200, // Placeholder for actual timing
            logs,
        };

        self.persistence.save_outcome(&outcome).await?;
        self.logger
            .log_remediation(outcome.success, outcome.latency_ms)
            .await?;

        Ok(outcome)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::orchestrator::MockOrchestrator;
    use mockito::Server;
    use uuid::Uuid;

    fn create_test_error() -> ClusterError {
        ClusterError {
            id: Uuid::new_v4(),
            timestamp: chrono::Utc::now(),
            severity: Severity::High,
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
        let mut server = Server::new_async().await;

        // Mock for MLflow run creation
        server
            .mock("POST", "/api/2.0/mlflow/runs/create")
            .with_status(200)
            .with_body(r#"{"run": {"info": {"run_id": "test-run-id"}}}"#)
            .create_async()
            .await;

        // Mock for MLflow logging during execution
        server
            .mock("POST", "/api/2.0/mlflow/runs/log-metric")
            .with_status(200)
            .create_async()
            .await;

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

        let remediator =
            RemediatorImpl::new_with_orchestrator(&server.url(), "mem://", Arc::new(mock_orch))
                .await
                .unwrap();

        let proposal: FixProposal = remediator.propose_fix(&error).await.unwrap();
        assert_eq!(proposal.error_id, error_id);

        let outcome: RemediationOutcome = remediator.execute_fix(&proposal).await.unwrap();
        assert!(outcome.success);
    }

    #[tokio::test]
    async fn test_classify_error_logic() {
        let server = Server::new_async().await;
        let remediator = RemediatorImpl::new(&server.url(), &server.url(), "mem://")
            .await
            .unwrap();

        let mut error = create_test_error();
        error.error_code = "OOMKilled".into();
        error.message = "Memory limit exceeded".into();

        assert!(remediator.classify_error(&error));

        error.message = "transient network error".into();
        assert!(!remediator.classify_error(&error));
    }
}
