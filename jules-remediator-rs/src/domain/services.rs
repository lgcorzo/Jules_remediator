use crate::domain::models::*;
use anyhow::Result;

#[cfg_attr(test, mockall::automock)]
#[async_trait::async_trait] // Added async_trait for better trait compatibility
pub trait Remediator {
    /// Classifies an error event and determines if remediation is needed.
    fn classify_error(&self, error: &ClusterError) -> bool;

    /// Proposes a fix for a given error event.
    async fn propose_fix(&self, error: &ClusterError) -> Result<FixProposal>;

    /// Executes a remediation fix.
    async fn execute_fix(&self, proposal: &FixProposal) -> Result<RemediationOutcome>;
}
