pub mod application_tests;
pub mod domain_tests;
pub mod k8s_tests;

use anyhow::Result;
use jules_remediator_rs::domain::models::*;

mockall::mock! {
    pub Remediator {}
    #[async_trait::async_trait]
    impl jules_remediator_rs::domain::services::Remediator for Remediator {
        fn classify_error(&self, error: &ClusterError) -> (bool, ErrorType);
        async fn propose_fix(&self, error: &ClusterError) -> Result<FixProposal>;
        async fn execute_fix(&self, proposal: &FixProposal) -> Result<RemediationOutcome>;
    }
}
