use crate::domain::models::*;
use crate::domain::services::Remediator;
use crate::infrastructure::mlflow_logger::MlflowLogger;
use crate::infrastructure::persistence::SurrealPersistence;
use crate::infrastructure::zeroclaw::ZeroClaw;
use anyhow::Result;
use std::sync::Arc;

pub struct RemediatorImpl {
    zeroclaw: Arc<ZeroClaw>,
    logger: Arc<MlflowLogger>,
    persistence: Arc<SurrealPersistence>,
}

impl RemediatorImpl {
    pub async fn new(_dispatcher_uri: &str, mlflow_uri: &str, db_path: &str) -> Result<Self> {
        Ok(Self {
            zeroclaw: Arc::new(ZeroClaw::new()?),
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
        let mut logs = String::new();

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
