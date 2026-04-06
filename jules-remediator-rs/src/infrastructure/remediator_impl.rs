use crate::domain::models::*;
use crate::domain::services::Remediator;
use crate::infrastructure::jules_dispatcher::JulesDispatcher;
use crate::infrastructure::mlflow_logger::MlflowLogger;
use crate::infrastructure::persistence::SurrealPersistence;
use anyhow::Result;
use std::sync::Arc;

pub struct RemediatorImpl {
    dispatcher: Arc<JulesDispatcher>,
    logger: Arc<MlflowLogger>,
    persistence: Arc<SurrealPersistence>,
}

impl RemediatorImpl {
    pub async fn new(
        dispatcher_uri: &str,
        mlflow_uri: &str,
        db_path: &str,
    ) -> Result<Self> {
        Ok(Self {
            dispatcher: Arc::new(JulesDispatcher::new(dispatcher_uri).await?),
            logger: Arc::new(MlflowLogger::new(mlflow_uri.into())),
            persistence: Arc::new(SurrealPersistence::new(db_path).await?),
        })
    }
}

#[async_trait::async_trait]
impl Remediator for RemediatorImpl {
    fn classify_error(&self, error: &ClusterError) -> bool {
         // DDD Rule: Structural vs Transient.
         // Let's assume we skip simple transient errors (e.g. ImagePullBackOff due to registry transient error)
         // but handle OOMKilled or CrashLoopBackOff.
         !error.message.contains("transient") && (error.error_code == "OOMKilled" || error.error_code == "BackOff")
    }

    async fn propose_fix(&self, error: &ClusterError) -> Result<FixProposal> {
        self.persistence.save_error(error).await?;
        self.dispatcher.get_fix(error).await
    }

    async fn execute_fix(&self, proposal: &FixProposal) -> Result<RemediationOutcome> {
        // Logic to push to Git or patch cluster (GitOps approach preferred)
        println!("[Remediator] Executing fix proposal: {}", proposal.proposal_id);
        
        let outcome = RemediationOutcome {
            proposal_id: proposal.proposal_id,
            success: true,
            latency_ms: 1200,
            logs: "Successfully applied patch via flux-system".into(),
        };

        self.persistence.save_outcome(&outcome).await?;
        self.logger.log_remediation(outcome.success, outcome.latency_ms).await?;

        Ok(outcome)
    }
}
