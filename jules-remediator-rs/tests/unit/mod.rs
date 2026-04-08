pub mod domain_tests;
pub mod application_tests;
pub mod infrastructure_tests;
pub mod k8s_tests;

use jules_remediator_rs::domain::models::*;
use anyhow::Result;

mockall::mock! {
    pub Remediator {}
    #[async_trait::async_trait]
    impl jules_remediator_rs::domain::services::Remediator for Remediator {
        fn classify_error(&self, error: &ClusterError) -> bool;
        async fn propose_fix(&self, error: &ClusterError) -> Result<FixProposal>;
        async fn execute_fix(&self, proposal: &FixProposal) -> Result<RemediationOutcome>;
    }
}
