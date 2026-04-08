use crate::domain::models::{ClusterError, FixProposal};
use anyhow::Result;
use async_trait::async_trait;

#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait Orchestrator: Send + Sync {
    async fn orchestrate_remediation(&self, error: &ClusterError) -> Result<FixProposal>;
}
